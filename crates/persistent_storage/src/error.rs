use config_parser::error::ConfigParserError;
use sqlx::migrate::MigrateError;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Unable to retrieve env variable for db initialization, error: {0}")]
    UnableToRetrieveEnvVar(String),
    #[error("Failed to establish connection with db, please check url [err: {0}, url: {1}]")]
    FailedToEstablishDbConn(sqlx::Error, String),
    #[error("Failed to initialize initial db config, error: {0}")]
    FailedToParseConfig(#[from] ConfigParserError),
    #[error("Failed to migrate db, error: {0}")]
    FailedToMigrateDb(#[from] MigrateError),
    #[error("Unable to retrieve db connection, error: {0}")]
    UnableToRetrieveConnection(#[from] sqlx::Error),
    #[error("Bad request error: {0}")]
    BadRequest(String),
    #[error("Invalid data error: {0}")]
    InvalidData(String),
    #[error("Signer error: {0}")]
    SignerError(String),
    #[error("Aggregator error: {0}")]
    AggregatorError(String),
}
