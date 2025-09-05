mod utils;

mod test_mocked_verifier_db_usage {
    pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

    use crate::utils::TEST_LOGGER;
    use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
    use frost::config::{AggregatorConfig, SignerConfig};
    use frost::mocks::{MockAggregatorUserStorage, MockSignerClient, MockSignerSessionStorage, MockSignerUserStorage};
    use frost::{aggregator::FrostAggregator, config::*, mocks::*, signer::FrostSigner, traits::SignerClient};
    use frost_secp256k1_tr::Identifier;
    use persistent_storage::init::PostgresRepo;
    use sqlx::{Pool, Postgres};
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use verifier_local_db_store::LocalDbStore;

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn test_user_state_setting(pool: Pool<Postgres>) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;
        let db_entity = PostgresRepo { pool };

        let local_storage: Arc<LocalDbStore> = LocalDbStore(db_entity.pool).into_shared();
        let _ = test_parallel_signing_sessions_via_aggregator(local_storage).await?;

        Ok(())
    }

    async fn test_parallel_signing_sessions_via_aggregator(local_db_store: Arc<LocalDbStore>) -> anyhow::Result<()> {
        let verifiers_map = init_objects(local_db_store)?;

        let aggregator = FrostAggregator::new(
            AggregatorConfig {
                threshold: 2,
                total_participants: 3,
                verifier_identifiers: vec![1, 2, 3],
            },
            verifiers_map,
            Arc::new(MockAggregatorUserStorage::new()),
        );

        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(&[1u8; 32]).unwrap();
        let user_id = PublicKey::from_secret_key(&secp, &secret_key);
        let msg_a = b"parallel message A".to_vec();
        let msg_b = b"parallel message B".to_vec();
        let tweak = None::<&[u8]>;

        let public_key_package = aggregator.run_dkg_flow(user_id.clone()).await.unwrap();

        let (sig_res_a, sig_res_b) = tokio::join!(
            aggregator.run_signing_flow(user_id.clone(), msg_a.as_slice(), tweak),
            aggregator.run_signing_flow(user_id.clone(), msg_b.as_slice(), tweak),
        );

        let signature_a = sig_res_a?;
        let signature_b = sig_res_b?;

        let pk = public_key_package.clone();
        pk.verifying_key()
            .verify(msg_a.as_slice(), &signature_a)
            .expect("signature A must be valid");

        pk.verifying_key()
            .verify(msg_b.as_slice(), &signature_b)
            .expect("signature B must be valid");

        assert_ne!(
            signature_a, signature_b,
            "signatures for different messages should differ"
        );
        Ok(())
    }

    fn create_mock_signer(identifier: u16, real_local_db_store: Option<Arc<LocalDbStore>>) -> FrostSigner {
        match real_local_db_store {
            Some(local_db_store) => FrostSigner::new(
                SignerConfig {
                    identifier,
                    threshold: 2,
                    total_participants: 3,
                },
                local_db_store.clone(),
                local_db_store,
            ),
            None => FrostSigner::new(
                SignerConfig {
                    identifier,
                    threshold: 2,
                    total_participants: 3,
                },
                Arc::new(MockSignerUserStorage::new()),
                Arc::new(MockSignerSessionStorage::new()),
            ),
        }
    }

    fn init_objects(local_db_store: Arc<LocalDbStore>) -> anyhow::Result<BTreeMap<Identifier, Arc<dyn SignerClient>>> {
        let signer1 = create_mock_signer(1, Some(local_db_store.clone()));
        let signer2 = create_mock_signer(2, None);
        let signer3 = create_mock_signer(3, None);

        let mock_signer_client1 = MockSignerClient::new(signer1);
        let mock_signer_client2 = MockSignerClient::new(signer2);
        let mock_signer_client3 = MockSignerClient::new(signer3);

        let identifier_1: Identifier = 1.try_into()?;
        let identifier_2: Identifier = 2.try_into()?;
        let identifier_3: Identifier = 3.try_into()?;

        Ok(BTreeMap::from([
            (identifier_1, Arc::new(mock_signer_client1) as Arc<dyn SignerClient>),
            (identifier_2, Arc::new(mock_signer_client2) as Arc<dyn SignerClient>),
            (identifier_3, Arc::new(mock_signer_client3) as Arc<dyn SignerClient>),
        ]))
    }
}
