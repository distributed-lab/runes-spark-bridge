pub use persistent_storage::error::*;
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Failed to open Pg client, error: {0}")]
    DbError(#[from] sqlx::Error),
    #[error("Failed to convert type from json, error: {0}")]
    SerdeJsonError(#[from] serde_json::error::Error),
}

pub type Result<T> = std::result::Result<T, DbError>;
