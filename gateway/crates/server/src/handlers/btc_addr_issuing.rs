use crate::error::GatewayError;
use crate::state::AppState;
use anyhow::bail;
use axum::{Json, extract::State};
use gateway_flow_processor::types::{BtcAddrIssueRequest, FlowProcessorMessage, FlowProcessorResponse};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{debug, instrument};

#[derive(Deserialize, Debug)]
pub struct RunesAddressIssueRequest {
    pub user_public_key: String,
    pub rune_id: String,
    pub amount: u64,
}

#[derive(Serialize, Debug)]
pub struct RunesAddressIssueResponse {
    pub address: String,
}

/// Handles Btc address issuing for replenishment
#[instrument(level = "info", skip(state, request), fields(request = ?request), ret)]
pub async fn handle(
    State(state): State<AppState>,
    Json(request): Json<RunesAddressIssueRequest>,
) -> Result<Json<RunesAddressIssueResponse>, GatewayError> {
    _handle_inner(state, request)
        .await
        .map_err(|e| GatewayError::FlowProcessorError(format!("Failed to issue deposit address for bridging: {e}")))
}

#[instrument(level = "debug", skip(state, request), fields(request = ?request), ret)]
async fn _handle_inner(
    state: AppState,
    request: RunesAddressIssueRequest,
) -> anyhow::Result<Json<RunesAddressIssueResponse>> {
    debug!("[handler-btc-addr-issuing] Handling request: {request:?}");
    let possible_response = state
        .flow_sender
        .send_messsage(FlowProcessorMessage::IssueDepositAddress(BtcAddrIssueRequest {
            musig_id: frost::types::MusigId::User {
                rune_id: request.rune_id,
                user_public_key: bitcoin::secp256k1::PublicKey::from_str(&request.user_public_key)?,
            },
            amount: request.amount,
        }))
        .await?;
    if let FlowProcessorResponse::IssueDepositAddress(flow_resp) = possible_response {
        Ok(Json(RunesAddressIssueResponse {
            address: flow_resp.addr_to_replenish.to_string(),
        }))
    } else {
        bail!("[Erroneous response on flow processor: {possible_response:?}]")
    }
}
