use crate::error::FlowProcessorError;
use crate::flow_router::FlowProcessorRouter;
use tracing::{info, instrument};

const LOG_PATH: &str = "flow_processor:routes:exit_spark_flow";

#[instrument(level = "info", skip(x), ret)]
pub async fn handle(x: &mut FlowProcessorRouter) -> Result<(), FlowProcessorError> {
    info!("[{LOG_PATH}] Handling exit spark flow ...");
    Ok(())
}
