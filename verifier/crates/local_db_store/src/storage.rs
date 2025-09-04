use async_trait::async_trait;
use frost::traits::SignerUserStorage;
pub use persistent_storage::error::DbError;
use persistent_storage::init::{PersistentDbConn, PersistentRepoShared, PersistentRepoTrait, PostgresPool};
use sqlx::Acquire;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct LocalDbStore(pub PostgresPool);

impl LocalDbStore {
    pub fn into_shared(self) -> Arc<LocalDbStore> {
        Arc::new(self)
    }
}
