use tonic::{Request, Response, Status, Code};
use std::sync::Arc;
use tokio::sync::RwLock;

// this is delegation of responsibilities

use crate::{
    btc_signer_proto::{
        btc_signer_server::{BtcSigner as BtcSignerService},
        SignRequest, SignResponse, PublicKeyRequest, PublicKeyResponse,
        MultiSigRequest, MultiSigResponse, PartialSignRequest, PartialSignResponse,
        AggregateRequest, AggregateResponse, Signature as ProtoSignature,
        PublicKey as ProtoPublicKey, PartialSignature as ProtoPartialSignature,
    },
    traits::{Signer, MultiSigner},
    types::{BtcSignature, BtcPublicKey, SignatureAlgorithm, PublicKeyFormat},
    multi_signer::BtcMultiSigner,
    errors::{BtcSignerError, Result},
};

pub struct BtcSignerGrpcService {
    multi_signer: Arc<RwLock<BtcMultiSigner>>, // link to multisigner
}

impl BtcSignerGrpcService {
    pub fn new(multi_signer: BtcMultiSigner) -> Self {
        Self {
            multi_signer: Arc::new(RwLock::new(multi_signer)),
        }
    }

    fn error_to_status(error: BtcSignerError) -> Status { // cool erros
        match error {
            BtcSignerError::SignerNotFound { .. } => {
                Status::new(Code::NotFound, error.to_string())
            }
            BtcSignerError::SessionNotFound { .. } => {
                Status::new(Code::NotFound, error.to_string())
            }
            BtcSignerError::InvalidSignature => {
                Status::new(Code::InvalidArgument, error.to_string())
            }
            BtcSignerError::InvalidPublicKey => {
                Status::new(Code::InvalidArgument, error.to_string())
            }
            BtcSignerError::InvalidMessage => {
                Status::new(Code::InvalidArgument, error.to_string())
            }
            BtcSignerError::InvalidThreshold { .. } => {
                Status::new(Code::InvalidArgument, error.to_string())
            }
            BtcSignerError::DuplicateSigner { .. } => {
                Status::new(Code::AlreadyExists, error.to_string())
            }
            BtcSignerError::InsufficientSignatures { .. } => {
                Status::new(Code::FailedPrecondition, error.to_string())
            }
            BtcSignerError::AggregationFailed { .. } => {
                Status::new(Code::Internal, error.to_string())
            }
            BtcSignerError::InvalidSessionState { .. } => {
                Status::new(Code::FailedPrecondition, error.to_string())
            }
            _ => Status::new(Code::Internal, error.to_string()),
        }
    }

    fn btc_signature_to_proto(signature: &BtcSignature) -> ProtoSignature {
        ProtoSignature {
            data: signature.data.clone(),
            algorithm: match signature.algorithm {
                SignatureAlgorithm::SchnorrSecp256k1 => "schnorr_secp256k1".to_string(),
            },
        }
    }

    fn btc_pubkey_to_proto(pubkey: &BtcPublicKey) -> ProtoPublicKey {
        ProtoPublicKey {
            data: pubkey.data.clone(),
            format: match pubkey.format {
                PublicKeyFormat::XOnlyCompressed => "x_only_compressed".to_string(),
                PublicKeyFormat::Compressed => "compressed".to_string(),
            },
        }
    }

    fn partial_sig_to_proto(partial_sig: &crate::types::PartialSignature) -> ProtoPartialSignature {
        ProtoPartialSignature {
            data: partial_sig.signature.as_ref().to_vec(),
            signer_id: partial_sig.signer_id.clone(),
        }
    }
}

#[tonic::async_trait]
impl BtcSignerService for BtcSignerGrpcService {
    async fn sign(
        &self,
        request: Request<SignRequest>,
    ) -> std::result::Result<Response<SignResponse>, Status> {
        let req = request.into_inner();
        let multi_signer = self.multi_signer.read().await;

        let signer = multi_signer
            .get_signer(&req.signer_id)
            .await
            .map_err(Self::error_to_status)?;

        let signature = signer
            .sign(&req.message)
            .await
            .map_err(Self::error_to_status)?;

        let response = SignResponse {
            result: Some(crate::btc_signer_proto::sign_response::Result::Signature(
                Self::btc_signature_to_proto(&signature),
            )),
        };

        Ok(Response::new(response))
    }

    async fn get_public_key(
        &self,
        request: Request<PublicKeyRequest>,
    ) -> std::result::Result<Response<PublicKeyResponse>, Status> {
        let req = request.into_inner();
        let multi_signer = self.multi_signer.read().await;

        let signer = multi_signer
            .get_signer(&req.signer_id)
            .await
            .map_err(Self::error_to_status)?;

        let pubkey = signer
            .get_public_key()
            .await
            .map_err(Self::error_to_status)?;

        let response = PublicKeyResponse {
            result: Some(crate::btc_signer_proto::public_key_response::Result::PublicKey(
                Self::btc_pubkey_to_proto(&pubkey),
            )),
        };

        Ok(Response::new(response))
    }

