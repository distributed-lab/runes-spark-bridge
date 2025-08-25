use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// there we basically take hard stuff - use it to create easy stuff - so in the future use it

use crate::{
    traits::{Signer, MultiSigner, SignatureAggregator},
    types::{BtcSignature, PartialSignature, MultiSigSession, SessionState, MultiSignerConfig},
    aggregator::SchnorrAggregator,
    errors::{BtcSignerError, Result},
};

pub struct BtcMultiSigner {
    signers: HashMap<String, Arc<dyn Signer>>,
    sessions: Arc<RwLock<HashMap<String, MultiSigSession>>>, // for async
    aggregator: SchnorrAggregator,
    default_threshold: u32,
}

impl BtcMultiSigner {
    pub fn new() -> Self {
        Self { // empty
            signers: HashMap::new(),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            aggregator: SchnorrAggregator::new(),
            default_threshold: 1,
        }
    }

    // creating signers
    pub fn from_config(config: MultiSignerConfig) -> Result<Self> {
        let mut multi_signer = Self::new();
        multi_signer.default_threshold = config.default_threshold;

        for signer_config in config.signers {
            let signer = crate::signer::BtcSigner::from_config(signer_config);
            multi_signer.signers.insert(
                signer.get_id().to_string(),
                Arc::new(signer),
            );
        }

        Ok(multi_signer)
    }

    // if we have some data
    pub fn with_signers_and_threshold(
        signers: Vec<Box<dyn Signer>>,
        threshold: u32,
    ) -> Result<Self> {
        let mut multi_signer = Self::new();
        multi_signer.default_threshold = threshold;

        for signer in signers {
            let id = signer.get_id().to_string();
            multi_signer.signers.insert(id, Arc::from(signer));
        }

        if threshold > multi_signer.signers.len() as u32 {
            return Err(BtcSignerError::InvalidThreshold {
                threshold,
                total: multi_signer.signers.len() as u32,
            });
        }

        Ok(multi_signer)
    }

