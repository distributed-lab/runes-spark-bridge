use crate::errors::VerifierError;
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use frost::traits::{DkgRound1Request, DkgRound1Response};
use tracing::instrument;

#[instrument(level = "debug", skip_all, ret)]
pub async fn handle(
    State(state): State<AppState>,
    Json(request): Json<DkgRound1Request>,
) -> Result<Json<DkgRound1Response>, VerifierError> {
    let response = state.frost_signer.dkg_round_1(request).await?;
    tracing::debug!("DKG round1 response: {:?}", response);

    Ok(Json(response))
}
