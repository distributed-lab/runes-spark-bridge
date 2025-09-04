use crate::LocalDbStore;
use frost::errors::SignerError;
use frost::traits::{SignerSessionState, SignerSessionStorage};
use persistent_storage::error::DbError;
use persistent_storage::init::PersistentRepoTrait;
use sqlx::Acquire;
use sqlx::types::Json;
use tracing::{debug, info, instrument};

#[async_trait::async_trait]
impl SignerSessionStorage for LocalDbStore {
    #[instrument(level = "debug", skip(self), ret)]
    async fn get_session_state(
        &self,
        user_id: String,
        session_id: String,
    ) -> Result<Option<SignerSessionState>, DbError> {
        let mut lock = self.0.acquire().await?;
        let pg_conn = lock.acquire().await?;

        debug!(user_id =% user_id, session_id =% session_id, "Get session state" );
        let result: Option<(Json<SignerSessionState>,)> = sqlx::query_as(
            "SELECT session_state FROM verifier.user_session_state WHERE user_id = $1 AND session_id = $1",
        )
        .bind(user_id)
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

    #[instrument(level = "debug", skip(self), ret)]
    async fn set_session_state(
        &self,
        user_id: String,
        session_id: String,
        state: SignerSessionState,
    ) -> Result<(), DbError> {
        let mut lock = self.0.acquire().await?;
        let pg_conn = lock.acquire().await?;

        debug!(user_id =% user_id, session_id =% session_id, state =? state, "Set session state" );
        let _ = sqlx::query("INSERT INTO verifier.user_session_state (user_id, session_id, session_state) VALUES ($1, $2, $3) ON CONFLICT (session_id) DO UPDATE SET session_state = $2")
            .bind(user_id)
            .bind(session_id)
            .bind(Json(state))
            .execute(pg_conn)
            .await
            .map_err(|e| DbError::BadRequest(e.to_string()))?;
        Ok(())
    }
}
