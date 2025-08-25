fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/btc_signer.proto")?; // now we use tonic for proto files
    Ok(())
}