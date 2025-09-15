use crate::signer_client::SignerClient;
use frost::aggregator::FrostAggregator;
use frost::traits::{AggregatorMusigIdStorage, AggregatorSignSessionStorage, SignerClient as SignerClientTrait};
use frost_secp256k1_tr::Identifier;
use gateway_config_parser::config::ServerConfig;
use std::collections::BTreeMap;
use std::sync::Arc;

pub fn create_aggregator_from_config(
    config: ServerConfig,
    musig_id_storage: Arc<dyn AggregatorMusigIdStorage>,
    sign_session_storage: Arc<dyn AggregatorSignSessionStorage>,
) -> FrostAggregator {
    let mut verifiers = BTreeMap::<Identifier, Arc<dyn SignerClientTrait>>::new();

    for verifier in config.verifiers.0 {
        let signer_client = SignerClient::new(verifier.clone());
        verifiers.insert(verifier.id.try_into().unwrap(), Arc::new(signer_client));
    }

    FrostAggregator::new(verifiers, musig_id_storage, sign_session_storage)
}
