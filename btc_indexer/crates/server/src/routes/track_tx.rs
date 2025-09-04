use std::sync::Arc;

use axum::extract::{Json, State};
use btc_indexer_internals::{api::BtcIndexerApi, indexer::BtcIndexer};
use global_utils::common_types::{TxIdWrapped, UrlWrapped, get_uuid};
use local_db_store_indexer::{
    PersistentRepoTrait,
    schemas::runes_spark::btc_indexer_work_checkpoint::{BtcIndexerWorkCheckpoint, StatusBtcIndexer, Task, Update},
};
use persistent_storage::error::DbError;
use serde::{Deserialize, Serialize};
use sqlx::types::{Json as SqlxJson, chrono::Utc};
use titan_client::Transaction;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, trace};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{AppState, common::Empty, error::ServerError, routes::common::api_result_request::ApiResponseOwned};

const PATH_TO_LOG: &str = "btc_indexer_server:track_tx";

#[derive(Deserialize, Serialize, ToSchema, Debug)]
#[schema(example = json!({
    "tx_id": "fb0c9ab881331ec7acdd85d79e3197dcaf3f95055af1703aeee87e0d853e81ec",
    "callback_url": "http://127.0.0.1:8080"
}))]
pub struct TrackTxRequest {
    pub tx_id: TxIdWrapped,
    pub callback_url: UrlWrapped,
}

#[utoipa::path(
    post,
    path = "/track_tx",
    request_body = TrackTxRequest,
    responses(
        (status = 200, description = "Success", body = Empty),
        (status = 400, description = "Bad Request", body = String),
        (status = 500, description = "Internal Server Error", body = String),
    ),
)]
#[instrument(skip(state))]
pub async fn handler<T: titan_client::TitanApi, Db: PersistentRepoTrait + Clone + 'static>(
    State(state): State<AppState<T, Db>>,
    Json(payload): Json<TrackTxRequest>,
) -> Result<Json<Empty>, ServerError> {
    info!("Received track tx: {:?}", payload);
    let uuid = get_uuid();
    {
        // Insert value in db to save info about processing some value
        let mut conn = state.persistent_storage.get_conn().await?;
        let time_now = Utc::now();
        BtcIndexerWorkCheckpoint {
            uuid,
            status: StatusBtcIndexer::Created,
            task: SqlxJson::from(Task::TrackTx(payload.tx_id.clone())),
            created_at: time_now,
            callback_url: payload.callback_url.clone(),
            error: None,
            updated_at: time_now,
        }
        .insert(&mut conn)
        .await?;
    }

    let cancellation_token = spawn_tx_tracking_task(state.clone(), payload, uuid).await?;
    {
        let mut write_guard = state.cached_tasks.write().await;
        write_guard.insert(uuid, cancellation_token);
    }
    Ok(Json(Empty {}))
}

/// Spawns tracking task for tracking whether we receive event from indexer_internals and send via reqwest msg about completion
#[instrument(skip(app_state))]
pub(crate) async fn spawn_tx_tracking_task<T: titan_client::TitanApi, Db: PersistentRepoTrait + Clone + 'static>(
    app_state: AppState<T, Db>,
    payload: TrackTxRequest,
    uuid: Uuid,
) -> Result<CancellationToken, DbError> {
    let cancellation_token = CancellationToken::new();
    tokio::task::spawn({
        let local_cancellation_token = cancellation_token.child_token();
        async move {
            let response = _retrieve_tx_info_result(
                app_state.persistent_storage,
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

#[instrument(level = "trace", skip(db, indexer, payload), fields(tx_id=payload.tx_id.0.to_string()) ret)]
async fn _retrieve_tx_info_result<T: titan_client::TitanApi, Db: PersistentRepoTrait + Clone + 'static>(
    db: Db,
    indexer: Arc<BtcIndexer<T, Db>>,
    payload: &TrackTxRequest,
    uuid: Uuid,
    cancellation_token: CancellationToken,
) -> crate::error::Result<Transaction> {
    let confirmed_tx = _inner_retrieve_tx_info_result(indexer, payload, uuid, cancellation_token).await;
    {
        let mut conn = db.get_conn().await?;
        let time_now = Utc::now();
        match confirmed_tx.as_ref() {
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
    confirmed_tx
}

async fn _inner_retrieve_tx_info_result<T: titan_client::TitanApi, Db: PersistentRepoTrait + Clone + 'static>(
    indexer: Arc<BtcIndexer<T, Db>>,
    payload: &TrackTxRequest,
    uuid: Uuid,
    cancellation_token: CancellationToken,
) -> Result<Transaction, ServerError> {
    let oneshot_receiver = indexer
        .track_tx_changes(payload.tx_id.0, uuid)
        .await
        .inspect_err(|e| error!("[{PATH_TO_LOG}] Occurred error on signing on tx updates via channel, err: {e}"))?;
    tokio::select! {
        _ = cancellation_token.cancelled() => {
            info!("[{PATH_TO_LOG}] Position manager signal listener cancelled");
            Err(ServerError::TaskCancelled(PATH_TO_LOG.to_string()))
        }
        confirmed_tx = oneshot_receiver => {
            Ok(confirmed_tx?)
        }
    }
}
