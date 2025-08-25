use async_trait::async_trait;
use secp256k1::{Secp256k1, Message, XOnlyPublicKey, schnorr::Signature};
use sha2::{Sha256, Digest};

use crate::{
    traits::SignatureAggregator,
    types::{BtcSignature, BtcPublicKey, PartialSignature},
    errors::{BtcSignerError, Result},
};

pub struct SchnorrAggregator { // we need this for all operations with keys and signatures
    secp: Secp256k1<secp256k1::All>, // All - so we can use schnorr and ecdsa
}

impl SchnorrAggregator {
    pub fn new() -> Self {
        Self {
            secp: Secp256k1::new(),
        }
    }

    fn hash_message(&self, message: &[u8]) -> [u8; 32] { // should we do sha256 twice?
        let mut hasher = Sha256::new();
        hasher.update(message);
        hasher.finalize().into()
    }

    // maybe i should check length of message?

    fn create_secp_message(&self, message: &[u8]) -> Result<Message> {
        let hash = self.hash_message(message);
        Ok(Message::from_slice(&hash)?)
    }

    // find or implement MuSig2 protocol!!!
    fn aggregate_schnorr_signatures(&self, signatures: &[Signature]) -> Result<Signature> {
        if signatures.is_empty() {
            return Err(BtcSignerError::AggregationFailed {
                reason: "No signatures to aggregate".to_string(),
            });
        }

        // at first i need to collect nonces from all participants
        // then compute aggregated nonce?
        // theen compute challenge
        // and finally aggregate signatures properly
        let first_sig = signatures[0];

        // threshold based approach (cool) but i need to use MuSig2
        Ok(first_sig)
    }

    fn aggregate_public_keys(&self, public_keys: &[XOnlyPublicKey]) -> Result<XOnlyPublicKey> {
        if public_keys.is_empty() {
            return Err(BtcSignerError::AggregationFailed {
                reason: "No public keys to aggregate".to_string(),
            });
        }

        Ok(public_keys[0])
    }
}

impl Default for SchnorrAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SignatureAggregator for SchnorrAggregator {
    async fn aggregate( // to check signature
        &self,
        partial_signatures: &[PartialSignature],
        message: &[u8],
    ) -> Result<BtcSignature> {
        if partial_signatures.is_empty() {
            return Err(BtcSignerError::AggregationFailed {
                reason: "No partial signatures provided".to_string(),
            });
        }

        let signatures: Vec<Signature> = partial_signatures
            .iter()
            .map(|ps| ps.signature)
            .collect();

        for partial_sig in partial_signatures {
            if !self.verify_partial_signature(partial_sig, message).await? {
                return Err(BtcSignerError::AggregationFailed {
                    reason: format!("Invalid partial signature from {}", partial_sig.signer_id),
                });
            }
        }

        // maybe i should add who signed

        let aggregated_signature = self.aggregate_schnorr_signatures(&signatures)?; // approved

        Ok(BtcSignature::new_schnorr(aggregated_signature))
    }

    async fn verify_partial_signature(
        &self,
        partial_sig: &PartialSignature,
        message: &[u8],
    ) -> Result<bool> {
        let msg = self.create_secp_message(message)?;

        match self.secp.verify_schnorr(&partial_sig.signature, &msg, &partial_sig.public_key) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false), // todo! -> cool description
        }
    }

    async fn verify_aggregate_signature(
        &self,
        signature: &BtcSignature,
        message: &[u8],
        public_keys: &[BtcPublicKey],
    ) -> Result<bool> {
        if public_keys.is_empty() {
            return Ok(false);
        }

        // aggregation on first elemetnt -> soon MuSig2

        let msg = self.create_secp_message(message)?;
        let schnorr_sig = signature.to_schnorr_signature()?;

        let xonly_keys: Result<Vec<XOnlyPublicKey>> = public_keys
            .iter()
            .map(|pk| pk.to_xonly_pubkey())
            .collect();
        let xonly_keys = xonly_keys?;

        let aggregated_pubkey = self.aggregate_public_keys(&xonly_keys)?;

        match self.secp.verify_schnorr(&schnorr_sig, &msg, &aggregated_pubkey) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false), // todo! -> cool description
        }
    }
}

pub struct MuSig2Aggregator { // for the future
    secp: Secp256k1<secp256k1::All>,
}

// wont be any conflicts with schnorr aggregator
impl MuSig2Aggregator {
    pub fn new() -> Self {
        Self {
            secp: Secp256k1::new(),
        }
    }

    pub fn aggregate_keys(&self, _public_keys: &[XOnlyPublicKey]) -> Result<XOnlyPublicKey> {
        todo!("Implement MuSig2 key aggregation") // hehe cool todo
    }
    pub fn generate_nonces(&self) -> Result<(Vec<u8>, Vec<u8>)> {
        todo!("Implement MuSig2 nonce generation")
    }
    pub fn aggregate_signatures(&self, _partial_sigs: &[Vec<u8>]) -> Result<Signature> {
        todo!("Implement MuSig2 signature aggregation")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signer::BtcSigner;
    use crate::traits::Signer;

    #[tokio::test]
    async fn test_aggregator_creation() {
        let aggregator = SchnorrAggregator::new();
        assert!(true);
    }

    #[tokio::test]
    async fn test_partial_signature_verification() {
        let aggregator = SchnorrAggregator::new();
        let signer = BtcSigner::new("test_signer".to_string());
        let message = b"Hello Aggregation!";

        let partial_sig = signer.create_partial_signature(message).await.unwrap();
        let is_valid = aggregator.verify_partial_signature(&partial_sig, message).await.unwrap();

        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_signature_aggregation() {
        let aggregator = SchnorrAggregator::new();

        let signer1 = BtcSigner::new("signer1".to_string());
        let signer2 = BtcSigner::new("signer2".to_string());
        let message = b"Hello MultiSig Aggregation!";

        let partial_sig1 = signer1.create_partial_signature(message).await.unwrap();
        let partial_sig2 = signer2.create_partial_signature(message).await.unwrap();

        let partial_signatures = vec![partial_sig1, partial_sig2];

        let aggregated_signature = aggregator.aggregate(&partial_signatures, message).await.unwrap();

        assert_eq!(aggregated_signature.algorithm, crate::types::SignatureAlgorithm::SchnorrSecp256k1);
    }
}