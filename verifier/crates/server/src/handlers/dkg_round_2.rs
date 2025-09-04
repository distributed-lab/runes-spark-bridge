use crate::errors::VerifierError;
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use frost::traits::{DkgRound2Request, DkgRound2Response};
use tracing::instrument;

#[instrument(level = "debug", skip_all, ret)]
pub async fn handle(
    State(state): State<AppState>,
    Json(request): Json<DkgRound2Request>,
) -> Result<Json<DkgRound2Response>, VerifierError> {
    let response = state.frost_signer.dkg_round_2(request).await?;
    tracing::debug!("DKG round2 response: {:?}", response);

    Ok(Json(response))
}
