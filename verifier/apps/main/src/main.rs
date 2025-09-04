use anyhow::anyhow;
use global_utils::config_variant::ConfigVariant;
use global_utils::env_parser::lookup_ip_addr;
use global_utils::logger::init_logger;
use persistent_storage::{config::PostgresDbCredentials, init::PostgresRepo};
use tokio::net::TcpListener;
use tracing::instrument;
use verifier_config_parser::config::ServerConfig;
use verifier_utils::frost_signer::create_frost_signer;

#[instrument(level = "debug", ret)]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _logger_guard = init_logger();

    let config_path = std::env::var("CONFIG_PATH").unwrap();

    let app_config = ServerConfig::init_config(ConfigVariant::OnlyOneFilepath(config_path))?;
    tracing::debug!("App config: {:?}", app_config);

    let postgres_url = std::env::var("DATABASE_URL").unwrap();
    let postgres_creds = PostgresDbCredentials { url: postgres_url };

    let db_pool = PostgresRepo::from_config(postgres_creds).await?.into_shared();
    let frost_signer = create_frost_signer(app_config.frost_signer);
    let app = verifier_server::init::create_app(frost_signer).await?;

    let addr_to_listen = (lookup_ip_addr(&app_config.server.ip)?, app_config.server.port);
    let listener = TcpListener::bind(addr_to_listen)
        .await
        .map_err(|e| anyhow!("Failed to bind to address: {}", e))?;
    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow!("Failed to serve: {}", e))
}
