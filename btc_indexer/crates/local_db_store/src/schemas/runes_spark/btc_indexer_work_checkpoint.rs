use global_utils::common_types::{TxIdWrapped, UrlWrapped};
use persistent_storage::error::DbError;
use persistent_storage::init::PersistentDbConn;
use serde::{Deserialize, Serialize};
use sqlx::{
    Connection, FromRow, Postgres, Row,
    postgres::PgArguments,
    query::{Query, QueryAs},
    types::{
        Json,
        chrono::{DateTime, Utc},
    },
};
use tracing::instrument;
use uuid::Uuid;

use crate::schemas::common::{ValuesMaxCapacity, ValuesToModifyInit};

const DB_NAME: &str = "runes_spark.btc_indexer_work_checkpoint";

#[derive(Debug, FromRow, Clone, PartialEq, Eq, Hash)]
pub struct BtcIndexerWorkCheckpoint {
    pub uuid: Uuid,
    pub status: StatusBtcIndexer,
    pub task: Json<Task>,
    pub created_at: DateTime<Utc>,
    pub callback_url: UrlWrapped,
    pub error: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, Copy, Eq, PartialEq, Hash)]
#[sqlx(rename_all = "snake_case", type_name = "STATUS_BTC_INDEXER")]
pub enum StatusBtcIndexer {
    Created,
    Processing,
    FinishedSuccess,
    FinishedError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Task {
    TrackTx(TxIdWrapped),
    TrackWallet(String),
}

#[derive(Debug, Clone, Default)]
pub struct Update {
    pub status: Option<StatusBtcIndexer>,
    pub error: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub uuid: Option<Uuid>,
    pub status: Option<StatusBtcIndexer>,
    pub task: Option<Json<Task>>,
    pub callback_url: Option<UrlWrapped>,
}

impl ValuesMaxCapacity for Update {
    const MAX_CAPACITY: usize = 3;
}
impl ValuesMaxCapacity for Filter {
    const MAX_CAPACITY: usize = 4;
}

impl Filter {
    pub fn new() -> Self {
        Filter::default()
    }
    pub fn uuid(mut self, uuid: Uuid) -> Self {
        self.uuid = Some(uuid);
        self
    }
    pub fn status(mut self, status: StatusBtcIndexer) -> Self {
        self.status = Some(status);
        self
    }
    pub fn task(mut self, task: Json<Task>) -> Self {
        self.task = Some(task);
        self
    }
    pub fn callback_url(mut self, callback_url: UrlWrapped) -> Self {
        self.callback_url = Some(callback_url);
        self
    }

    fn get_params_sets(&self) -> Vec<String> {
        const DEFAULT_INIT_PARAM: usize = 1;
        let (mut conditions, mut get_condition_closure) = Filter::init_values_to_modify(DEFAULT_INIT_PARAM);
        if self.uuid.is_some() {
            conditions.push(get_condition_closure("uuid"));
        }
        if self.status.is_some() {
            conditions.push(get_condition_closure("status"));
        }
        if self.task.is_some() {
            conditions.push(get_condition_closure("task"));
        }
        if self.callback_url.is_some() {
            conditions.push(get_condition_closure("callback_url"));
        }
        conditions
    }

    fn bind_params<'a>(&'a self, mut query: Query<'a, Postgres, PgArguments>) -> Query<'a, Postgres, PgArguments> {
        if let Some(uuid) = self.uuid {
            query = query.bind(uuid);
        }
        if let Some(status) = self.status {
            query = query.bind(status);
        }
        if let Some(task) = &self.task {
            query = query.bind(task);
        }
        if let Some(callback_url) = &self.callback_url {
            query = query.bind(callback_url);
        }
        query
    }

    fn bind_params_btc_params<'a>(
        &'a self,
        mut query: QueryAs<'a, Postgres, BtcIndexerWorkCheckpoint, PgArguments>,
    ) -> QueryAs<'a, Postgres, BtcIndexerWorkCheckpoint, PgArguments> {
        if let Some(uuid) = self.uuid {
            query = query.bind(uuid);
        }
        if let Some(status) = self.status {
            query = query.bind(status);
        }
        if let Some(task) = &self.task {
            query = query.bind(task);
        }
        if let Some(callback_url) = &self.callback_url {
            query = query.bind(callback_url);
        }
        query
    }
}

impl Update {
    pub fn new() -> Self {
        Update::default()
    }
    pub fn status(mut self, status: StatusBtcIndexer) -> Self {
        self.status = Some(status);
        self
    }
    pub fn error(mut self, error: String) -> Self {
        self.error = Some(error);
        self
    }
    pub fn updated_at(mut self, updated_at: DateTime<Utc>) -> Self {
        self.updated_at = Some(updated_at);
        self
    }

    fn get_params_sets(&self) -> Vec<String> {
        const DEFAULT_INIT_PARAM: usize = 1;
        let (mut sets, mut get_condition_closure) = Filter::init_values_to_modify(DEFAULT_INIT_PARAM);
        if self.status.is_some() {
            sets.push(get_condition_closure("status"));
        }
        if self.error.is_some() {
            sets.push(get_condition_closure("error"));
        }
        if self.updated_at.is_some() {
            sets.push(get_condition_closure("updated_at"));
        }
        sets
    }

    fn bind_params<'a>(&'a self, mut query: Query<'a, Postgres, PgArguments>) -> Query<'a, Postgres, PgArguments> {
        if let Some(status) = &self.status {
            query = query.bind(status);
        }
        if let Some(error) = &self.error {
            query = query.bind(error);
        }
        if let Some(updated_at) = &self.updated_at {
            query = query.bind(updated_at);
        }
        query
    }
}

