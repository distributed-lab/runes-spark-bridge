use thiserror::Error;

#[derive(Error, Debug)]
pub enum BtcSignerError {
    #[error("Cryptographic error: {0}")]
    CryptoError(#[from] secp256k1::Error),

    #[error("Invalid signature format")]
    InvalidSignature,

    #[error("Invalid public key format")]
    InvalidPublicKey,

    #[error("Invalid message format")]
    InvalidMessage,

    #[error("Signer not found: {id}")]
    SignerNotFound { id: String },

    #[error("Session not found: {id}")]
    SessionNotFound { id: String },

    #[error("Insufficient partial signatures: got {got}, need {need}")]
    InsufficientSignatures { got: usize, need: usize },

    #[error("Aggregation failed: {reason}")]
    AggregationFailed { reason: String },

    #[error("Invalid threshold: {threshold}, must be <= {total}")]
    InvalidThreshold { threshold: u32, total: u32 },

    #[error("Duplicate signer: {id}")]
    DuplicateSigner { id: String },

    #[error("Invalid session state: {state}")]
    InvalidSessionState { state: String },

    #[error("Hex decode error: {0}")]
    HexError(#[from] hex::FromHexError),

    #[error("Transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    #[error("gRPC status error: {0}")]
    Status(#[from] tonic::Status),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, BtcSignerError>;