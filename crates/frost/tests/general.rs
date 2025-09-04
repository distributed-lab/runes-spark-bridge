use std::{collections::BTreeMap, sync::Arc};

use frost::traits::{DkgRound1Request, SignRound1Request, SignRound2Request};
use frost::{aggregator::FrostAggregator, config::*, mocks::*, signer::FrostSigner, traits::SignerClient};
use frost_secp256k1_tr::{Identifier, keys::Tweak};

fn create_signer(identifier: u16) -> FrostSigner {
    FrostSigner::new(
        SignerConfig {
            identifier,
            threshold: 2,
            total_participants: 3,
        },
        Arc::new(MockSignerUserStorage::new()),
        Arc::new(MockSignerSessionStorage::new()),
    )
}

fn helper_1() -> BTreeMap<Identifier, Arc<dyn SignerClient>> {
    let signer1 = create_signer(1);
    let signer2 = create_signer(2);
    let signer3 = create_signer(3);

    let mock_signer_client1 = MockSignerClient::new(signer1);
    let mock_signer_client2 = MockSignerClient::new(signer2);
    let mock_signer_client3 = MockSignerClient::new(signer3);

    let identifier_1: Identifier = 1.try_into().unwrap();
    let identifier_2: Identifier = 2.try_into().unwrap();
    let identifier_3: Identifier = 3.try_into().unwrap();

    BTreeMap::from([
        (identifier_1, Arc::new(mock_signer_client1) as Arc<dyn SignerClient>),
        (identifier_2, Arc::new(mock_signer_client2) as Arc<dyn SignerClient>),
        (identifier_3, Arc::new(mock_signer_client3) as Arc<dyn SignerClient>),
    ])
}

#[tokio::test]
async fn test_aggregator_signer_integration() {
    let verifiers_map = helper_1();

    let aggregator = FrostAggregator::new(
        AggregatorConfig {
            threshold: 2,
            total_participants: 3,
            verifier_identifiers: vec![1, 2, 3],
        },
        verifiers_map,
        Arc::new(MockAggregatorUserStorage::new()),
    );

    let user_id = "test_user";
    let message = b"test_message";
    // let tweak = Some(b"test_tweak".as_slice());
    let tweak = None;

    let public_key_package = aggregator.run_dkg_flow(user_id.to_string()).await.unwrap();
    let signature = aggregator
        .run_signing_flow(user_id.to_string(), message, tweak)
        .await
        .unwrap();

    let tweaked_public_key_package = match tweak.clone() {
        Some(tweak) => public_key_package.clone().tweak(Some(tweak.to_vec())),
        None => public_key_package.clone(),
    };
    tweaked_public_key_package
        .verifying_key()
        .verify(message, &signature)
        .unwrap();
}

fn create_signer_with_stores(identifier: u16) -> FrostSigner {
    FrostSigner::new(
        SignerConfig {
            identifier,
            threshold: 2,
            total_participants: 3,
        },
        Arc::new(MockSignerUserStorage::new()),
        Arc::new(MockSignerSessionStorage::new()),
    )
}
#[tokio::test]
async fn test_parallel_signing_sessions_via_aggregator() {
    let verifiers_map = helper_1();

    let aggregator = FrostAggregator::new(
        AggregatorConfig {
            threshold: 2,
            total_participants: 3,
            verifier_identifiers: vec![1, 2, 3],
        },
        verifiers_map,
        Arc::new(MockAggregatorUserStorage::new()),
    );

    let user_id = "test_user".to_string();
    let msg_a = b"parallel message A".to_vec();
    let msg_b = b"parallel message B".to_vec();
    let tweak = None::<&[u8]>;

    let public_key_package = aggregator.run_dkg_flow(user_id.clone()).await.unwrap();

    let (sig_res_a, sig_res_b) = tokio::join!(
        aggregator.run_signing_flow(user_id.clone(), msg_a.as_slice(), tweak),
        aggregator.run_signing_flow(user_id.clone(), msg_b.as_slice(), tweak),
    );

    let signature_a = sig_res_a.unwrap();
    let signature_b = sig_res_b.unwrap();

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
}

fn create_signer_with_stores_2(identifier: u16) -> (FrostSigner, Arc<MockSignerSessionStorage>) {
    let user_storage = Arc::new(MockSignerUserStorage::new());
    let session_storage = Arc::new(MockSignerSessionStorage::new());

    let signer = FrostSigner::new(
        SignerConfig {
            identifier,
            threshold: 2,
            total_participants: 3,
        },
        user_storage,
        session_storage.clone(),
    );

    (signer, session_storage)
}

