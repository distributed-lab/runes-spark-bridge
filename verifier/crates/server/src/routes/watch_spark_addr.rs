use std::sync::Arc;

use axum::extract::{Json, State};
use btc_indexer_internals::{
    api::{AccountReplenishmentEvent, BtcIndexerApi},
    indexer::BtcIndexer,
};
use global_utils::common_types::{UrlWrapped, get_uuid};
use indexer_local_db_store::{
    PersistentRepoTrait,
    error::DbError,
    schemas::runes_spark::btc_indexer_work_checkpoint::{BtcIndexerWorkCheckpoint, StatusBtcIndexer, Task, Update},
};
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
    State(state): State<AppState<impl PersistentRepoTrait + Clone + 'static>>,
    Json(payload): Json<TrackWalletRequest>,
) -> Result<Json<Empty>, ServerError> {
    info!("Received TrackWalletRequest: {:?}", payload);
    todo!();
    Ok(Json(Empty {}))
}
