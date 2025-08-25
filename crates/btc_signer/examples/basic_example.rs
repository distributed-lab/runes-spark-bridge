use btc_signer::{
    BtcSigner, BtcMultiSigner, BtcSignerClient,
    traits::{Signer, MultiSigner},
    errors::Result,
};
use tokio;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("Bitcoin Multi-Signer Basic Usage Example");

    basic_signer_example().await?;

    multi_signature_example().await?;

    remote_signer_example().await?;

    Ok(())
}

async fn basic_signer_example() -> Result<()> {
    info!("--- Basic Signer Example ---");

    let signer = BtcSigner::new("alice".to_string());
    info!("Created signer: {}", signer.get_id());

    let pubkey = signer.get_public_key().await?;
    info!("Public key: {}", hex::encode(&pubkey.data));

    let message = b"Hello, Bitcoin!";
    let signature = signer.sign(message).await?;
    info!("Signed message, signature length: {} bytes", signature.data.len());
    info!("Signature: {}", hex::encode(&signature.data));

    let partial_sig = signer.create_partial_signature(message).await?;
    info!("Created partial signature from signer: {}", partial_sig.signer_id);

    Ok(())
}

async fn multi_signature_example() -> Result<()> {
    info!("--- Multi-Signature Example ---");

    let mut multi_signer = BtcMultiSigner::new();

    let signer_ids = vec!["alice", "bob", "charlie", "david", "eve"];
    for id in &signer_ids {
        let signer = Box::new(BtcSigner::new(id.to_string()));
        multi_signer.add_signer(signer).await?;
        info!("Added signer: {}", id);
    }

    info!("Multisigner created with {} signers", multi_signer.signer_count());

    let message = b"Multi-signature transaction data";
    let participants = vec![
        "alice".to_string(),
        "bob".to_string(),
        "charlie".to_string(),
    ];
    let threshold = 2; 

    let session_id = multi_signer
        .create_multi_sig_session(message, &participants, threshold)
        .await?;

    info!("Created multisig session: {}", session_id);
    info!("Threshold: {}/{}", threshold, participants.len());

    info!("Collecting partial signatures!!!");

    let partial_sig1 = multi_signer
        .add_partial_signature(&session_id, "alice", message)
        .await?;
    info!("Added partial signature from: {}", partial_sig1.signer_id);

    let partial_sig2 = multi_signer
        .add_partial_signature(&session_id, "bob", message)
        .await?;
    info!("Added partial signature from: {}", partial_sig2.signer_id);

    info!("Aggregating signatures!!!");
    let final_signature = multi_signer
        .aggregate_signatures(&session_id)
        .await?;

    info!("Successfully created aggregated signature!");
    info!("Final signature length: {} bytes", final_signature.data.len());
    info!("Final signature: {}", hex::encode(&final_signature.data));

    Ok(())
}

#[allow(dead_code)]
async fn remote_signer_example() -> Result<()> {
    info!("--- Remote Signer Example ---");

    let mut client = BtcSignerClient::connect("http://127.0.0.1:50051").await?;
    info!("Connected to remote signer");

    let pubkey = client.get_public_key("signer_0").await?;
    info!("Remote public key: {}", hex::encode(&pubkey.data));

    let message = b"Remote signing test";
    let signature = client.sign(message, "signer_0").await?;
    info!("Remote signature: {}", hex::encode(&signature.data));

    Ok(())
}

#[allow(dead_code)]
async fn error_handling_example() -> Result<()> {
    info!("--- Error Handling Example ---");

    let mut multi_signer = BtcMultiSigner::new();

    match multi_signer.get_signer("non_existent").await {
        Ok(_) => info!("This cant happen"),
        Err(e) => info!("Expected error: {}", e),
    }

    let message = b"test message";
    let participants = vec!["alice".to_string()];

    match multi_signer.create_multi_sig_session(message, &participants, 5).await {
        Ok(_) => info!("This cant happen"),
        Err(e) => info!("Expected error: {}", e),
    }

    Ok(())
}

#[allow(dead_code)]
async fn performance_example() -> Result<()> {
    info!("--- Performance Example ---");

    let signer = BtcSigner::new("performance_test".to_string());
    let message = b"Performance test message";

    let start = std::time::Instant::now();
    let iterations = 1000;

    for i in 0..iterations {
        let _signature = signer.sign(message).await?;
        if (i + 1) % 100 == 0 {
            info!("Completed {} signatures", i + 1);
        }
    }

    let duration = start.elapsed();
    info!("Signed {} messages in {:?}", iterations, duration);
    info!("Average time per signature: {:?}", duration / iterations);

    Ok(())
}