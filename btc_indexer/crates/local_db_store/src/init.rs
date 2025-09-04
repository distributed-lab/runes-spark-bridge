use std::fmt::{Debug, Formatter};

use async_trait::async_trait;
use persistent_storage::{
    config::PostgresDbCredentials,
    error::DbError,
    init::{PersistentDbConn, PersistentRepoShared, PersistentRepoTrait, PostgresRepo},
};
use tracing::instrument;

/// Has to be understood as "LocalDb - Indexer"
#[derive(Clone)]
pub struct LocalDbIndexer {
    pub postgres_repo: PersistentRepoShared,
}

impl Debug for LocalDbIndexer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Postgres DB")
    }
}

impl LocalDbIndexer {
    #[instrument(level = "trace", ret)]
    pub async fn from_config(creds: PostgresDbCredentials) -> Result<Self, DbError> {
        let pool = PostgresRepo::from_config(creds).await?;
        sqlx::migrate!().run(&pool.pool).await?;
        Ok(Self {
            postgres_repo: pool.into_shared(),
        })
    }
}

#[async_trait]
impl PersistentRepoTrait for LocalDbIndexer {
    async fn get_conn(&self) -> Result<PersistentDbConn, DbError> {
        self.postgres_repo.get_conn().await
    }
}
