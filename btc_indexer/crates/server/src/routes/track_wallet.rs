use std::sync::Arc;

use axum::extract::{Json, State};
use btc_indexer_internals::{
    api::{AccountReplenishmentEvent, BtcIndexerApi},
    indexer::BtcIndexer,
};
use global_utils::common_types::{UrlWrapped, get_uuid};
use local_db_store_indexer::{
    PersistentRepoTrait,
    schemas::runes_spark::btc_indexer_work_checkpoint::{BtcIndexerWorkCheckpoint, StatusBtcIndexer, Task, Update},
};
use persistent_storage::error::DbError;
use serde::{Deserialize, Serialize};
use sqlx::types::{Json as SqlxJson, chrono::Utc};
use titan_client::TitanApi;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, trace};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{AppState, common::Empty, error::ServerError, routes::common::api_result_request::ApiResponseOwned};

const PATH_TO_LOG: &str = "btc_indexer_server:track_wallet";

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[schema(example = json!({
    "wallet": "sprt1pgss8fxt9jxuv4dgjwrg539s6u06ueausq076xvfej7wdah0htvjlxunt9fa4n",
    "callback_url": "127.0.0.1:8080"
}))]
pub struct TrackWalletRequest {
    pub wallet_id: String,
    pub callback_url: UrlWrapped,
}

#[utoipa::path(
    post,
    path = "/track_wallet",
    request_body = TrackWalletRequest,
    responses(
        (status = 200, description = "Success", body = Empty),
        (status = 400, description = "Bad Request", body = String),
        (status = 500, description = "Internal Server Error", body = String),
    ),
)]
#[instrument(skip(state))]
pub async fn handler(
    State(state): State<AppState<impl titan_client::TitanApi, impl PersistentRepoTrait + Clone + 'static>>,
    Json(payload): Json<TrackWalletRequest>,
) -> Result<Json<Empty>, ServerError> {
    info!("Received TrackWalletRequest: {:?}", payload);
    let uuid = get_uuid();
    {
        // Insert value in db to save info about processing some value
        let mut conn = state.persistent_storage.get_conn().await?;
        let time_now = Utc::now();
        BtcIndexerWorkCheckpoint {
            uuid,
            status: StatusBtcIndexer::Created,
            task: SqlxJson::from(Task::TrackWallet(payload.wallet_id.clone())),
            created_at: time_now,
            callback_url: payload.callback_url.clone(),
            error: None,
            updated_at: time_now,
        }
        .insert(&mut conn)
        .await?;
    }
    let cancellation_token = spawn_wallet_tracking_task(state.clone(), payload, uuid).await?;
    {
        let mut write_guard = state.cached_tasks.write().await;
        write_guard.insert(uuid, cancellation_token);
    }
    Ok(Json(Empty {}))
}

/// Spawns tracking task for tracking whether we receive event from indexer_internals and send via reqwest msg about completion
#[instrument(skip(app_state))]
pub(crate) async fn spawn_wallet_tracking_task<T: TitanApi, Db: PersistentRepoTrait + Clone + 'static>(
    app_state: AppState<T, Db>,
    payload: TrackWalletRequest,
    uuid: Uuid,
) -> Result<CancellationToken, DbError> {
    let cancellation_token = CancellationToken::new();
    tokio::task::spawn({
        let local_cancellation_token = cancellation_token.child_token();
        async move {
            let response = _retrieve_account_info_result(
                app_state.persistent_storage.clone(),
                app_state.btc_indexer,
                &payload,
                uuid,
                local_cancellation_token,
            )
            .await;
            let response = ApiResponseOwned::from(response).encode_string_json();
            trace!(
                "[{PATH_TO_LOG}] Formed response to send to callback url[{}]: {response:?}",
                payload.callback_url.0.to_string()
            );
            let _ = app_state
                .http_client
                .post(payload.callback_url.0.to_string())
                .header("Content-Type", "application/json")
                .body(response)
                .send()
                .await
                .inspect_err(|e| error!("[{PATH_TO_LOG}] Receive error on sending response: {:?}", e))
                .inspect(|r| debug!("[{PATH_TO_LOG}] (Finishing task execution) Receive response: {r:?}"));
            app_state.cached_tasks.write().await.remove(&uuid);
        }
    });
    Ok(cancellation_token)
}

#[instrument(level = "trace", skip(db, indexer, payload), fields(tx_id=payload.wallet_id) ret)]
async fn _retrieve_account_info_result<T: TitanApi, Db: PersistentRepoTrait + Clone>(
    db: Db,
    indexer: Arc<BtcIndexer<T, Db>>,
    payload: &TrackWalletRequest,
    uuid: Uuid,
    cancellation_token: CancellationToken,
) -> crate::error::Result<AccountReplenishmentEvent> {
    let confirmed_wallet_info = _inner_retrieve_account_info_result(indexer, &payload, uuid, cancellation_token).await;
    {
        let mut conn = db.get_conn().await?;
        let time_now = Utc::now();
        match confirmed_wallet_info.as_ref() {
            Ok(_) => {
                BtcIndexerWorkCheckpoint::update(
                    &mut conn,
                    &uuid,
                    &Update {
                        status: Some(StatusBtcIndexer::FinishedSuccess),
                        error: None,
                        updated_at: Some(time_now),
                    },
                )
                .await?;
            }
            Err(e) => {
                BtcIndexerWorkCheckpoint::update(
                    &mut conn,
                    &uuid,
                    &Update {
                        status: Some(StatusBtcIndexer::FinishedError),
                        error: Some(e.to_string()),
                        updated_at: Some(time_now),
                    },
                )
                .await?;
            }
        }
    }
    confirmed_wallet_info
}

async fn _inner_retrieve_account_info_result<T: TitanApi, Db: PersistentRepoTrait>(
    indexer: Arc<BtcIndexer<T, Db>>,
    payload: &&TrackWalletRequest,
    uuid: Uuid,
    cancellation_token: CancellationToken,
) -> Result<AccountReplenishmentEvent, ServerError> {
    let oneshot_receiver = indexer
        .track_account_changes(&payload.wallet_id, uuid)
        .await
        .inspect_err(|e| error!("[{PATH_TO_LOG}] Occurred error on signing on tx updates via channel, err: {e}"))?;
    tokio::select! {
        _ = cancellation_token.cancelled() => {
            info!("[{PATH_TO_LOG}] Position manager signal listener cancelled");
            Err(ServerError::TaskCancelled(PATH_TO_LOG.to_string()))
        }
        confirmed_wallet_info = oneshot_receiver => {
            Ok(confirmed_wallet_info?)
        }
    }
}
