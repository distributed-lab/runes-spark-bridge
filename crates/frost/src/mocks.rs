use std::{collections::BTreeMap, sync::Arc};

use persistent_storage::error::DbError;

use crate::{
    errors::{AggregatorError, SignerError},
    signer::FrostSigner,
    traits::*,
};
use async_trait::async_trait;
use bitcoin::secp256k1::PublicKey;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct MockSignerUserStorage {
    user_states: Arc<Mutex<BTreeMap<PublicKey, DkgUserState>>>,
}

pub struct MockSignerSessionStorage {
    session_state: Arc<Mutex<BTreeMap<(PublicKey, Uuid), SignerSessionState>>>,
}

impl MockSignerSessionStorage {
    pub fn new() -> Self {
        Self {
            session_state: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    pub async fn has_session(&self, user_id: &PublicKey, session_id: &Uuid) -> bool {
        let map = self.session_state.lock().await;
        map.contains_key(&(*user_id, *session_id))
    }
}

#[async_trait]
impl SignerSessionStorage for MockSignerSessionStorage {
    async fn get_session_state(
        &self,
        user_id: PublicKey,
        session_id: Uuid,
    ) -> Result<Option<SignerSessionState>, DbError> {
        Ok(self
            .session_state
            .lock()
            .await
            .get(&(user_id.clone(), session_id.clone()))
            .cloned())
    }

    async fn set_session_state(
        &self,
        user_id: PublicKey,
        session_id: Uuid,
        state: SignerSessionState,
    ) -> Result<(), DbError> {
        self.session_state
            .lock()
            .await
            .insert((user_id.clone(), session_id.clone()), state);
        Ok(())
    }
}

impl MockSignerUserStorage {
    pub fn new() -> Self {
        Self {
            user_states: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
}

#[async_trait]
impl SignerUserStorage for MockSignerUserStorage {
    async fn get_user_state(&self, user_id: PublicKey) -> Result<Option<DkgUserState>, DbError> {
        Ok(self.user_states.lock().await.get(&user_id).map(|state| state.clone()))
    }

    async fn set_user_state(&self, user_id: PublicKey, state: DkgUserState) -> Result<(), DbError> {
        self.user_states.lock().await.insert(user_id, state);
        Ok(())
    }
}

pub struct MockAggregatorUserStorage {
    user_states: Arc<Mutex<BTreeMap<PublicKey, AggregatorUserState>>>,
}

impl MockAggregatorUserStorage {
    pub fn new() -> Self {
        Self {
            user_states: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
}

#[async_trait]
impl AggregatorUserStorage for MockAggregatorUserStorage {
    async fn get_user_state(&self, user_id: PublicKey) -> Result<Option<AggregatorUserState>, DbError> {
        Ok(self.user_states.lock().await.get(&user_id).map(|state| state.clone()))
    }

    async fn set_user_state(&self, user_id: PublicKey, state: AggregatorUserState) -> Result<(), DbError> {
        self.user_states.lock().await.insert(user_id, state);
        Ok(())
    }
}

#[derive(Clone)]
pub struct MockSignerClient {
    signer: FrostSigner,
}

impl MockSignerClient {
    pub fn new(signer: FrostSigner) -> Self {
        Self { signer }
    }
}

#[async_trait]
impl SignerClient for MockSignerClient {
    async fn dkg_round_1(&self, request: DkgRound1Request) -> Result<DkgRound1Response, AggregatorError> {
        self.signer
            .dkg_round_1(request)
            .await
            .map_err(|e| AggregatorError::SignerError(e))
    }

    async fn dkg_round_2(&self, request: DkgRound2Request) -> Result<DkgRound2Response, AggregatorError> {
        self.signer
            .dkg_round_2(request)
            .await
            .map_err(|e| AggregatorError::SignerError(e))
    }

    async fn dkg_finalize(&self, request: DkgFinalizeRequest) -> Result<DkgFinalizeResponse, AggregatorError> {
        self.signer
            .dkg_finalize(request)
            .await
            .map_err(|e| AggregatorError::SignerError(e))
    }

    async fn sign_round_1(&self, request: SignRound1Request) -> Result<SignRound1Response, AggregatorError> {
        self.signer
            .sign_round_1(request)
            .await
            .map_err(|e| AggregatorError::SignerError(e))
    }

    async fn sign_round_2(&self, request: SignRound2Request) -> Result<SignRound2Response, AggregatorError> {
        self.signer
            .sign_round_2(request)
            .await
            .map_err(|e| AggregatorError::SignerError(e))
    }
}
