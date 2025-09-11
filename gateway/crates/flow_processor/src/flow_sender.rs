use crate::error::FlowProcessorError;
use crate::types::*;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

// This trait is used in order to send typed messages to the flow processor
#[async_trait::async_trait]
pub trait TypedMessageSender<S, R> {
    async fn send(&self, message: S) -> Result<R, FlowProcessorError>;
}

// This is helper struct that sends messages to the flow processor and waits for the response
// This struct implements the TypedMessageSender trait for each type of message
#[derive(Clone)]
pub struct FlowSender {
    tx_sender: mpsc::Sender<(FlowProcessorMessage, OneshotFlowProcessorSender)>,
    cancellation_token: CancellationToken,
}

impl FlowSender {
    pub fn new(
        tx_sender: mpsc::Sender<(FlowProcessorMessage, OneshotFlowProcessorSender)>,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            tx_sender,
            cancellation_token,
        }
    }

    pub async fn send_messsage(
        &self,
        message: FlowProcessorMessage,
    ) -> Result<FlowProcessorResponse, FlowProcessorError> {
        let (router_sender, router_receiver) = oneshot::channel::<Result<FlowProcessorResponse, FlowProcessorError>>();
        let send_response = self.tx_sender.send((message, router_sender)).await;

        match send_response {
            Ok(_) => router_receiver
                .await
                .map_err(|_| FlowProcessorError::ChannelClosedError("Channel closed".to_string()))?,
            Err(e) => Err(FlowProcessorError::ChannelClosedError(e.to_string())),
        }
    }

    pub async fn shutdown(&self) {
        self.cancellation_token.cancel();
    }
}

#[async_trait::async_trait]

impl TypedMessageSender<BtcAddrIssueRequest, BtcAddrIssueResponse> for FlowSender {
    async fn send(&self, dkg_message: BtcAddrIssueRequest) -> Result<BtcAddrIssueResponse, FlowProcessorError> {
        let response = self
            .send_messsage(FlowProcessorMessage::IssueDepositAddress(dkg_message))
            .await?;
        match response {
            FlowProcessorResponse::IssueDepositAddress(response) => Ok(response),
            x => Err(FlowProcessorError::InvalidResponseType(format!(
                "Invalid response type, obtain: {x:?}, expected: [FlowProcessorResponse::RunDkgFlow]"
            ))),
        }
    }
}

#[async_trait::async_trait]
impl TypedMessageSender<BridgeRunesRequest, BridgeRunesResponse> for FlowSender {
    async fn send(&self, bridge_runes_message: BridgeRunesRequest) -> Result<BridgeRunesResponse, FlowProcessorError> {
        let response = self
            .send_messsage(FlowProcessorMessage::BridgeRunes(bridge_runes_message))
            .await?;
        match response {
            FlowProcessorResponse::BridgeRunes(response) => Ok(response),
            x => Err(FlowProcessorError::InvalidResponseType(format!(
                "Invalid response type, obtain: {x:?}, expected: [FlowProcessorResponse::BridgeRunes]"
            ))),
        }
    }
}

#[async_trait::async_trait]
impl TypedMessageSender<ExitSparkRequest, ExitSparkResponse> for FlowSender {
    async fn send(&self, exit_spark_message: ExitSparkRequest) -> Result<ExitSparkResponse, FlowProcessorError> {
        let response = self
            .send_messsage(FlowProcessorMessage::ExitSpark(exit_spark_message))
            .await?;
        match response {
            FlowProcessorResponse::ExitSpark(response) => Ok(response),
            x => Err(FlowProcessorError::InvalidResponseType(format!(
                "Invalid response type, obtain: {x:?}, expected: [FlowProcessorMessage::ExitSpark]"
            ))),
        }
    }
}
