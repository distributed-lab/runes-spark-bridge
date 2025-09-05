use crate::LocalDbStore;
use bitcoin::secp256k1::PublicKey;
use frost::errors::SignerError;
use frost::traits::{SignerSessionState, SignerSessionStorage};
use global_utils::common_types::Secp256K1PubkeyWrapped;
use persistent_storage::error::DbError;
use persistent_storage::init::PersistentRepoTrait;
use sqlx::Acquire;
use sqlx::types::Json;
use tracing::{debug, info, instrument};
use uuid::Uuid;

#[async_trait::async_trait]
impl SignerSessionStorage for LocalDbStore {
    #[instrument(level = "trace", skip(self), ret)]
    async fn get_session_state(
        &self,
        user_pubkey: PublicKey,
        session_id: Uuid,
    ) -> Result<Option<SignerSessionState>, DbError> {
        let mut lock = self.0.acquire().await?;
        let pg_conn = lock.acquire().await?;

        let result: Option<(Json<SignerSessionState>,)> = sqlx::query_as(
            "SELECT session_state FROM verifier.user_session_state WHERE user_pubkey = $1 AND session_uuid = $2",
        )
        .bind(Secp256K1PubkeyWrapped(user_pubkey))
        .bind(session_id)
        .fetch_optional(pg_conn)
        .await
        .map_err(|e| DbError::BadRequest(e.to_string()))?;

        if let Some((user_state,)) = result {
            Ok(Some(user_state.0))
        } else {
            Ok(None)
        }
    }

    #[instrument(level = "trace", skip(self), ret)]
    async fn set_session_state(
        &self,
        user_pubkey: PublicKey,
        session_uuid: Uuid,
        state: SignerSessionState,
    ) -> Result<(), DbError> {
        let mut lock = self.0.acquire().await?;
        let pg_conn = lock.acquire().await?;

        let _ = sqlx::query("INSERT INTO verifier.user_session_state (user_pubkey, session_uuid, session_state) VALUES ($1, $2, $3) ON CONFLICT (user_pubkey, session_uuid) DO UPDATE SET session_state = $3")
            .bind(Secp256K1PubkeyWrapped(user_pubkey))
            .bind(session_uuid)
            .bind(Json(state))
            .execute(pg_conn)
            .await
            .map_err(|e| DbError::BadRequest(e.to_string()))?;
        Ok(())
    }
}
