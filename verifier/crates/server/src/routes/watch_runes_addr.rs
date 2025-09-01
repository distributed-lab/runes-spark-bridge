use std::sync::Arc;

use axum::extract::{Json, State};
use btc_indexer_internals::{api::BtcIndexerApi, indexer::BtcIndexer};
use global_utils::common_types::{TxIdWrapped, UrlWrapped, get_uuid};
use indexer_local_db_store::{
    PersistentRepoTrait,
    error::DbError,
    schemas::runes_spark::btc_indexer_work_checkpoint::{BtcIndexerWorkCheckpoint, StatusBtcIndexer, Task, Update},
};
use serde::{Deserialize, Serialize};
use sqlx::{
    Row,
    types::{Json as SqlxJson, chrono::Utc},
};
use titan_client::{TitanApi, Transaction};
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
pub async fn handler<Db: PersistentRepoTrait + Clone + 'static>(
    State(state): State<AppState<Db>>,
    Json(payload): Json<TrackTxRequest>,
) -> Result<Json<Empty>, ServerError> {
    info!("Received track tx: {:?}", payload);
    todo!();
    Ok(Json(Empty {}))
}
