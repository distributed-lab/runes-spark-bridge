use std::{collections::HashMap, sync::Arc};

use axum::{Router, routing::post};
use btc_indexer_internals::indexer::BtcIndexer;
use indexer_local_db_store::init::LocalDbIndexer;
use titan_client::TitanClient;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::instrument;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState<Db> {
    pub http_client: reqwest::Client,
    pub persistent_storage: Db,
}

#[derive(OpenApi)]
#[openapi(paths(
    crate::routes::create_commitment::handler,
    crate::routes::create_partial_signature::handler,
    crate::routes::watch_runes_addr::handler,
    crate::routes::watch_spark_addr::handler
))]
struct ApiDoc;

#[instrument(skip(db_pool, btc_indexer))]
pub async fn create_app(db_pool: LocalDbIndexer, btc_indexer: BtcIndexer<TitanClient, LocalDbIndexer>) -> Router {
    let state = AppState {
        http_client: reqwest::Client::new(),
        persistent_storage: db_pool,
    };
    let app = Router::new()
        .route("/create_commitment", post(crate::routes::watch_runes_addr::handler))
        .route(
            "/create_partial_signature",
            post(crate::routes::watch_spark_addr::handler),
        )
        .route("/create_commitment", post(crate::routes::create_commitment::handler))
        .route(
            "/create_partial_signature",
            post(crate::routes::create_partial_signature::handler),
        )
        .with_state(state);

    #[cfg(feature = "swagger")]
    let app = app.merge(SwaggerUi::new("/swagger-ui/").url("/api-docs/openapi.json", ApiDoc::openapi()));
    //todo: spawn initial task to renew unfinished ones
    app
}
