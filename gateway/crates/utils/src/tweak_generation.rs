use bitcoin::hashes::{FromSliceError, Hash, sha256};
use bitcoin::key::{Parity, Secp256k1, TapTweak, TweakedPublicKey};
use bitcoin::secp256k1::rand::Rng;
use bitcoin::secp256k1::{All, PublicKey};
use bitcoin::{TapNodeHash, secp256k1};
use frost_secp256k1_tr::VerifyingKey;
use frost_secp256k1_tr::keys::{PublicKeyPackage, Tweak};
use thiserror::Error;
use tracing::{instrument, trace};

pub type Nonce = [u8; 32];
pub type HashData = [u8; 32];
pub type RuneId = String;
pub struct TweakGenerator;

#[derive(Error, Debug)]
pub enum TweakGeneratorError {
    #[error("Failed to serialize into bytes verifying key, err: {0}")]
    VerifyingKeySerializeError(String),
    #[error("Occurred error in secp256k1, err: {0}")]
    Secp256k1Error(#[from] secp256k1::Error),
}

impl TweakGenerator {
    pub fn generate_nonce() -> Nonce {
        let mut rand = bitcoin::key::rand::thread_rng();
        let mut nonce: Nonce = [0; 32];
        rand.fill(&mut nonce);
        nonce
    }

    /// Function uses Bitcoin hash algorithm to hash values
    pub fn hash(data: impl AsRef<[u8]>) -> HashData {
        sha256::Hash::hash(data.as_ref()).to_byte_array()
    }

    #[instrument(level = "trace", skip(hashed_bytes), fields(hashed_bytes =% hex::encode(hashed_bytes)), err, ret)]
    pub fn tweak_btc_pubkey(
        secp: &Secp256k1<All>,
        pubkey: PublicKey,
        hashed_bytes: &HashData,
    ) -> Result<(TweakedPublicKey, Parity), FromSliceError> {
        let (tweaked_pubkey, parity) = pubkey
            .x_only_public_key()
            .0
            .tap_tweak(secp, Some(TapNodeHash::from_slice(hashed_bytes)).transpose()?);
        Ok((tweaked_pubkey, parity))
    }

    #[instrument(level = "trace", skip(hashed_bytes), fields(hashed_byted =% hex::encode(hashed_bytes)), ret)]
    pub fn tweak_pubkey_package(public_key_package: PublicKeyPackage, hashed_bytes: &HashData) -> PublicKeyPackage {
        public_key_package.clone().tweak(Some(hashed_bytes))
    }

    #[instrument(level = "trace", err, ret)]
    pub fn tweaked_verifying_key_to_tweaked_pubkey(
        verifying_key: &VerifyingKey,
    ) -> Result<(TweakedPublicKey, Parity), TweakGeneratorError> {
        let btc_pubkey = PublicKey::from_slice(
            &verifying_key
                .serialize()
                .map_err(|e| TweakGeneratorError::VerifyingKeySerializeError(e.to_string()))?,
        )?;
        let (tweaked_x, parity) = btc_pubkey.x_only_public_key();
        Ok((TweakedPublicKey::dangerous_assume_tweaked(tweaked_x), parity))
    }

    /// Serialized data into vec of bytes in standardised way both for Spark & Runes
    #[instrument(level = "trace", skip(nonce), fields(nonce =% hex::encode(nonce)), ret)]
    pub fn serialize_tweak_data(user_pubkey: secp256k1::PublicKey, rune_id: RuneId, nonce: Nonce) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(user_pubkey.to_string().as_bytes());
        data.extend_from_slice(rune_id.as_bytes());
        data.extend_from_slice(&nonce);
        trace!(
            "[tweaking] Tweak data to generate tweaked pubkey: {}",
            hex::encode(&data)
        );
        data
    }
}
