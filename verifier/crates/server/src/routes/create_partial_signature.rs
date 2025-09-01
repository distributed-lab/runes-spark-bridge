use axum::{Json, extract::State};
use indexer_local_db_store::PersistentRepoTrait;
use tracing::instrument;

use crate::{AppState, common::Empty, error::ServerError, routes::watch_spark_addr::TrackWalletRequest};

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
    todo!()
}
