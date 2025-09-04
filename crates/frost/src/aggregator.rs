use std::{collections::BTreeMap, sync::Arc};

use crate::{config::AggregatorConfig, errors::AggregatorError, traits::*};
use frost_secp256k1_tr::{Identifier, Signature, SigningPackage, keys, keys::Tweak};
use futures::future::join_all;
use tracing::{debug, instrument};
use uuid::Uuid;

#[derive(Clone)]
pub struct FrostAggregator {
    config: AggregatorConfig,
    verifiers: BTreeMap<Identifier, Arc<dyn SignerClient>>, // TODO: implement signer client
    user_storage: Arc<dyn AggregatorUserStorage>,           // TODO: implement aggregator storage storage
}

impl FrostAggregator {
    pub fn new(
        config: AggregatorConfig,
        verifiers: BTreeMap<Identifier, Arc<dyn SignerClient>>,
        user_storage: Arc<dyn AggregatorUserStorage>,
    ) -> Self {
        Self {
            config,
            verifiers,
            user_storage,
        }
    }

    #[instrument(level = "debug", skip(self), ret)]
    pub async fn dkg_round_1(&self, user_id: String) -> Result<(), AggregatorError> {
        let state = self.user_storage.get_user_state(user_id.clone()).await?;
        debug!("round1 current state: {:?}", state);
        match state {
            Some(_) => Err(AggregatorError::InvalidUserState("User state is not None".to_string())),
            None => {
                let signer_clients_request = DkgRound1Request {
                    user_id: user_id.clone(),
                };

                let mut verifier_responses = BTreeMap::new();
                let mut join_handles = vec![];

                for (verifier_id, signer_client) in self.verifiers.clone() {
                    let verifier_signer_clients_request = signer_clients_request.clone();
                    let join_handle = async move {
                        (
                            verifier_id,
                            signer_client.dkg_round_1(verifier_signer_clients_request).await,
                        )
                    };
                    join_handles.push(join_handle);
                }

                let join_handles = join_all(join_handles).await;

                for (verifier_id, response) in join_handles {
                    verifier_responses.insert(verifier_id, response?.round1_package);
                }

                self.user_storage
                    .set_user_state(
                        user_id.clone(),
                        AggregatorUserState::DkgRound1 {
                            round1_packages: verifier_responses,
                        },
                    )
                    .await?;

                Ok(())
            }
        }
    }

    #[instrument(level = "debug", skip(self), ret)]
    async fn dkg_round_2(&self, user_id: String) -> Result<(), AggregatorError> {
        let state = self.user_storage.get_user_state(user_id.clone()).await?;
        debug!("round2 current state: {:?}", state);

        match state {
            Some(AggregatorUserState::DkgRound1 { round1_packages }) => {
                let mut verifier_responses = BTreeMap::new();
                let mut join_handles = vec![];

                for (verifier_id, signer_client) in self.verifiers.clone() {
                    let mut packages = round1_packages.clone();
                    packages.remove(&verifier_id);
                    let signer_requests = DkgRound2Request {
                        user_id: user_id.clone(),
                        round1_packages: packages,
                    };
                    let join_handle = async move { (verifier_id, signer_client.dkg_round_2(signer_requests).await) };
                    join_handles.push(join_handle);
                }

                let join_handles = join_all(join_handles).await;

                for (verifier_id, response) in join_handles {
                    for (receiver_identifier, round2_package) in response?.round2_packages {
                        verifier_responses
                            .entry(receiver_identifier)
                            .or_insert(BTreeMap::new())
                            .insert(verifier_id, round2_package);
                    }
                }

                self.user_storage
                    .set_user_state(
                        user_id.clone(),
                        AggregatorUserState::DkgRound2 {
                            round1_packages: round1_packages,
                            round2_packages: verifier_responses,
                        },
                    )
                    .await?;
                Ok(())
            }
            _ => Err(AggregatorError::InvalidUserState(
                "User state is not DkgRound2".to_string(),
            )),
        }
    }

    #[instrument(level = "debug", skip(self), ret)]
    async fn dkg_finalize(&self, user_id: String) -> Result<(), AggregatorError> {
        let state = self.user_storage.get_user_state(user_id.clone()).await?;
        debug!("finalize current state: {:?}", state);

        match state {
            Some(AggregatorUserState::DkgRound2 {
                round1_packages,
                round2_packages,
            }) => {
                let mut public_key_packages = vec![];
                let mut join_handles = vec![];

                for (verifier_id, signer_client) in self.verifiers.clone() {
                    let mut verifier_round1_packages = round1_packages.clone();
                    verifier_round1_packages.remove(&verifier_id);
                    let request = DkgFinalizeRequest {
                        user_id: user_id.clone(),
                        round1_packages: verifier_round1_packages,
                        round2_packages: round2_packages
                            .get(&verifier_id)
                            .ok_or(AggregatorError::Internal("Round2 packages not found".to_string()))?
                            .clone(),
                    };
                    let join_handle = async move { (verifier_id, signer_client.dkg_finalize(request).await) };
                    join_handles.push(join_handle);
                }

                let join_handles = join_all(join_handles).await;

                for (verifier_id, response) in join_handles {
                    let public_key_package = response?.public_key_package;
                    public_key_packages.push(public_key_package);
                }

                let public_key_package = public_key_packages[0].clone();
                for _public_key_package in public_key_packages {
                    if public_key_package != _public_key_package {
                        return Err(AggregatorError::Internal(
                            "Public key packages are not equal".to_string(),
                        ));
                    }
                }

                self.user_storage
                    .set_user_state(
                        user_id.clone(),
                        AggregatorUserState::DkgFinalized {
                            public_key_package: public_key_package.clone(),
                        },
                    )
                    .await?;

                Ok(())
            }
            _ => Err(AggregatorError::InvalidUserState(
                "User state is not DkgFinalized".to_string(),
            )),
        }
    }

