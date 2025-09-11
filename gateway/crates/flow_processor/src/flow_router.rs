use crate::error::FlowProcessorError;
use crate::types::*;
use bitcoin::{KnownHrp, Network};
use frost::aggregator::FrostAggregator;
use gateway_local_db_store::storage::LocalDbStorage;
use persistent_storage::init::PostgresRepo;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing;
use tracing::info;
use uuid::Uuid;

const LOG_PATH: &str = "flow_processor";

// This struct is used to route the message to the correct flow
// This struct instance is created for each message that is sent to the flow processor
pub struct FlowProcessorRouter {
    pub storage: Arc<LocalDbStorage>,
    pub flow_id: Uuid,
    pub response_sender: OneshotFlowProcessorSender,
    pub task_sender: mpsc::Sender<Uuid>,
    pub frost_aggregator: FrostAggregator,
    pub network: Network,
}

impl FlowProcessorRouter {
    pub async fn run(mut self, message: FlowProcessorMessage) {
        let response = match message {
            FlowProcessorMessage::IssueDepositAddress(request) => {
                let response = self.run_btc_addr_issuing(request, self.network).await;
                let answer = response.map(|response| FlowProcessorResponse::IssueDepositAddress(response));
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

    #[tracing::instrument(level = "trace", skip(self, request), ret)]
    async fn run_btc_addr_issuing(
        &mut self,
        request: BtcAddrIssueRequest,
        network: Network,
    ) -> Result<BtcAddrIssueResponse, FlowProcessorError> {
        info!("[{LOG_PATH}] issuing btc addr to user with request: {request:?}");
        let pubkey = crate::routes::btc_addr_issuing::handle(self, request, network).await?;
        Ok(BtcAddrIssueResponse {
            addr_to_replenish: pubkey,
        })
    }

    #[tracing::instrument(level = "trace", skip(self, request), ret)]
    async fn run_bridge_runes_flow(
        &mut self,
        request: BridgeRunesRequest,
    ) -> Result<BridgeRunesResponse, FlowProcessorError> {
        info!("[{LOG_PATH}] bridging runes flow with request: {request:?}");
        crate::routes::bridge_runes_flow::handle(self).await?;
        Ok(BridgeRunesResponse {
            message: format!("message for {}", request.request_id),
        })
    }

    #[tracing::instrument(level = "trace", skip(self, request), ret)]
    async fn run_exit_spark_flow(
        &mut self,
        request: ExitSparkRequest,
    ) -> Result<ExitSparkResponse, FlowProcessorError> {
        info!("[{LOG_PATH}] exiting spark flow with request: {request:?}");
        crate::routes::exit_spark_flow::handle(self).await?;
        Ok(ExitSparkResponse {
            message: format!("message for {}", request.request_id),
        })
    }
}
