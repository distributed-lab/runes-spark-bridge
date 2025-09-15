use crate::error::FlowProcessorError;
use crate::flow_router::FlowProcessorRouter;
use tracing::info;

const LOG_PATH: &str = "flow_processor:routes:bridge_runes_flow.rs";

pub async fn handle(x: &mut FlowProcessorRouter) -> Result<(), FlowProcessorError> {
    info!("[{LOG_PATH}] Handling btc addr bridge runes flow ...");
    Ok(())
}
