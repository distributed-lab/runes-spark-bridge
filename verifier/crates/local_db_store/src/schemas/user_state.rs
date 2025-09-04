use crate::DbError;
use crate::LocalDbStore;
use frost::errors::SignerError;
use frost::traits::{SignerSessionState, SignerSessionStorage, SignerUserState, SignerUserStorage};
use persistent_storage::init::PersistentRepoTrait;
use sqlx::Acquire;
use sqlx::types::Json;
use tracing::{debug, instrument};

#[async_trait::async_trait]
impl SignerUserStorage for LocalDbStore {
    #[instrument(level = "debug", skip(self), ret)]
    async fn get_user_state(&self, user_pubkey: String) -> Result<Option<SignerUserState>, DbError> {
        let mut lock = self.0.acquire().await?;
        let pg_conn = lock.acquire().await?;
        debug!(user_pubkey =% user_pubkey, "Get user state");
        let result: Option<(Json<SignerUserState>,)> =
            sqlx::query_as("SELECT signing_state FROM verifier.user_state WHERE user_public_key = $1")
                .bind(user_pubkey)
                .fetch_optional(pg_conn)
                .await
                .map_err(|e| DbError::BadRequest(e.to_string()))?;
        if let Some((user_state,)) = result {
            Ok(Some(user_state.0))
        } else {
            Ok(None)
        }
    }

    #[instrument(level = "debug", skip(self), ret)]
    async fn set_user_state(&self, user_public_key: String, user_state: SignerUserState) -> Result<(), DbError> {
        let mut conn = self.0.acquire().await?;
        let pg_conn = conn.acquire().await?;
        debug!(user_pubkey =% user_public_key, user_state =? user_state, "Get user state");
        let _ = sqlx::query("INSERT INTO verifier.user_state (user_public_key, signing_state) VALUES ($1, $2) ON CONFLICT (user_public_key) DO UPDATE SET signing_state = $2")
            .bind(user_public_key)
            .bind(Json(user_state))
            .execute(pg_conn)
            .await
            .map_err(|e| DbError::BadRequest(e.to_string()))?;

        Ok(())
    }
}
