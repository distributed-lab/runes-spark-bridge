use std::sync::Arc;

use frost_secp256k1_tr::{Identifier, keys::Tweak};

use rand_core::OsRng;

use crate::{config::SignerConfig, errors::SignerError, traits::*};

#[derive(Clone)]
pub struct FrostSigner {
    config: SignerConfig,
    user_storage: Arc<dyn SignerUserStorage>, // TODO: implement signer storage
    identifier: Identifier,
    session_storage: Arc<dyn SignerSessionStorage>,
}

impl FrostSigner {
    pub fn new(
        config: SignerConfig,
        user_storage: Arc<dyn SignerUserStorage>,
        session_storage: Arc<dyn SignerSessionStorage>,
    ) -> Self {
        Self {
            config: config.clone(),
            user_storage,
            identifier: config.identifier.try_into().unwrap(),
            session_storage,
        }
    }

    pub async fn dkg_round_1(&self, request: DkgRound1Request) -> Result<DkgRound1Response, SignerError> {
        let user_id = request.user_id;
        let state = self.user_storage.get_user_state(user_id.clone()).await?;

        match state {
            None => {
                let (secret_package, package) = frost_secp256k1_tr::keys::dkg::part1(
                    self.identifier,
                    self.config.total_participants,
                    self.config.threshold,
                    &mut OsRng,
                )
                .map_err(|e| SignerError::Internal(format!("DKG round1 failed: {e}")))?;

                self.user_storage
                    .set_user_state(
                        user_id.clone(),
                        SignerUserState::DkgRound1 {
                            round1_secret_package: secret_package,
                        },
                    )
                    .await?;

                Ok(DkgRound1Response {
                    round1_package: package,
                })
            }
            _ => Err(SignerError::InvalidUserState("User state is not None".to_string())),
        }
    }

    pub async fn dkg_round_2(&self, request: DkgRound2Request) -> Result<DkgRound2Response, SignerError> {
        let user_id = request.user_id;
        let state = self.user_storage.get_user_state(user_id.clone()).await?;

        match state {
            Some(SignerUserState::DkgRound1 { round1_secret_package }) => {
                let (secret_package, packages) =
                    frost_secp256k1_tr::keys::dkg::part2(round1_secret_package.clone(), &request.round1_packages)
                        .map_err(|e| SignerError::Internal(format!("DKG round2 failed: {e}")))?;

                self.user_storage
                    .set_user_state(
                        user_id.clone(),
                        SignerUserState::DkgRound2 {
                            round2_secret_package: secret_package,
                            round1_packages: request.round1_packages,
                        },
                    )
                    .await?;

                Ok(DkgRound2Response {
                    round2_packages: packages,
                })
            }
            _ => Err(SignerError::InvalidUserState("User state is not DkgRound1".to_string())),
        }
    }

    pub async fn dkg_finalize(&self, request: DkgFinalizeRequest) -> Result<DkgFinalizeResponse, SignerError> {
        let user_id = request.user_id;
        let state = self.user_storage.get_user_state(user_id.clone()).await?;

        match state {
            Some(SignerUserState::DkgRound2 {
                round2_secret_package,
                round1_packages,
            }) => {
                let (key_package, public_key_package) = frost_secp256k1_tr::keys::dkg::part3(
                    &round2_secret_package,
                    &round1_packages,
                    &request.round2_packages,
                )
                .map_err(|e| SignerError::Internal(format!("DKG finalize failed: {e}")))?;

                self.user_storage
                    .set_user_state(user_id.clone(), SignerUserState::DkgFinalized { key_package })
                    .await?;
                Ok(DkgFinalizeResponse { public_key_package })
            }
            _ => Err(SignerError::InvalidUserState("User state is not DkgRound2".to_string())),
        }
    }

    pub async fn sign_round_1(&self, request: SignRound1Request) -> Result<SignRound1Response, SignerError> {
        let user_id = request.user_id;
        let session_id = request.session_id.clone();
        let tweak = request.tweak;

        let state = self.user_storage.get_user_state(user_id.clone()).await?;

        match state {
            Some(SignerUserState::DkgFinalized { key_package }) => {
                let tweak_key_package = match tweak.clone() {
                    Some(tweak) => key_package.clone().tweak(Some(tweak.to_vec())),
                    None => key_package.clone(),
                };
                let (nonces, commitments) =
                    frost_secp256k1_tr::round1::commit(tweak_key_package.signing_share(), &mut OsRng);

                self.session_storage
                    .set_session_state(
                        user_id.clone(),
                        session_id.clone(),
                        SignerSessionState::SigningRound1 {
                            key_package,
                            tweak,
                            nonces,
                        },
                    )
                    .await?;
                Ok(SignRound1Response {
                    user_id,
                    session_id,
                    commitments,
                })
            }
            _ => Err(SignerError::InvalidUserState(
                "User state is not DkgFinalized".to_string(),
            )),
        }
    }

    pub async fn sign_round_2(&self, request: SignRound2Request) -> Result<SignRound2Response, SignerError> {
        let user_id = request.user_id.clone();
        let session_id = request.session_id.clone();

        let session_state = self
            .session_storage
            .get_session_state(user_id.clone(), session_id.clone())
            .await?;

        match session_state {
            Some(SignerSessionState::SigningRound1 {
                key_package,
                tweak,
                nonces,
            }) => {
                let tweak_key_package = match tweak.clone() {
                    Some(tweak) => key_package.clone().tweak(Some(tweak.to_vec())),
                    None => key_package.clone(),
                };
                let signature_share =
                    frost_secp256k1_tr::round2::sign(&request.signing_package, &nonces, &tweak_key_package)
                        .map_err(|e| SignerError::Internal(format!("Sign round2 failed: {e}")))?;

                self.session_storage
                    .set_session_state(
                        user_id,
                        session_id.clone(),
                        SignerSessionState::SigningRound2 {
                            key_package,
                            tweak,
                            signature_share,
                        },
                    )
                    .await?;

                Ok(SignRound2Response {
                    session_id,
                    signature_share,
                })
            }
            _ => Err(SignerError::InvalidUserState(
                "User state is not SigningRound1".to_string(),
            )),
        }
    }
}
