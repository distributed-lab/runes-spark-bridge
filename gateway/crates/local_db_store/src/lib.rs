pub mod schemas;
pub use persistent_storage::error::DbError;
use persistent_storage::{
    config::*,
    init::{PostgresPool, PostgresRepo},
};

pub struct Storage {
    pub postgres_repo: PostgresRepo,
}

impl Storage {
    pub async fn new(database_url: String) -> Result<Self, DbError> {
        let postgres_repo = PostgresRepo::from_config(PostgresDbCredentials { url: database_url }).await?;
        Ok(Self { postgres_repo })
    }

    pub async fn get_conn(&self) -> Result<PostgresPool, DbError> {
        Ok(self.postgres_repo.pool.clone())
    }
}
