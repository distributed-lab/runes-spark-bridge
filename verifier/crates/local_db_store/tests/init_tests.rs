mod utils;

mod init_tests {
    use crate::utils::TEST_LOGGER;
    use persistent_storage::{config::PostgresDbCredentials, init::PostgresRepo};
    use sqlx::Connection;
    use verifier_local_db_store::LocalDbStore;

    #[ignore]
    #[tokio::test]
    pub async fn pg_conn_health_check() -> anyhow::Result<()> {
        let _ = dotenv::dotenv();
        let _ = *TEST_LOGGER;
        let db_entity = PostgresRepo::from_config(PostgresDbCredentials::from_db_url()?).await?;
        let conn = db_entity.pool;
        let mut storage = LocalDbStore(conn);
        {
            let mut conn_local = storage.0.acquire().await?;
            assert_eq!(conn_local.ping().await?, ());
        }
        Ok(())
    }
}
