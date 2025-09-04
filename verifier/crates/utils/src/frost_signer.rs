use frost::mocks::MockSignerUserStorage;
use frost::signer::FrostSigner;
use std::sync::Arc;
use verifier_config_parser::config::SignerConfig;

pub fn create_frost_signer(signer_config: SignerConfig) -> FrostSigner {
    FrostSigner::new(
        Arc::new(MockSignerUserStorage::new()),
        signer_config.identifier,
        signer_config.total_participants,
        signer_config.threshold,
    )
}
