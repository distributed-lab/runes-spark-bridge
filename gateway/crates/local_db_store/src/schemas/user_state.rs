use serde_json;
use sqlx;
use std::collections::BTreeMap;

use crate::Storage;

#[async_trait::async_trait]
impl AggregatorUserStorage for Storage {
    async fn get_user_state(&self, user_public_key: String) -> Result<Option<AggregatorUserState>, DbError> {
        let result: Option<(String,)> = sqlx::query_as("SELECT state_data FROM user_state WHERE user_public_key = $1")
            .bind(user_public_key)
            .fetch_optional(&self.get_conn().await?)
            .await
            .map_err(|e| DbError::BadRequest(e.to_string()))?;

        let state: Option<AggregatorUserState> = if let Some((state_data,)) = result {
            Some(
                serde_json::from_str(&state_data)
                    .map_err(|e| DbError::BadRequest(format!("Failed to deserialize state: {}", e)))?,
            )
        } else {
            None
        };

        Ok(state)
    }

    async fn set_user_state(&self, user_public_key: String, state: AggregatorUserState) -> Result<(), DbError> {
        let state_data = serde_json::to_string(&state)
            .map_err(|e| DbError::BadRequest(format!("Failed to serialize state: {}", e)))?;

        sqlx::query("INSERT INTO user_state (user_public_key, state_data) VALUES ($1, $2) ON CONFLICT (user_public_key) DO UPDATE SET state_data = $2")
            .bind(user_public_key)
            .bind(state_data)
            .execute(&self.get_conn().await?)
            .await
            .map_err(|e| DbError::BadRequest(e.to_string()))?;

        Ok(())
    }
}
