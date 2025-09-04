use btc_indexer_internals::error::BtcIndexerError;
use persistent_storage::error::DbError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Bad Request: {0}")]
    BadRequest(String),
    #[error("Internal Server Error: {0}")]
    InternalError(String),
    #[error("Not Found: {0}")]
    NotFound(String),
    #[error("Btc indexer: {0}")]
    BtcIndexerError(#[from] BtcIndexerError),
    #[error("One shot channel error: {0}")]
    OneshotRecvError(#[from] tokio::sync::oneshot::error::RecvError),
    #[error("Your task was cancelled, msg: {0}")]
    TaskCancelled(String),
    #[error("Database error, msg: {0}")]
    DatabaseError(#[from] DbError),
}

mod response_conversion {
    use axum::http::StatusCode;

    use super::*;
    impl axum::response::IntoResponse for ServerError {
        fn into_response(self) -> axum::response::Response {
            self.into_status_msg_tuple().into_response()
        }
    }

    impl ServerError {
        pub fn into_status_msg_tuple(self) -> (StatusCode, String) {
            match self {
                ServerError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
                ServerError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
                ServerError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.to_string()),
                ServerError::BtcIndexerError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
                ServerError::OneshotRecvError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
                ServerError::TaskCancelled(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
                ServerError::DatabaseError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, ServerError>;
