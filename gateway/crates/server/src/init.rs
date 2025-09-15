use crate::handlers;
use crate::state::AppState;
use axum::Router;
use axum::routing::post;
use gateway_flow_processor::flow_sender::FlowSender;
use tracing::instrument;

#[instrument(level = "debug", skip(flow_sender), ret)]
pub async fn create_app(flow_sender: FlowSender) -> anyhow::Result<Router> {
    let state = AppState { flow_sender };
    Ok(Router::new()
        .route("/api/user/runes-address", post(handlers::btc_addr_issuing::handle))
        .route("/api/user/bridge-runes", post(handlers::bridge_runes::handle))
        .route("/api/user/exit-spark", post(handlers::exit_spark::handle))
        .route(
            "/api/verifier/notify-runes-address",
            post(handlers::notify_runes_address::handle),
        )
        .route(
            "/api/verifier/notify-spark-address",
            post(handlers::notify_spark_address::handle),
        )
        .with_state(state))
}