    async fn get_session(&self, session_id: &str) -> Result<MultiSigSession> {
        let sessions = self.sessions.read().await;
        sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| BtcSignerError::SessionNotFound {
                id: session_id.to_string(),
            })
    }

    async fn update_session(&self, session: MultiSigSession) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.id.clone(), session);
        Ok(())
    }

    fn generate_session_id() -> String {
        Uuid::new_v4().to_string()
    }

    // if too old - 1 hour
    pub async fn cleanup_expired_sessions(&self) -> Result<usize> {
        let mut sessions = self.sessions.write().await;
        let now = std::time::SystemTime::now();
        let one_hour = std::time::Duration::from_secs(3600);

        let expired_sessions: Vec<String> = sessions
            .iter()
            .filter_map(|(id, session)| {
                if now.duration_since(session.created_at).unwrap_or_default() > one_hour {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect();

        let count = expired_sessions.len();
        for session_id in expired_sessions {
            sessions.remove(&session_id);
        }

        Ok(count)
    }
}

impl Default for BtcMultiSigner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MultiSigner for BtcMultiSigner {
    async fn add_signer(&mut self, signer: Box<dyn Signer>) -> Result<()> {
        let id = signer.get_id().to_string();

        if self.signers.contains_key(&id) {
            return Err(BtcSignerError::DuplicateSigner { id });
        }

        self.signers.insert(id, Arc::from(signer));
        Ok(())
    }

    // finally!!!
    async fn remove_signer(&mut self, signer_id: &str) -> Result<()> {
        self.signers
            .remove(signer_id)
            .ok_or_else(|| BtcSignerError::SignerNotFound {
                id: signer_id.to_string(),
            })?;

        Ok(())
    }

    async fn get_signer(&self, signer_id: &str) -> Result<&dyn Signer> {
        let signer = self
            .signers
            .get(signer_id)
            .ok_or_else(|| BtcSignerError::SignerNotFound {
                id: signer_id.to_string(),
            })?;

        Ok(signer.as_ref())
    }

    fn list_signer_ids(&self) -> Vec<String> {
        self.signers.keys().cloned().collect()
    }

    async fn create_multi_sig_session(
        &self,
        message: &[u8],
        signer_ids: &[String],
        threshold: u32,
    ) -> Result<String> {
        if threshold > signer_ids.len() as u32 {
            return Err(BtcSignerError::InvalidThreshold {
                threshold,
                total: signer_ids.len() as u32,
            });
        }

        if threshold == 0 {
            return Err(BtcSignerError::InvalidThreshold {
                threshold: 0,
                total: signer_ids.len() as u32,
            });
        }

        for signer_id in signer_ids {
            if !self.signers.contains_key(signer_id) {
                return Err(BtcSignerError::SignerNotFound {
                    id: signer_id.clone(),
                });
            }
        }

        let session_id = Self::generate_session_id();
        let session = MultiSigSession::new(
            session_id.clone(),
            message.to_vec(),
            threshold,
            signer_ids.to_vec(),
        );

        self.update_session(session).await?;
        Ok(session_id)
    }

    async fn add_partial_signature(
        &mut self,
        session_id: &str,
        signer_id: &str,
        message: &[u8],
    ) -> Result<PartialSignature> {
        let signer = self.signers
            .get(signer_id)
            .ok_or_else(|| BtcSignerError::SignerNotFound {
                id: signer_id.to_string(),
            })?
            .clone();

        let mut session = self.get_session(session_id).await?;

        if !matches!(session.state, SessionState::WaitingForSignatures) {
            return Err(BtcSignerError::InvalidSessionState {
                state: format!("{:?}", session.state),
            });
        }

        if session.message != message {
            return Err(BtcSignerError::InvalidMessage);
        }

        let partial_signature = signer.create_partial_signature(message).await?;

        // !!!
        if !self.aggregator.verify_partial_signature(&partial_signature, message).await? {
            return Err(BtcSignerError::InvalidSignature);
        }

        session.add_partial_signature(partial_signature.clone())?;

        self.update_session(session).await?;

        Ok(partial_signature)
    }

    async fn aggregate_signatures(&mut self, session_id: &str) -> Result<BtcSignature> {
        let mut session = self.get_session(session_id).await?;

        if !session.is_ready_for_aggregation() {
            return Err(BtcSignerError::InsufficientSignatures {
                got: session.partial_signatures.len(),
                need: session.threshold as usize,
            });
        }

        let partial_signatures: Vec<PartialSignature> = session
            .partial_signatures
            .values()
            .cloned()
            .collect();

        let final_signature = self.aggregator
            .aggregate(&partial_signatures, &session.message)
            .await?;

        session.state = SessionState::Completed;
        self.update_session(session).await?;

        Ok(final_signature)
    }

    fn signer_count(&self) -> usize {
        self.signers.len()
    }
}

// add new tests - threshold/signer/message

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signer::BtcSigner;

    #[tokio::test]
    async fn test_multi_signer_creation() {
        let multi_signer = BtcMultiSigner::new();
        assert_eq!(multi_signer.signer_count(), 0);
    }

    #[tokio::test]
    async fn test_add_remove_signer() {
        let mut multi_signer = BtcMultiSigner::new();
        let signer = Box::new(BtcSigner::new("test_signer".to_string()));

        multi_signer.add_signer(signer).await.unwrap();
        assert_eq!(multi_signer.signer_count(), 1);
        assert!(multi_signer.list_signer_ids().contains(&"test_signer".to_string()));

        multi_signer.remove_signer("test_signer").await.unwrap();
        assert_eq!(multi_signer.signer_count(), 0);
    }

    #[tokio::test]
    async fn test_multi_sig_session() {
        let mut multi_signer = BtcMultiSigner::new();

        for i in 0..3 {
            let signer = Box::new(BtcSigner::new(format!("signer_{}", i)));
            multi_signer.add_signer(signer).await.unwrap();
        }

        let message = b"Hello, Multi-Sig!";
        let signer_ids = vec![
            "signer_0".to_string(),
            "signer_1".to_string(),
            "signer_2".to_string(),
        ];

        let session_id = multi_signer
            .create_multi_sig_session(message, &signer_ids, 2)
            .await
            .unwrap();

        let _partial_sig1 = multi_signer
            .add_partial_signature(&session_id, "signer_0", message)
            .await
            .unwrap();

        let _partial_sig2 = multi_signer
            .add_partial_signature(&session_id, "signer_1", message)
            .await
            .unwrap();

        let _final_signature = multi_signer
            .aggregate_signatures(&session_id)
            .await
            .unwrap();
    }
}