fn helper_2() -> BTreeMap<Identifier, Arc<dyn SignerClient>> {
    let (signer1, session1) = create_signer_with_stores_2(1);
    let (signer2, session2) = create_signer_with_stores_2(2);
    let (signer3, session3) = create_signer_with_stores_2(3);

    let mock_signer_client1 = MockSignerClient::new(signer1.clone());
    let mock_signer_client2 = MockSignerClient::new(signer2.clone());
    let mock_signer_client3 = MockSignerClient::new(signer3.clone());

    let identifier_1: Identifier = 1.try_into().unwrap();
    let identifier_2: Identifier = 2.try_into().unwrap();
    let identifier_3: Identifier = 3.try_into().unwrap();

    BTreeMap::from([
        (identifier_1, Arc::new(mock_signer_client1) as Arc<dyn SignerClient>),
        (identifier_2, Arc::new(mock_signer_client2) as Arc<dyn SignerClient>),
        (identifier_3, Arc::new(mock_signer_client3) as Arc<dyn SignerClient>),
    ])
}

#[tokio::test]
async fn test_session_storage_in_signing_flow() {
    let (signer1, session1) = create_signer_with_stores_2(1);
    let (signer2, session2) = create_signer_with_stores_2(2);
    let (signer3, session3) = create_signer_with_stores_2(3);

    let mock_signer_client1 = MockSignerClient::new(signer1.clone());
    let mock_signer_client2 = MockSignerClient::new(signer2.clone());
    let mock_signer_client3 = MockSignerClient::new(signer3.clone());

    let identifier_1: Identifier = 1.try_into().unwrap();
    let identifier_2: Identifier = 2.try_into().unwrap();
    let identifier_3: Identifier = 3.try_into().unwrap();
    let verifiers_map = BTreeMap::from([
        (identifier_1, Arc::new(mock_signer_client1) as Arc<dyn SignerClient>),
        (identifier_2, Arc::new(mock_signer_client2) as Arc<dyn SignerClient>),
        (identifier_3, Arc::new(mock_signer_client3) as Arc<dyn SignerClient>),
    ]);

    let aggregator = FrostAggregator::new(
        AggregatorConfig {
            threshold: 2,
            total_participants: 3,
            verifier_identifiers: vec![1, 2, 3],
        },
        verifiers_map,
        Arc::new(MockAggregatorUserStorage::new()),
    );

    let user_id = "test_user".to_string();
    let message = b"hello_session".to_vec();
    let session_id = "session_1".to_string();

    let public_key_package = aggregator.run_dkg_flow(user_id.clone()).await.unwrap();

    let signature = aggregator
        .run_signing_flow(user_id.clone(), &message, None)
        .await
        .unwrap();

    let sign1_request = SignRound1Request {
        user_id: user_id.clone(),
        session_id: "test_session".to_string(),
        tweak: None,
    };
    let sign1_response = signer1.clone().sign_round_1(sign1_request).await.unwrap();
    let sign2_request = SignRound1Request {
        user_id: user_id.clone(),
        session_id: "test_session".to_string(),
        tweak: None,
    };
    let sign2_response = signer2.clone().sign_round_1(sign2_request).await.unwrap();

    let sign3_request = SignRound1Request {
        user_id: user_id.clone(),
        session_id: "test_session".to_string(),
        tweak: None,
    };
    let sign3_response = signer3.clone().sign_round_1(sign3_request).await.unwrap();

    assert!(
        session1.has_session(&user_id, &sign1_response.session_id).await,
        "signer1 session storage must have the session"
    );
    assert!(
        session2.has_session(&user_id, &sign2_response.session_id).await,
        "signer2 session storage must have the session"
    );
    assert!(
        session3.has_session(&user_id, &sign3_response.session_id).await,
        "signer3 session storage must have the session"
    );

    public_key_package
        .verifying_key()
        .verify(&message, &signature)
        .expect("signature must be valid");
}

#[tokio::test]
async fn test_basic_signing_flow() {
    let verifiers_map = helper_2();

    let aggregator = FrostAggregator::new(
        AggregatorConfig {
            threshold: 2,
            total_participants: 3,
            verifier_identifiers: vec![1, 2, 3],
        },
        verifiers_map,
        Arc::new(MockAggregatorUserStorage::new()),
    );

    let user_id = "basic_user".to_string();
    let message = b"hello_basic_signing".to_vec();

    let public_key_package = aggregator.run_dkg_flow(user_id.clone()).await.unwrap();

    let signature = aggregator
        .run_signing_flow(user_id.clone(), &message, None)
        .await
        .unwrap();

    public_key_package
        .verifying_key()
        .verify(&message, &signature)
        .expect("signature must be valid");
}
