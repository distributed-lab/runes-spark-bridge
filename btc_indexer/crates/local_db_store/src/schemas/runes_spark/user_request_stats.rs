use persistent_storage::error::DbError;
use persistent_storage::init::PersistentDbConn;
use serde::{Deserialize, Serialize};
use sqlx::{
    Acquire, FromRow, Postgres, Row,
    postgres::PgArguments,
    query::{Query, QueryAs},
    types::chrono::{DateTime, Utc},
};
use tracing::instrument;
use uuid::Uuid;

use crate::schemas::common::{ValuesMaxCapacity, ValuesToModifyInit};

const DB_NAME: &str = "runes_spark.user_request_stats";

#[derive(Debug, FromRow, Clone, PartialEq, Eq, Hash)]
pub struct UserRequestStats {
    pub uuid: Uuid,
    pub status: StatusTransferring,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, Copy, Eq, PartialEq, Hash)]
#[sqlx(rename_all = "snake_case", type_name = "STATUS_TRANSFERRING")]
pub enum StatusTransferring {
    Created,
    Processing,
    FinishedSuccess,
    FinishedError,
}

#[derive(Debug, Clone, Default)]
pub struct Update {
    pub status: Option<StatusTransferring>,
    pub error: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub uuid: Option<Uuid>,
    pub status: Option<StatusTransferring>,
    pub error: Option<Option<String>>,
}

impl ValuesMaxCapacity for Update {
    const MAX_CAPACITY: usize = 3;
}
impl ValuesMaxCapacity for Filter {
    const MAX_CAPACITY: usize = 3;
}

impl<'a> Filter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn uuid(mut self, uuid: Uuid) -> Self {
        self.uuid = Some(uuid);
        self
    }

    pub fn status(mut self, status: StatusTransferring) -> Self {
        self.status = Some(status);
        self
    }

    pub fn error(mut self, error: Option<String>) -> Self {
        self.error = Some(error);
        self
    }

    fn get_params_sets(&'a self) -> Vec<String> {
        const DEFAULT_INIT_PARAM: usize = 1;
        let (mut conditions, mut get_condition_closure) = Filter::init_values_to_modify(DEFAULT_INIT_PARAM);
        if self.uuid.is_some() {
            conditions.push(get_condition_closure("uuid"));
        }
        if self.status.is_some() {
            conditions.push(get_condition_closure("status"));
        }
        if let Some(err) = self.error.as_ref() {
            match err {
                None => conditions.push("error IS NULL".to_string()),
                Some(_) => {
                    conditions.push(get_condition_closure("error"));
                }
            }
        }
        conditions
    }

    fn bind_params(&'a self, mut query: Query<'a, Postgres, PgArguments>) -> Query<'a, Postgres, PgArguments> {
        if let Some(uuid) = self.uuid {
            query = query.bind(uuid);
        }
        if let Some(status) = self.status {
            query = query.bind(status);
        }
        if let Some(error) = &self.error
            && error.is_some()
        {
            query = query.bind(error);
        }
        query
    }

    fn bind_params_user_req_stats(
        &'a self,
        mut query: QueryAs<'a, Postgres, UserRequestStats, PgArguments>,
    ) -> QueryAs<'a, Postgres, UserRequestStats, PgArguments> {
        if let Some(uuid) = self.uuid {
            query = query.bind(uuid);
        }
        if let Some(status) = self.status {
            query = query.bind(status);
        }
        if let Some(error) = &self.error {
            query = query.bind(error);
        }
        query
    }
}

impl<'a> Update {
    fn get_params_sets(&'a self) -> Vec<String> {
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

    fn bind_params(&'a self, mut query: Query<'a, Postgres, PgArguments>) -> Query<'a, Postgres, PgArguments> {
        if let Some(status) = self.status {
            query = query.bind(status);
        }
        if let Some(error) = &self.error {
            query = query.bind(error);
        }
        if let Some(updated_at) = self.updated_at {
            query = query.bind(updated_at);
        }
        query
    }

    pub fn new() -> Self {
        Self::default()
    }
    pub fn status(mut self, status: StatusTransferring) -> Self {
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
}

impl UserRequestStats {
    #[instrument(skip(conn), level = "trace")]
    pub async fn insert(self, conn: &mut PersistentDbConn) -> Result<(), DbError> {
        let mut transaction = conn.begin().await?;
        sqlx::query(&format!(
            "INSERT INTO {DB_NAME} (uuid, status, error, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)"
        ))
        .bind(self.uuid)
        .bind(self.status)
        .bind(self.error)
        .bind(self.created_at)
        .bind(self.updated_at)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    #[instrument(skip(conn), level = "trace")]
    pub async fn update(conn: &mut PersistentDbConn, uuid: &Uuid, update: &Update) -> Result<u64, DbError> {
        let sets = update.get_params_sets();
        if sets.is_empty() {
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

        let mut transaction = conn.begin().await?;
        let result = query.execute(&mut *transaction).await?;
        transaction.commit().await?;
        Ok(result.rows_affected())
    }

    #[instrument(skip(conn), level = "trace")]
    pub async fn remove(conn: &mut PersistentDbConn, filter: Option<&Filter>) -> Result<u64, DbError> {
        match filter {
            None => Self::remove_all(conn).await,
            Some(f) => Self::remove_with_filter(conn, f).await,
        }
    }

    #[instrument(skip(conn), level = "trace")]
    pub async fn remove_all(conn: &mut PersistentDbConn) -> Result<u64, DbError> {
        let mut transaction = conn.begin().await?;
        let result = sqlx::query(&format!("DELETE FROM {DB_NAME}"))
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(result.rows_affected())
    }

    #[instrument(skip(conn), level = "trace")]
    pub async fn remove_with_filter(conn: &mut PersistentDbConn, filter: &Filter) -> Result<u64, DbError> {
        let conditions = filter.get_params_sets();
        if conditions.is_empty() {
            return Self::remove_all(conn).await;
        }

        let sql = format!("DELETE FROM {DB_NAME} WHERE {}", conditions.join(" AND "));
        let query = sqlx::query(&sql);
        let query = filter.bind_params(query);
        let mut transaction = conn.begin().await?;
        let result = query.execute(&mut *transaction).await?;
        transaction.commit().await?;
        Ok(result.rows_affected())
    }

    #[instrument(skip(conn), level = "trace")]
    pub async fn filter(
        conn: &mut PersistentDbConn,
        filter: Option<&Filter>,
    ) -> Result<Vec<UserRequestStats>, DbError> {
        match filter {
            None => Self::get_all(conn).await,
            Some(f) => Self::get_with_filter(conn, f).await,
        }
    }

    #[instrument(skip(conn), level = "trace")]
    pub async fn get_all(conn: &mut PersistentDbConn) -> Result<Vec<UserRequestStats>, DbError> {
        let mut transaction = conn.begin().await?;
        let results = sqlx::query_as::<_, UserRequestStats>(&format!(
            "SELECT uuid, status, error, created_at, updated_at FROM {DB_NAME}"
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
    ) -> Result<Vec<UserRequestStats>, DbError> {
        let conditions = filter.get_params_sets();
        if conditions.is_empty() {
            return Self::get_all(conn).await;
        }

        let sql = format!(
            "SELECT uuid, status, error, created_at, updated_at FROM {DB_NAME} WHERE {}",
            conditions.join(" AND ")
        );
        let query = sqlx::query_as::<_, UserRequestStats>(&sql);
        let query = filter.bind_params_user_req_stats(query);

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
