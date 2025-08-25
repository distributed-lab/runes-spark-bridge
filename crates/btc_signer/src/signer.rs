use async_trait::async_trait;
use rand::rngs::OsRng;
use secp256k1::{Keypair, Message, Secp256k1, XOnlyPublicKey};
use sha2::{Digest, Sha256};

// single body that want to sign smth

use crate::{
    errors::Result,
    traits::Signer,
    types::{BtcPublicKey, BtcSignature, PartialSignature, SignerConfig},
};

pub struct BtcSigner {
    config: SignerConfig,
    secp: Secp256k1<secp256k1::All>,
}

impl BtcSigner {
    pub fn new(id: String) -> Self { // cool keys
        let secp = Secp256k1::new();
        let keypair = Keypair::new(&secp, &mut OsRng);

        let config = SignerConfig { id, keypair };

        Self { config, secp }
    }

    // if we already have config
    pub fn from_config(config: SignerConfig) -> Self {
        let secp = Secp256k1::new();
        Self { config, secp }
    }

    // to restore
    pub fn from_private_key(id: String, private_key_hex: &str) -> Result<Self> {
        let secp = Secp256k1::new();
        let private_key_bytes = hex::decode(private_key_hex)?;

        let secret_key = secp256k1::SecretKey::from_slice(&private_key_bytes)?;
        let keypair = Keypair::from_secret_key(&secp, &secret_key);

        let config = SignerConfig { id, keypair };

        Ok(Self { config, secp })
    }

    pub fn keypair(&self) -> &Keypair {
        &self.config.keypair
    }

    pub fn x_only_public_key(&self) -> XOnlyPublicKey { // from doc, classic
        XOnlyPublicKey::from_keypair(&self.config.keypair).0
    }

    fn hash_message(&self, message: &[u8]) -> [u8; 32] { // maybe double sha256?
        let mut hasher = Sha256::new();
        hasher.update(message);
        hasher.finalize().into()
    }

    fn create_secp_message(&self, message: &[u8]) -> Result<Message> {
        let hash = self.hash_message(message);
        Ok(Message::from_slice(&hash)?)
    }

    //we always sign hash - not original message!!!
}

#[async_trait]
impl Signer for BtcSigner {
    async fn sign(&self, message: &[u8]) -> Result<BtcSignature> {
        let msg = self.create_secp_message(message)?;
        let signature = self.secp.sign_schnorr(&msg, &self.config.keypair);

        Ok(BtcSignature::new_schnorr(signature))
    }

    async fn get_public_key(&self) -> Result<BtcPublicKey> {
        let (xonly_pubkey, _parity) = XOnlyPublicKey::from_keypair(&self.config.keypair);
        Ok(BtcPublicKey::new_xonly(xonly_pubkey))
    }

    fn get_id(&self) -> &str {
        &self.config.id
    }

    async fn create_partial_signature(&self, message: &[u8]) -> Result<PartialSignature> {
        let msg = self.create_secp_message(message)?;
        let signature = self.secp.sign_schnorr(&msg, &self.config.keypair);
        let (xonly_pubkey, _parity) = XOnlyPublicKey::from_keypair(&self.config.keypair);

        Ok(PartialSignature {
            signature,
            signer_id: self.config.id.clone(),
            public_key: xonly_pubkey,
        })
    }
}

impl Clone for BtcSigner { // for safe copy of signer
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            secp: Secp256k1::new(), // stateless so its ok
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_signer_creation() {
        let signer = BtcSigner::new("test_signer".to_string());
        assert_eq!(signer.get_id(), "test_signer");
    }

    #[tokio::test]
    async fn test_sign_and_verify() {
        let signer = BtcSigner::new("test_signer".to_string());
        let message = b"Hello, Bitcoin!";

        let signature = signer.sign(message).await.unwrap();
        let pubkey = signer.get_public_key().await.unwrap();

        let secp = Secp256k1::new();
        let mut hasher = Sha256::new();
        hasher.update(message);
        let hash: [u8; 32] = hasher.finalize().into();
        let msg = Message::from_slice(&hash).unwrap();

        let schnorr_sig = signature.to_schnorr_signature().unwrap();
        let xonly_pubkey = pubkey.to_xonly_pubkey().unwrap();

        assert!(secp.verify_schnorr(&schnorr_sig, &msg, &xonly_pubkey).is_ok());
    }

    #[tokio::test]
    async fn test_partial_signature() {
        let signer = BtcSigner::new("test_signer".to_string());
        let message = b"Hello, Bitcoin!";

        let partial_sig = signer.create_partial_signature(message).await.unwrap();
        assert_eq!(partial_sig.signer_id, "test_signer");
    }
}