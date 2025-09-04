use std::sync::Arc;

use async_trait::async_trait;
use sqlx::{PgPool, Pool, Postgres, pool::PoolConnection};
use tracing::{instrument, trace};

use crate::{config::PostgresDbCredentials, error::DbError};

pub type PostgresPool = Pool<Postgres>;
pub type PersistentDbConn = PoolConnection<Postgres>;
pub type PersistentRepoShared = Arc<Box<dyn PersistentRepoTrait>>;

#[derive(Debug, Clone)]
pub struct PostgresRepo {
    pub pool: PostgresPool,
}

/// Trait for implementing Persistent storage that'd use Postgres
#[async_trait]
pub trait PersistentRepoTrait: Send + Sync {
    async fn get_conn(&self) -> Result<PersistentDbConn, DbError>;
}

impl PostgresRepo {
    /// Provides **unmigrated** connection to db
    ///
    /// Why **unmigrated**? - it requires path to `migrate` folder as literal
    #[instrument(level = "trace", ret)]
    pub async fn from_config(creds: PostgresDbCredentials) -> Result<Self, DbError> {
        trace!("Creating PG connection pool with creds: {:?}", creds);
        let pool = PgPool::connect(&creds.url)
            .await
            .map_err(|x| DbError::FailedToEstablishDbConn(x, creds.url.clone()))?;
        trace!(db_url = creds.url, "Creating Postgres pool with config");
        Ok(Self { pool })
    }

    pub fn into_shared(self) -> PersistentRepoShared {
        Arc::new(Box::new(self))
    }

    pub async fn ping(conn: &mut PersistentDbConn) -> Result<(), DbError> {
        db_helpers::ping(conn).await
    }
}

#[async_trait]
impl PersistentRepoTrait for PostgresRepo {
    async fn get_conn(&self) -> Result<PersistentDbConn, DbError> {
        Ok(self.pool.acquire().await?)
    }
}

pub mod db_helpers {
    use sqlx::Connection;

    use super::*;

    #[inline]
    pub async fn ping(conn: &mut PersistentDbConn) -> Result<(), DbError> {
        Ok(conn.ping().await?)
    }
}