    #[instrument(level = "debug", skip(self), ret)]
    pub async fn run_dkg_flow(&self, user_id: String) -> Result<keys::PublicKeyPackage, AggregatorError> {
        self.dkg_round_1(user_id.clone()).await?;
        self.dkg_round_2(user_id.clone()).await?;
        self.dkg_finalize(user_id.clone()).await?;

        let state = self.user_storage.get_user_state(user_id.clone()).await?;
        match state {
            Some(AggregatorUserState::DkgFinalized { public_key_package }) => Ok(public_key_package),
            _ => Err(AggregatorError::InvalidUserState(
                "User state is not DkgFinalized".to_string(),
            )),
        }
    }

    #[instrument(level = "debug", skip(self), ret)]
    async fn sign_round_1(
        &self,
        user_id: String,
        session_id: String,
        message: &[u8],
        tweak: Option<&[u8]>,
    ) -> Result<(), AggregatorError> {
        let state = self.user_storage.get_user_state(user_id.clone()).await?;
        match state {
            Some(AggregatorUserState::DkgFinalized { public_key_package }) => {
                let mut commitments = BTreeMap::new();
                let mut join_handles = vec![];

                for (verifier_id, signer_client) in self.verifiers.clone() {
                    let request = SignRound1Request {
                        user_id: user_id.clone(),
                        session_id: session_id.clone(),
                        tweak: tweak.map(|t| t.to_vec()),
                    };
                    let join_handle = async move { (verifier_id, signer_client.sign_round_1(request).await) };
                    join_handles.push(join_handle);
                }

                let join_handles = join_all(join_handles).await;

                for (verifier_id, response) in join_handles {
                    commitments.insert(verifier_id, response?.commitments);
                }

                let signing_package = SigningPackage::new(commitments.clone(), message);

                self.user_storage
                    .set_user_state(
                        user_id.clone(),
                        AggregatorUserState::SigningRound1 {
                            tweak: tweak.map(|t| t.to_vec()),
                            message: message.to_vec(),
                            signing_package,
                            public_key_package,
                        },
                    )
                    .await?;

                Ok(())
            }
            _ => Err(AggregatorError::InvalidUserState(
                "User state is not DkgFinalized".to_string(),
            )),
        }
    }

    #[instrument(level = "debug", skip(self), ret)]
    async fn sign_round_2(&self, user_id: String, session_id: String) -> Result<(), AggregatorError> {
        let state = self.user_storage.get_user_state(user_id.clone()).await?;

        match state {
            Some(AggregatorUserState::SigningRound1 {
                tweak,
                message,
                signing_package,
                public_key_package,
            }) => {
                let tweaked_public_key_package = match tweak.clone() {
                    Some(tweak) => public_key_package.clone().tweak(Some(tweak.to_vec())),
                    None => public_key_package.clone(),
                };
                let mut signature_shares = BTreeMap::new();
                let mut join_handles = vec![];

                for (verifier_id, signer_client) in self.verifiers.clone() {
                    let request = SignRound2Request {
                        user_id: user_id.clone(),
                        session_id: session_id.clone(),
                        signing_package: signing_package.clone(),
                    };
                    let join_handle = async move { (verifier_id, signer_client.sign_round_2(request).await) };
                    join_handles.push(join_handle);
                }

                let join_handles = join_all(join_handles).await;

                for (verifier_id, response) in join_handles {
                    signature_shares.insert(verifier_id, response?.signature_share);
                }

                let signature =
                    frost_secp256k1_tr::aggregate(&signing_package, &signature_shares, &tweaked_public_key_package)
                        .map_err(|e| AggregatorError::Internal(format!("Signature aggregation failed: {:?}", e)))?;

                let is_valid = tweaked_public_key_package
                    .verifying_key()
                    .verify(message.as_slice(), &signature)
                    .is_ok();
                if !is_valid {
                    return Err(AggregatorError::Internal("Signature is not valid".to_string()));
                }

                self.user_storage
                    .set_user_state(
                        user_id.clone(),
                        AggregatorUserState::SigningRound2 {
                            tweak: tweak.map(|t| t.to_vec()),
                            message: message.to_vec(),
                            signature,
                            public_key_package,
                        },
                    )
                    .await?;

                Ok(())
            }
            _ => Err(AggregatorError::InvalidUserState(
                "User state is not DkgFinalized".to_string(),
            )),
        }
    }

    #[instrument(level = "debug", skip(self), ret)]
    pub async fn run_signing_flow(
        &self,
        user_id: String,
        message: &[u8],
        tweak: Option<&[u8]>,
    ) -> Result<Signature, AggregatorError> {
        let session_id = Uuid::new_v4().to_string();

        self.sign_round_1(user_id.clone(), session_id.clone(), message, tweak)
            .await?;
        self.sign_round_2(user_id.clone(), session_id.clone()).await?;

        let state = self.user_storage.get_user_state(user_id.clone()).await?;
        match state {
            Some(AggregatorUserState::SigningRound2 {
                tweak: _,
                message: _,
                signature,
                public_key_package,
            }) => {
                self.user_storage
                    .set_user_state(
                        user_id.clone(),
                        AggregatorUserState::DkgFinalized { public_key_package },
                    )
                    .await?;
                Ok(signature)
            }
            _ => Err(AggregatorError::InvalidUserState(
                "User state is not DkgFinalized".to_string(),
            )),
        }
    }
}
