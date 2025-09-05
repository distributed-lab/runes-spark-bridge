use crate::errors::FlowProcessorError;
use crate::types::*;
use frost::aggregator::FrostAggregator;
use frost::utils::convert_public_key_package;
use persistent_storage::init::PostgresRepo;
use std::str::FromStr;
use tokio::sync::mpsc;
use tracing;
use uuid::Uuid;

// This struct is used to route the message to the correct flow
// This struct instance is created for each message that is sent to the flow processor
pub struct FlowProcessorRouter {
    storage: PostgresRepo,
    flow_id: Uuid,
    response_sender: OneshotFlowProcessorSender,
    task_sender: mpsc::Sender<Uuid>,
    frost_aggregator: FrostAggregator,
}

impl FlowProcessorRouter {
    pub fn new(
        storage: PostgresRepo,
        flow_id: Uuid,
        response_sender: OneshotFlowProcessorSender,
        task_sender: mpsc::Sender<Uuid>,
        frost_aggregator: FrostAggregator,
    ) -> Self {
        Self {
            storage,
            flow_id,
            response_sender,
            task_sender,
            frost_aggregator,
        }
    }

    pub async fn run(mut self, message: FlowProcessorMessage) {
        let response = match message {
            FlowProcessorMessage::RunDkgFlow(request) => {
                let response = self.run_dkg_flow(request).await;
                let answer = response.map(|response| FlowProcessorResponse::RunDkgFlow(response));
                answer
            }
            FlowProcessorMessage::BridgeRunes(request) => {
                let response = self.run_bridge_runes_flow(request).await;
                let answer = response.map(|response| FlowProcessorResponse::BridgeRunes(response));
                answer
            }
            FlowProcessorMessage::ExitSpark(request) => {
                let response = self.run_exit_spark_flow(request).await;
                let answer = response.map(|response| FlowProcessorResponse::ExitSpark(response));
                answer
            }
        };

        let _ = self.response_sender.send(response).map_err(|_| {
            tracing::error!("[router] Failed to send response for flow id {}", self.flow_id);
        });

        let _ = self.task_sender.send(self.flow_id).await.map_err(|_| {
            tracing::error!("[router] Failed to send task for flow id {}", self.flow_id);
        });
    }

    async fn run_dkg_flow(&mut self, request: DkgFlowRequest) -> Result<DkgFlowResponse, FlowProcessorError> {
        let public_key_package = self
            .frost_aggregator
            .run_dkg_flow(bitcoin::secp256k1::PublicKey::from_str(&request.user_public_key).unwrap())
            .await
            .map_err(|e| FlowProcessorError::FrostAggregatorError(e.to_string()))?;

        let public_key = convert_public_key_package(public_key_package)
            .map_err(|e| FlowProcessorError::InvalidDataError(e.to_string()))?;

        Ok(DkgFlowResponse {
            public_key: public_key.to_string(),
        })
    }

    async fn run_bridge_runes_flow(
        &mut self,
        request: BridgeRunesRequest,
    ) -> Result<BridgeRunesResponse, FlowProcessorError> {
        Ok(BridgeRunesResponse {
            message: format!("message for {}", request.request_id),
        })
    }

    async fn run_exit_spark_flow(
        &mut self,
        request: ExitSparkRequest,
    ) -> Result<ExitSparkResponse, FlowProcessorError> {
        Ok(ExitSparkResponse {
            message: format!("message for {}", request.request_id),
        })
    }
}
