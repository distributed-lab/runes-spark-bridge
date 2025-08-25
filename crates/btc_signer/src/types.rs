use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use secp256k1::{schnorr, XOnlyPublicKey, Keypair};

// oop in rust
// like class

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BtcSignature {
    pub data: Vec<u8>,
    pub algorithm: SignatureAlgorithm,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BtcPublicKey {
    pub data: Vec<u8>,
    pub format: PublicKeyFormat,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SignatureAlgorithm {
    SchnorrSecp256k1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PublicKeyFormat {
    XOnlyCompressed,
    Compressed,
}

// #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// pub struct PartialSignature {
//     pub signature: schnorr::Signature,
//     pub signer_id: String,
//     pub public_key: XOnlyPublicKey,
// }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartialSignature {
    #[serde(serialize_with = "serialize_schnorr_signature", deserialize_with = "deserialize_schnorr_signature")]
    pub signature: schnorr::Signature,

    pub signer_id: String,

    #[serde(serialize_with = "serialize_xonly", deserialize_with = "deserialize_xonly")]
    pub public_key: XOnlyPublicKey,
}

// Serialize/Deserialize Signature becuase it wont work without this funk
fn serialize_schnorr_signature<S>(
    sig: &schnorr::Signature,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_bytes(sig.as_ref())
}

fn deserialize_schnorr_signature<'de, D>(deserializer: D) -> Result<schnorr::Signature, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let bytes: &[u8] = serde::Deserialize::deserialize(deserializer)?;
    schnorr::Signature::from_slice(bytes).map_err(serde::de::Error::custom)
}

// Serialize/Deserialize XOnlyPublicKey because it wont work without this funk
fn serialize_xonly<S>(key: &XOnlyPublicKey, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_bytes(&key.serialize())
}

fn deserialize_xonly<'de, D>(deserializer: D) -> Result<XOnlyPublicKey, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let bytes: &[u8] = serde::Deserialize::deserialize(deserializer)?;
    XOnlyPublicKey::from_slice(bytes).map_err(serde::de::Error::custom)
}

#[derive(Debug, Clone)]
pub struct MultiSigSession {
    pub id: String,
    pub message: Vec<u8>,
    pub threshold: u32,
    pub participants: Vec<String>,
    pub partial_signatures: HashMap<String, PartialSignature>,
    pub state: SessionState,
    pub created_at: std::time::SystemTime,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    WaitingForSignatures,
    ReadyForAggregation,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct SignerConfig {
    pub id: String,
    pub keypair: Keypair,
}

#[derive(Debug, Clone)]
pub struct MultiSignerConfig {
    pub signers: Vec<SignerConfig>,
    pub default_threshold: u32,
}

impl BtcSignature {
    pub fn new_schnorr(signature: schnorr::Signature) -> Self {
        Self {
            data: signature.as_ref().to_vec(),
            algorithm: SignatureAlgorithm::SchnorrSecp256k1,
        }
    }

    pub fn to_schnorr_signature(&self) -> crate::Result<schnorr::Signature> {
        if self.algorithm != SignatureAlgorithm::SchnorrSecp256k1 {
            return Err(crate::BtcSignerError::InvalidSignature);
        }

        let sig_bytes: [u8; 64] = self.data.as_slice().try_into()
            .map_err(|_| crate::BtcSignerError::InvalidSignature)?;

        Ok(schnorr::Signature::from_slice(&sig_bytes)?)
    }
}

impl BtcPublicKey {
    pub fn new_xonly(pubkey: XOnlyPublicKey) -> Self {
        Self {
            data: pubkey.serialize().to_vec(),
            format: PublicKeyFormat::XOnlyCompressed,
        }
    }

    pub fn to_xonly_pubkey(&self) -> crate::Result<XOnlyPublicKey> {
        if self.format != PublicKeyFormat::XOnlyCompressed {
            return Err(crate::BtcSignerError::InvalidPublicKey);
        }

        let key_bytes: [u8; 32] = self.data.as_slice().try_into()
            .map_err(|_| crate::BtcSignerError::InvalidPublicKey)?;

        Ok(XOnlyPublicKey::from_slice(&key_bytes)?)
    }
}

impl MultiSigSession {
    pub fn new(
        id: String,
        message: Vec<u8>,
        threshold: u32,
        participants: Vec<String>,
    ) -> Self {
        Self {
            id,
            message,
            threshold,
            participants,
            partial_signatures: HashMap::new(),
            state: SessionState::WaitingForSignatures,
            created_at: std::time::SystemTime::now(),
        }
    }

    pub fn add_partial_signature(&mut self, partial_sig: PartialSignature) -> crate::Result<()> {
        if !self.participants.contains(&partial_sig.signer_id) {
            return Err(crate::BtcSignerError::SignerNotFound {
                id: partial_sig.signer_id.clone(),
            });
        }

        self.partial_signatures.insert(partial_sig.signer_id.clone(), partial_sig);

        if self.partial_signatures.len() >= self.threshold as usize {
            self.state = SessionState::ReadyForAggregation;
        }

        Ok(())
    }

    pub fn is_ready_for_aggregation(&self) -> bool {
        matches!(self.state, SessionState::ReadyForAggregation)
    }
}