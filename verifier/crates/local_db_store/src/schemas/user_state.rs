use crate::DbError;
use crate::LocalDbStore;
use frost::traits::{SignerSessionState, SignerSessionStorage, SignerUserState, SignerUserStorage};
use global_utils::common_types::Secp256K1PubkeyWrapped;
use sqlx::Acquire;
use sqlx::types::Json;
use tracing::{debug, instrument};

#[async_trait::async_trait]
impl SignerUserStorage for LocalDbStore {
    #[instrument(level = "trace", skip(self), ret)]
    async fn get_user_state(
        &self,
        user_pubkey: bitcoin::secp256k1::PublicKey,
    ) -> Result<Option<SignerUserState>, DbError> {
        let mut lock = self.0.acquire().await?;
        let pg_conn = lock.acquire().await?;

        let result: Option<(Json<SignerUserState>,)> =
            sqlx::query_as("SELECT signing_state FROM verifier.user_state WHERE user_pubkey = $1")
                .bind(Secp256K1PubkeyWrapped(user_pubkey))
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
    async fn set_user_state(
        &self,
        user_pubkey: bitcoin::secp256k1::PublicKey,
        user_state: SignerUserState,
    ) -> Result<(), DbError> {
        let mut conn = self.0.acquire().await?;
        let pg_conn = conn.acquire().await?;

        let _ = sqlx::query("INSERT INTO verifier.user_state (user_pubkey, signing_state) VALUES ($1, $2) ON CONFLICT (user_pubkey) DO UPDATE SET signing_state = $2")
            .bind(Secp256K1PubkeyWrapped(user_pubkey))
            .bind(Json(user_state))
            .execute(pg_conn)
            .await
            .map_err(|e| DbError::BadRequest(e.to_string()))?;

        Ok(())
    }
}
