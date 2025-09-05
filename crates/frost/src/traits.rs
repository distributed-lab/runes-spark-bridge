use std::collections::{BTreeMap, HashMap};
use async_trait::async_trait;
use bitcoin::secp256k1::PublicKey;
use frost_secp256k1_tr::{
    Identifier, Signature, SigningPackage,
    keys::{
        KeyPackage, PublicKeyPackage,
        dkg::{round1, round2},
    },
    round1::{SigningCommitments, SigningNonces},
    round2::SignatureShare,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;
use crate::errors::{AggregatorError, SignerError};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DkgRound1Request {
    pub user_id: PublicKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DkgRound1Response {
    pub round1_package: round1::Package,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DkgRound2Request {
    pub user_id: PublicKey,
    pub round1_packages: BTreeMap<Identifier, round1::Package>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DkgRound2Response {
    pub round2_packages: BTreeMap<Identifier, round2::Package>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DkgFinalizeRequest {

    pub user_id: PublicKey,
    pub round1_packages: BTreeMap<Identifier, round1::Package>,
    pub round2_packages: BTreeMap<Identifier, round2::Package>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DkgFinalizeResponse {
    pub public_key_package: PublicKeyPackage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignRound1Request {
    pub user_id: PublicKey,
    pub session_id: Uuid,
    pub tweak: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignRound1Response {
    pub user_id: PublicKey,
    pub session_id: Uuid,
    pub commitments: SigningCommitments, // Only commitment
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignRound2Request {
    pub user_id: PublicKey,
    pub session_id: Uuid,
    pub signing_package: SigningPackage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignRound2Response {
    pub session_id: Uuid,
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
    async fn get_user_state(&self, user_id: PublicKey) -> Result<Option<AggregatorUserState>, AggregatorError>;
    async fn set_user_state(&self, user_id: PublicKey, state: AggregatorUserState) -> Result<(), AggregatorError>;
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
    async fn get_user_state(&self, user_id: PublicKey) -> Result<Option<SignerUserState>, SignerError>;
    async fn set_user_state(&self, user_id: PublicKey, state: SignerUserState) -> Result<(), SignerError>;
}

#[async_trait]
pub trait SignerSessionStorage: Send + Sync {
    async fn get_session_state(
        &self,
        user_id: PublicKey,
        session_id: Uuid,
    ) -> Result<Option<SignerSessionState>, SignerError>;

    async fn set_session_state(
        &self,
        user_id: PublicKey,
        session_id: Uuid,
        state: SignerSessionState,
    ) -> Result<(), SignerError>;
}
