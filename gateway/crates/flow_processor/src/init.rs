use crate::flow_processor::FlowProcessor;
use crate::flow_sender::FlowSender;
use bitcoin::Network;
use frost::aggregator::FrostAggregator;
use std::sync::Arc;

use gateway_local_db_store::storage::LocalDbStorage;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub fn create_flow_processor(
    storage: Arc<LocalDbStorage>,
    cancellation_retries: u64,
    frost_aggregator: FrostAggregator,
    network: Network,
) -> (FlowProcessor, FlowSender) {
    let (tx_sender, tx_receiver) = mpsc::channel(1000);
    let cancellation_token = CancellationToken::new();
    let flow_processor = FlowProcessor::new(
        tx_receiver,
        storage,
        cancellation_retries,
        frost_aggregator,
        network,
        cancellation_token.clone(),
    );
    let flow_sender = FlowSender::new(tx_sender, cancellation_token);
    (flow_processor, flow_sender)
}
