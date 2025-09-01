use std::{env::VarError, io, net::AddrParseError, num::ParseIntError};

use bitcoincore_rpc::bitcoin::network::ParseNetworkError;
use config::ConfigError;
use global_utils::env_parser::EnvParserError;
use thiserror::Error;

pub type Result<T> = core::result::Result<T, ConfigParserError>;

#[derive(Debug, Error)]
pub enum ConfigParserError {
    #[error("Failed to convert SocketAddr, err: {0}")]
    SocketConversionError(#[from] AddrParseError),
    #[error("Failed to parse address, error: {0}")]
    ConfigMergingError(#[from] ConfigError),
    #[error(transparent)]
    ConfigEnvParseError(#[from] EnvParserError),
    #[error("Occurred custom error: {0}")]
    Custom(String),
    #[error("Failed to parse integer, var name: {var_name}, err: {err}")]
    ParseIntError { var_name: String, err: ParseIntError },
    #[error("Failed to parse Btc network, err: {0}")]
    ParseNetworkError(#[from] ParseNetworkError),
}
