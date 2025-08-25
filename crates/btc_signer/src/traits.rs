use async_trait::async_trait;
use crate::{BtcSignature, BtcPublicKey, PartialSignature, Result};

// oop in rust :))
// like interfaces if Java

#[async_trait]
pub trait Signer: Send + Sync {
    async fn sign(&self, message: &[u8]) -> Result<BtcSignature>;
    async fn get_public_key(&self) -> Result<BtcPublicKey>;
    fn get_id(&self) -> &str;
    async fn create_partial_signature(&self, message: &[u8]) -> Result<PartialSignature>;
}

#[async_trait]
pub trait MultiSigner: Send + Sync {
    async fn add_signer(&mut self, signer: Box<dyn Signer>) -> Result<()>;

    async fn remove_signer(&mut self, signer_id: &str) -> Result<()>;

    async fn get_signer(&self, signer_id: &str) -> Result<&dyn Signer>;

    fn list_signer_ids(&self) -> Vec<String>;

    async fn create_multi_sig_session(
        &self,
        message: &[u8],
        signer_ids: &[String],
        threshold: u32,
    ) -> Result<String>;

    async fn add_partial_signature(
        &mut self,
        session_id: &str,
        signer_id: &str,
        message: &[u8],
    ) -> Result<PartialSignature>;

    async fn aggregate_signatures(&mut self, session_id: &str) -> Result<BtcSignature>;

    fn signer_count(&self) -> usize;
}

#[async_trait]
pub trait SignatureAggregator: Send + Sync {
    async fn aggregate(
        &self,
        partial_signatures: &[PartialSignature],
        message: &[u8],
    ) -> Result<BtcSignature>;

    async fn verify_partial_signature(
        &self,
        partial_sig: &PartialSignature,
        message: &[u8],
    ) -> Result<bool>;

    async fn verify_aggregate_signature(
        &self,
        signature: &BtcSignature,
        message: &[u8],
        public_keys: &[BtcPublicKey],
    ) -> Result<bool>;
}