use async_trait::async_trait;
use frost_secp256k1_tr::{
    Identifier, Signature, SigningPackage,
    keys::{
        KeyPackage, PublicKeyPackage,
        dkg::{round1, round2},
    },
    round1::{SigningCommitments, SigningNonces},
    round2::SignatureShare,
};
use persistent_storage::error::DbError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::errors::{AggregatorError, SignerError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DkgRound1Request {
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DkgRound1Response {
    pub round1_package: round1::Package,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DkgRound2Request {
    pub user_id: String,
    pub round1_packages: BTreeMap<Identifier, round1::Package>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DkgRound2Response {
    pub round2_packages: BTreeMap<Identifier, round2::Package>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DkgFinalizeRequest {
    pub user_id: String,
    pub round1_packages: BTreeMap<Identifier, round1::Package>,
    pub round2_packages: BTreeMap<Identifier, round2::Package>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DkgFinalizeResponse {
    pub public_key_package: PublicKeyPackage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignRound1Request {
    pub user_id: String,
    pub session_id: String,
    pub tweak: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignRound1Response {
    pub user_id: String,
    pub session_id: String,
    pub commitments: SigningCommitments, // Only commitment
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignRound2Request {
    pub user_id: String,
    pub session_id: String,
    pub signing_package: SigningPackage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignRound2Response {
    pub session_id: String,
    pub signature_share: SignatureShare,
}

#[async_trait]
pub trait SignerClient: Send + Sync {
    async fn dkg_round_1(&self, request: DkgRound1Request) -> Result<DkgRound1Response, AggregatorError>;

    async fn dkg_round_2(&self, request: DkgRound2Request) -> Result<DkgRound2Response, AggregatorError>;

    async fn dkg_finalize(&self, request: DkgFinalizeRequest) -> Result<DkgFinalizeResponse, AggregatorError>;

    async fn sign_round_1(&self, request: SignRound1Request) -> Result<SignRound1Response, AggregatorError>;

    async fn sign_round_2(&self, request: SignRound2Request) -> Result<SignRound2Response, AggregatorError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregatorUserState {
    DkgRound1 {
        round1_packages: BTreeMap<Identifier, round1::Package>,
    },
    DkgRound2 {
        round1_packages: BTreeMap<Identifier, round1::Package>,
        round2_packages: BTreeMap<Identifier, BTreeMap<Identifier, round2::Package>>,
    },
    DkgFinalized {
        public_key_package: PublicKeyPackage,
    },
    SigningRound1 {
        tweak: Option<Vec<u8>>,
        message: Vec<u8>,
        signing_package: SigningPackage,
        public_key_package: PublicKeyPackage,
    },
    SigningRound2 {
        tweak: Option<Vec<u8>>,
        message: Vec<u8>,
        public_key_package: PublicKeyPackage,
        signature: Signature,
    },
}

#[async_trait]
pub trait AggregatorUserStorage: Send + Sync {
    async fn get_user_state(&self, user_id: String) -> Result<Option<AggregatorUserState>, DbError>;
    async fn set_user_state(&self, user_id: String, state: AggregatorUserState) -> Result<(), DbError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignerUserState {
    DkgRound1 {
        round1_secret_package: round1::SecretPackage,
    },
    DkgRound2 {
        round2_secret_package: round2::SecretPackage,
        round1_packages: BTreeMap<Identifier, round1::Package>,
    },
    DkgFinalized {
        key_package: KeyPackage,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignerSessionState {
    SigningRound1 {
        key_package: KeyPackage,
        tweak: Option<Vec<u8>>,
        nonces: SigningNonces,
    },
    SigningRound2 {
        key_package: KeyPackage,
        tweak: Option<Vec<u8>>,
        signature_share: SignatureShare,
    },
}

#[async_trait]
pub trait SignerUserStorage: Send + Sync {
    async fn get_user_state(&self, user_id: String) -> Result<Option<SignerUserState>, DbError>;
    async fn set_user_state(&self, user_id: String, state: SignerUserState) -> Result<(), DbError>;
}

#[async_trait]
pub trait SignerSessionStorage: Send + Sync {
    async fn get_session_state(
        &self,
        user_id: String,
        session_id: String,
    ) -> Result<Option<SignerSessionState>, DbError>;

    async fn set_session_state(
        &self,
        user_id: String,
        session_id: String,
        state: SignerSessionState,
    ) -> Result<(), DbError>;
}
