use async_trait::async_trait;
use frost::errors::AggregatorError;
use frost::traits::SignerClient as SignerClientTrait;
use frost::traits::*;
use gateway_config_parser::config::VerifierConfig;
use reqwest::{Client, Url};
use serde::Serialize;
use serde::de::DeserializeOwned;

pub struct SignerClient {
    config: VerifierConfig,
    client: reqwest::Client,
}

const DKG_ROUND_1_PATH: &str = "/api/gateway/dkg-round-1";
const DKG_ROUND_2_PATH: &str = "/api/gateway/dkg-round-2";
const DKG_FINALIZE_PATH: &str = "/api/gateway/dkg-finalize";
const SIGN_ROUND_1_PATH: &str = "/api/gateway/sign-round-1";
const SIGN_ROUND_2_PATH: &str = "/api/gateway/sign-round-2";

impl SignerClient {
    pub fn new(config: VerifierConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    pub async fn send_request<T: Serialize, U: DeserializeOwned>(
        &self,
        url: Url,
        request: T,
    ) -> Result<U, AggregatorError> {
        let response = self
            .client
            .post(url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AggregatorError::Internal(format!("Failed to send HTTP request: {:?}", e)))?;

        if response.status().is_success() {
            let response: U = response
                .json()
                .await
                .map_err(|e| AggregatorError::Internal(format!("Failed to deserialize response: {:?}", e)))?;
            Ok(response)
        } else {
            Err(AggregatorError::HttpError(format!(
                "Failed to send HTTP request with status {}, error: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )))
        }
    }

    pub async fn get_url(&self, path: &str) -> Result<Url, AggregatorError> {
        Url::parse(&format!("{}{}", self.config.address, path))
            .map_err(|e| AggregatorError::Internal(format!("Failed to parse URL: {:?}", e)))
    }
}

#[async_trait]
impl SignerClientTrait for SignerClient {
    async fn dkg_round_1(&self, request: DkgRound1Request) -> Result<DkgRound1Response, AggregatorError> {
        let url = self.get_url(DKG_ROUND_1_PATH).await?;

        self.send_request(url, request).await
    }

    async fn dkg_round_2(&self, request: DkgRound2Request) -> Result<DkgRound2Response, AggregatorError> {
        let url = self.get_url(DKG_ROUND_2_PATH).await?;

        self.send_request(url, request).await
    }

    async fn dkg_finalize(&self, request: DkgFinalizeRequest) -> Result<DkgFinalizeResponse, AggregatorError> {
        let url = self.get_url(DKG_FINALIZE_PATH).await?;

        self.send_request(url, request).await
    }

    async fn sign_round_1(&self, request: SignRound1Request) -> Result<SignRound1Response, AggregatorError> {
        let url = self.get_url(SIGN_ROUND_1_PATH).await?;

        self.send_request(url, request).await
    }

    async fn sign_round_2(&self, request: SignRound2Request) -> Result<SignRound2Response, AggregatorError> {
        let url = self.get_url(SIGN_ROUND_2_PATH).await?;

        self.send_request(url, request).await
    }
}
