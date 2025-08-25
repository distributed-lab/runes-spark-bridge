use btc_signer::{
    BtcSignerClient,
    errors::Result,
};
use tokio;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("Bitcoin Signer Client Example");

    let mut client = match BtcSignerClient::connect("http://127.0.0.1:50051").await {
        Ok(client) => {
            info!("Successfully connected to Bitcoin Signer server");
            client
        }
        Err(e) => {
            error!("Failed to connect to server: {}", e);
            error!("Make sure the server is running with: cargo run");
            return Err(e);
        }
    };

    info!("Test 1: Get Public Key");
    match client.get_public_key("signer_0").await {
        Ok(pubkey) => {
            info!("Public key for signer_0: {}", hex::encode(&pubkey.data));
        }
        Err(e) => {
            error!("Failed to get public key: {}", e);
        }
    }

    info!("Test 2: Sign Message");
    let message = b"Hello from gRPC client!";
    match client.sign(message, "signer_0").await {
        Ok(signature) => {
            info!("Successfully signed message");
            info!("Signature: {}", hex::encode(&signature.data));
        }
        Err(e) => {
            error!("Failed to sign message: {}", e);
        }
    }

    info!("Test 3: Sign Multiple Messages");
    let messages = vec![
        b"Message 1".as_slice(),
        b"Message 2".as_slice(),
        b"Message 3".as_slice(),
    ];

    for (i, msg) in messages.iter().enumerate() {
        match client.sign(msg, "signer_1").await {
            Ok(signature) => {
                info!("Message {}: {}", i + 1, hex::encode(&signature.data[..8]));
            }
            Err(e) => {
                error!("Failed to sign message {}: {}", i + 1, e);
            }
        }
    }

    info!("Test 4: Test Different Signers");
    let test_message = b"Test message for different signers";

    for i in 0..5 {
        let signer_id = format!("signer_{}", i);
        match client.sign(test_message, &signer_id).await {
            Ok(signature) => {
                info!("{}: {}", signer_id, hex::encode(&signature.data[..8]));
            }
            Err(e) => {
                error!("Failed to sign with {}: {}", signer_id, e);
            }
        }
    }

    info!("Test 5: Error Handling");
    match client.sign(b"test", "non_existent_signer").await {
        Ok(_) => {
            error!("This should have failed!");
        }
        Err(e) => {
            info!("Expected error for non-existent signer: {}", e);
        }
    }

    info!("Client example completed");
    Ok(())
}