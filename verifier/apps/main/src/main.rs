use std::{net::IpAddr, str::FromStr};

use btc_indexer_internals::indexer::{BtcIndexer, IndexerParams};
use global_utils::{env_parser::lookup_ip_addr, logger::init_logger};
use indexer_config_parser::config::{BtcRpcCredentials, ConfigVariant, ServerConfig};
use indexer_local_db_store::{PostgresDbCredentials, init::LocalDbIndexer};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _logger_guard = init_logger();
    let app_config = ServerConfig::init_config(ConfigVariant::init())?;
    let (btc_creds, postgres_creds) = (BtcRpcCredentials::new()?, PostgresDbCredentials::from_envs()?);
    let db_pool = LocalDbIndexer::from_config(postgres_creds).await?;
    let btc_indexer = BtcIndexer::with_api(IndexerParams {
        btc_rpc_creds: btc_creds,
        db_pool: db_pool.clone(),
        btc_indexer_params: app_config.btc_indexer_config,
    })?;
    let app = btc_indexer_server::create_app(db_pool, btc_indexer).await;

    let addr_to_listen = (
        lookup_ip_addr(&app_config.app_config.http_server_ip)?,
        app_config.app_config.http_server_port,
    );
    let listener = TcpListener::bind(addr_to_listen).await?;

    tracing::info!("Listening on {:?}", addr_to_listen);
    #[cfg(feature = "swagger")]
    tracing::info!("Swagger UI available at {:?}/swagger-ui/", addr_to_listen);

    axum::serve(listener, app).await?;

    Ok(())
}
