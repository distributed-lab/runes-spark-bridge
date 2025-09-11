use crate::types::BtcAddrIssueRequest;
use bitcoin::secp256k1;
use frost::errors::AggregatorError;
use gateway_local_db_store::schemas::deposit_address::DepositStatus;
use global_utils::tweak_generation::TweakGeneratorError;
use persistent_storage::error::DbError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FlowProcessorError {
    #[error("Channel closed error: {0}")]
    ChannelClosedError(String),
    #[error("Invalid response type: {0}")]
    InvalidResponseType(String),
    #[error("Frost aggregator error: {0}")]
    FrostAggregatorError(String),
    #[error("Invalid data error: {0}")]
    InvalidDataError(String),
    #[error("Database error: {0}")]
    DbError(#[from] DbError),
    #[error("Elliptic curve (secp256k1) error: {0}")]
    Secp256k1Error(#[from] secp256k1::Error),
    #[error("Failed conversion to TweakedPubkey error: {0}")]
    TweakingConversionError(String),
    #[error("Occurred problem with issuing btc addr: {0}")]
    BtcAddrIssueError(#[from] BtcAddrIssueErrorEnum),
}

#[derive(Error, Debug)]
pub enum BtcAddrIssueErrorEnum {
    #[error("Unfinished dkg state, please wait for completion. got: {got}, has to be Finalized")]
    UnfinishedDkgState { got: String },
    #[error("No required entry in db for request: {0:?}, while MuSigId exists")]
    NoDepositAddrInfoInDb(BtcAddrIssueRequest),
    #[error("Occurred error on Aggregator, failed to finalize dkg, err: {0}")]
    AggregatorError(#[from] AggregatorError),
    #[error("Occurred error tweak generation, err: {0}")]
    TweakGenerationError(#[from] TweakGeneratorError),
    #[error("Failed to change pubkey address, err: {context}")]
    ChangePubkeyAddr { context: String },
    #[error(
        "Obtained wrong status on issuing btc addr for replenishment, context: '{context}', got: {got:?}, expected: {expected:?}"
    )]
    WrongStatus {
        context: String,
        got: DepositStatus,
        expected: DepositStatus,
    },
    #[error("Database error occurred, err: {0}")]
    DbError(#[from] DbError),
}
