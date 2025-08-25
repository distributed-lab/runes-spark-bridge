use btc_signer::{
    BtcMultiSigner, BtcSigner, BtcSignerServer,
    traits::{Signer, MultiSigner},
    errors::Result,
};
use std::net::SocketAddr;
use tokio::signal;
use tracing::{info, error};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("Starting HamsterCoin...");

    // storage for signers
    let mut multi_signer = BtcMultiSigner::new();

    for i in 0..5 { // demo of 5 signers
        let signer = Box::new(BtcSigner::new(format!("signer_{}", i)));
        info!("Added signer: signer_{}", i);
        multi_signer.add_signer(signer).await?;
    }

    info!("Created multisigner with {} signers", multi_signer.signer_count());

    let server = BtcSignerServer::new(multi_signer); // gRPC!!!
    let addr: SocketAddr = "127.0.0.1:50051".parse()
        .expect("Failed to parse server address");

    info!("Starting gRPC server on {}", addr);

    let server_task = tokio::spawn(async move { // ???
        if let Err(e) = server.serve(addr).await {
            error!("Server error: {}", e);
        }
    });

    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Okay shut down");
        }
        Err(err) => {
            error!("Error at listen: {}", err);
        }
    }

    server_task.abort();

    info!("Ended okay");
    Ok(())
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub server_addr: SocketAddr,
    pub signer_count: usize,
    pub default_threshold: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_addr: "127.0.0.1:50051".parse().unwrap(),
            signer_count: 5,
            default_threshold: 3,
        }
    }
}

impl AppConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(addr) = std::env::var("SERVER_ADDR") {
            if let Ok(parsed_addr) = addr.parse() {
                config.server_addr = parsed_addr;
            }
        }

        if let Ok(count) = std::env::var("SIGNER_COUNT") {
            if let Ok(parsed_count) = count.parse() {
                config.signer_count = parsed_count;
            }
        }

        if let Ok(threshold) = std::env::var("DEFAULT_THRESHOLD") {
            if let Ok(parsed_threshold) = threshold.parse() {
                config.default_threshold = parsed_threshold;
            }
        }

        config
    }
}