    async fn create_multi_sig(
        &self,
        request: Request<MultiSigRequest>,
    ) -> std::result::Result<Response<MultiSigResponse>, Status> {
        let req = request.into_inner();
        let multi_signer = self.multi_signer.read().await;

        let session_id = multi_signer
            .create_multi_sig_session(&req.message, &req.signer_ids, req.threshold)
            .await
            .map_err(Self::error_to_status)?;

        let response = MultiSigResponse {
            result: Some(crate::btc_signer_proto::multi_sig_response::Result::SessionId(
                session_id,
            )),
        };

        Ok(Response::new(response))
    }

    async fn partial_sign(
        &self,
        request: Request<PartialSignRequest>,
    ) -> std::result::Result<Response<PartialSignResponse>, Status> {
        let req = request.into_inner();
        let mut multi_signer = self.multi_signer.write().await;

        let partial_sig = multi_signer
            .add_partial_signature(&req.session_id, &req.signer_id, &req.message)
            .await
            .map_err(Self::error_to_status)?;

        let response = PartialSignResponse {
            result: Some(crate::btc_signer_proto::partial_sign_response::Result::PartialSignature(
                Self::partial_sig_to_proto(&partial_sig),
            )),
        };

        Ok(Response::new(response))
    }

    async fn aggregate_signatures(
        &self,
        request: Request<AggregateRequest>,
    ) -> std::result::Result<Response<AggregateResponse>, Status> {
        let req = request.into_inner();
        let mut multi_signer = self.multi_signer.write().await;

        let final_signature = multi_signer
            .aggregate_signatures(&req.session_id)
            .await
            .map_err(Self::error_to_status)?;

        let response = AggregateResponse {
            result: Some(crate::btc_signer_proto::aggregate_response::Result::FinalSignature(
                Self::btc_signature_to_proto(&final_signature),
            )),
        };

        Ok(Response::new(response))
    }
}

pub struct BtcSignerServer {
    service: BtcSignerGrpcService,
}

impl BtcSignerServer {
    pub fn new(multi_signer: BtcMultiSigner) -> Self {
        Self {
            service: BtcSignerGrpcService::new(multi_signer),
        }
    }

    pub async fn serve(self, addr: std::net::SocketAddr) -> Result<()> {
        use crate::btc_signer_proto::btc_signer_server::BtcSignerServer;

        println!("Starting BTC Signer gRPC server on {}", addr);

        tonic::transport::Server::builder()
            .add_service(BtcSignerServer::new(self.service))
            .serve(addr)
            .await
            .map_err(|e| BtcSignerError::Transport(e))?;

        Ok(())
    }
}

pub struct BtcSignerClient {
    inner: crate::btc_signer_proto::btc_signer_client::BtcSignerClient<tonic::transport::Channel>,
}

impl BtcSignerClient { // we hide all hard stuff - so now we have cool api
    pub async fn connect<T>(endpoint: T) -> Result<Self>
    where
        T: TryInto<tonic::transport::Endpoint>,
        T::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        let client = crate::btc_signer_proto::btc_signer_client::BtcSignerClient::connect(endpoint)
            .await
            .map_err(|e| BtcSignerError::Transport(e))?;

        Ok(Self { inner: client })
    }

    // there we are converting to default types

    pub async fn sign(&mut self, message: &[u8], signer_id: &str) -> Result<BtcSignature> {
        let request = SignRequest {
            message: message.to_vec(),
            signer_id: signer_id.to_string(),
        };

        let response = self.inner
            .sign(request)
            .await
            .map_err(|e| BtcSignerError::Status(e))?
            .into_inner();

        match response.result {
            Some(crate::btc_signer_proto::sign_response::Result::Signature(sig)) => {
                Ok(BtcSignature {
                    data: sig.data,
                    algorithm: SignatureAlgorithm::SchnorrSecp256k1,
                })
            }
            Some(crate::btc_signer_proto::sign_response::Result::Error(err)) => {
                Err(BtcSignerError::Internal(err))
            }
            None => Err(BtcSignerError::Internal("No result in response".to_string())),
        }
    }

    pub async fn get_public_key(&mut self, signer_id: &str) -> Result<BtcPublicKey> {
        let request = PublicKeyRequest {
            signer_id: signer_id.to_string(),
        };

        let response = self.inner
            .get_public_key(request)
            .await
            .map_err(|e| BtcSignerError::Status(e))?
            .into_inner();

        match response.result {
            Some(crate::btc_signer_proto::public_key_response::Result::PublicKey(pk)) => {
                Ok(BtcPublicKey {
                    data: pk.data,
                    format: PublicKeyFormat::XOnlyCompressed,
                })
            }
            Some(crate::btc_signer_proto::public_key_response::Result::Error(err)) => {
                Err(BtcSignerError::Internal(err))
            }
            None => Err(BtcSignerError::Internal("No result in response".to_string())),
        }
    }
}