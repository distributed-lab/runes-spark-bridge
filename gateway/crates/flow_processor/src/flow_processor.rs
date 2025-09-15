use crate::flow_router::FlowProcessorRouter;
use crate::types::*;
use bitcoin::Network;
use frost::aggregator::FrostAggregator;
use gateway_local_db_store::storage::LocalDbStorage;
use global_utils::common_types::get_uuid;
use std::collections::HashMap;
use std::sync::Arc;
use tokio;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing;
use uuid::Uuid;

// This is core struct that handles flows execution
// For each request it creates a thread that runs the flow
pub struct FlowProcessor {
    pub tx_receiver: mpsc::Receiver<(FlowProcessorMessage, OneshotFlowProcessorSender)>,
    pub flow_receiver: mpsc::Receiver<Uuid>,
    pub flow_sender: mpsc::Sender<Uuid>,
    pub storage: Arc<LocalDbStorage>,
    pub flows: HashMap<Uuid, JoinHandle<()>>,
    pub cancellation_token: CancellationToken,
    pub cancellation_retries: u64,
    pub frost_aggregator: FrostAggregator,
    pub network: Network,
}

impl FlowProcessor {
    pub fn new(
        tx_receiver: mpsc::Receiver<(FlowProcessorMessage, OneshotFlowProcessorSender)>,
        storage: Arc<LocalDbStorage>,
        cancellation_retries: u64,
        frost_aggregator: FrostAggregator,
        network: Network,
        cancellation_token: CancellationToken,
    ) -> Self {
        let (flow_sender, flow_receiver) = mpsc::channel::<Uuid>(1000);
        Self {
            tx_receiver,
            flow_receiver,
            flow_sender,
            storage,
            flows: HashMap::default(),
            cancellation_token,
            cancellation_retries,
            frost_aggregator,
            network,
        }
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                flow = self.flow_receiver.recv() => {
                    match flow {
                        None => {
                            tracing::error!("[main] Task channel closed unexpectedly");
                            break;
                        }
                        Some(flow_id) => {
                            tracing::info!("[main] Received task for id {}", flow_id);
                            let _ = self.flows.remove(&flow_id);
                        }
                    }
                }
                wrapper = self.tx_receiver.recv() => {
                    match wrapper {
                        None => {
                            tracing::error!("[main] Message channel closed unexpectedly");
                            break;
                        }
                        Some(wrapper) => {
                            tracing::info!("[main] Received message");

                            let (message, response_sender) = wrapper;

                            let flow_id = get_uuid();

                            let router = FlowProcessorRouter{
                                storage: self.storage.clone(),
                                flow_id,
                                response_sender,
                                task_sender:  self.flow_sender.clone(),
                                frost_aggregator: self.frost_aggregator.clone(),
                                network: self.network,
                            };

                            let handle = tokio::task::spawn(async move {
                                tracing::info!("[main] Running flow for id {}", flow_id);
                                router.run(message).await;
                                tracing::info!("[main] Flow for id {} finished", flow_id);
                            });

                            self.flows.insert(flow_id, handle);
                        }
                    }
                }
                _ = self.cancellation_token.cancelled() => {
                    tracing::info!("[main] Shutting down flow processor");

                    for i in 0..self.cancellation_retries {
                        if self.flows.is_empty() {
                            return;
                        }
                        while let Ok(flow_id) = self.flow_receiver.try_recv() {
                            let _ = self.flows.remove(&flow_id);
                        }
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        tracing::info!("[main] Waiting flows to finish {}/{}", i + 1, self.cancellation_retries);
                    }

                    for (flow_id, handle) in self.flows.iter() {
                        let _ = handle.abort();
                        tracing::info!("[main] Aborted flow for id {}", flow_id);
                    }

                    self.flows.clear();
                    break;
                }
            }
        }
    }
}