impl BtcIndexerWorkCheckpoint {
    const DB_NAME: &'static str = "runes_spark.btc_indexer_work_checkpoint";

    #[instrument(skip(conn), level = "trace")]
    pub async fn insert(&self, conn: &mut PersistentDbConn) -> Result<(), DbError> {
        let mut transaction = conn.begin().await?;
        sqlx::query(
            &format!(
                "INSERT INTO {DB_NAME} (uuid, status, task, created_at, callback_url, error, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
        )
            .bind(self.uuid)
            .bind(self.status)
            .bind(self.task.clone())
            .bind(self.created_at)
            .bind(&self.callback_url)
            .bind(&self.error)
            .bind(self.updated_at)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    #[instrument(skip(conn), level = "trace")]
    pub async fn remove(conn: &mut PersistentDbConn, filter: Option<&Filter>) -> Result<u64, DbError> {
        match filter {
            None => Self::remove_all(conn).await,
            Some(f) => Self::remove_with_filter(conn, f).await,
        }
    }

    pub async fn remove_all(conn: &mut PersistentDbConn) -> Result<u64, DbError> {
        let mut transaction = conn.begin().await?;
        let result = sqlx::query(&format!("DELETE FROM {DB_NAME}"))
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(result.rows_affected())
    }

    pub async fn remove_with_filter(conn: &mut PersistentDbConn, filter: &Filter) -> Result<u64, DbError> {
        let conditions = filter.get_params_sets();
        if conditions.is_empty() {
            return Self::remove_all(conn).await;
        }

        let mut transaction = conn.begin().await?;
        let sql = format!("DELETE FROM {DB_NAME} WHERE {}", conditions.join(" AND "));
        let q = sqlx::query(&sql);
        let result = filter.bind_params(q).execute(&mut *transaction).await?;
        transaction.commit().await?;

        Ok(result.rows_affected())
    }

    #[instrument(skip(conn), level = "trace")]
    pub async fn update(conn: &mut PersistentDbConn, uuid: &Uuid, update: &Update) -> Result<u64, DbError> {
        let mut transaction = conn.begin().await?;
        let sets = update.get_params_sets();
        if sets.is_empty() {
            transaction.commit().await?;
            return Ok(0);
        }

        let sql = format!(
            "UPDATE {DB_NAME} SET {} WHERE uuid = ${}",
            sets.join(", "),
            sets.len() + 1
        );
        let query = sqlx::query(&sql);
        let query = update.bind_params(query);
        let query = query.bind(uuid);

        let result = query.execute(&mut *transaction).await?;
        transaction.commit().await?;
        Ok(result.rows_affected())
    }

    #[instrument(skip(conn), level = "trace")]
    pub async fn filter(
        conn: &mut PersistentDbConn,
        filter: Option<&Filter>,
    ) -> Result<Vec<BtcIndexerWorkCheckpoint>, DbError> {
        match filter {
            None => Self::get_all(conn).await,
            Some(f) => Self::get_with_filter(conn, f).await,
        }
    }

    #[instrument(skip(conn), level = "trace")]
    pub async fn get_all(conn: &mut PersistentDbConn) -> Result<Vec<BtcIndexerWorkCheckpoint>, DbError> {
        let mut transaction = conn.begin().await?;
        let results = sqlx::query_as::<_, BtcIndexerWorkCheckpoint>(&format!(
            "SELECT uuid, status, task, created_at, callback_url, error, updated_at FROM {DB_NAME}"
        ))
        .fetch_all(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(results)
    }

    #[instrument(skip(conn), level = "trace")]
    pub async fn get_with_filter(
        conn: &mut PersistentDbConn,
        filter: &Filter,
    ) -> Result<Vec<BtcIndexerWorkCheckpoint>, DbError> {
        let conditions = filter.get_params_sets();
        if conditions.is_empty() {
            return Self::get_all(conn).await;
        }

        let sql = format!(
            "SELECT uuid, status, task, created_at, callback_url, error, updated_at FROM {DB_NAME} WHERE {}",
            conditions.join(" AND ")
        );
        let query = sqlx::query_as::<_, BtcIndexerWorkCheckpoint>(&sql);
        let query = filter.bind_params_btc_params(query);

        let mut transaction = conn.begin().await?;
        let results = query.fetch_all(&mut *transaction).await?;
        transaction.commit().await?;
        Ok(results)
    }

    #[instrument(skip(conn), level = "trace")]
    pub async fn count(conn: &mut PersistentDbConn, filter: Option<&Filter>) -> Result<u64, DbError> {
        match filter {
            None => Self::count_all(conn).await,
            Some(f) => Self::count_with_filter(conn, f).await,
        }
    }

    #[instrument(skip(conn), level = "trace")]
    pub async fn count_all(conn: &mut PersistentDbConn) -> Result<u64, DbError> {
        let mut transaction = conn.begin().await?;
        let sql = format!("SELECT COUNT(*) FROM {DB_NAME}");
        let row = sqlx::query(&sql).fetch_one(&mut *transaction).await?;
        transaction.commit().await?;
        let count: i64 = row.get(0);
        Ok(count as u64)
    }

    #[instrument(skip(conn), level = "trace")]
    pub async fn count_with_filter(conn: &mut PersistentDbConn, filter: &Filter) -> Result<u64, DbError> {
        let conditions = filter.get_params_sets();
        if conditions.is_empty() {
            return Self::count_all(conn).await;
        }
        let mut transaction = conn.begin().await?;
        let sql = format!("SELECT COUNT(*) FROM {DB_NAME} WHERE {}", conditions.join(" AND "));
        let q = sqlx::query(&sql);
        let row = filter.bind_params(q).fetch_one(&mut *transaction).await?;
        transaction.commit().await?;
        let count: i64 = row.get(0);
        Ok(count as u64)
    }
}
