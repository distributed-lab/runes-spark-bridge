use global_utils::env_parser::{EnvParser, EnvParserError};
use serde::{Deserialize, Serialize};

pub const POSTGRES_URL_ENV_NAME: &str = "DATABASE_URL";
pub const POSTGRES_USER_ENV_NAME: &str = "POSTGRES_USER";
pub const POSTGRES_PASSWORD_ENV_NAME: &str = "POSTGRES_PASSWORD";
pub const POSTGRES_HOST_ENV_NAME: &str = "POSTGRES_HOST";
pub const POSTGRES_PORT_ENV_NAME: &str = "POSTGRES_PORT";
pub const POSTGRES_DB_ENV_NAME: &str = "POSTGRES_DB";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PostgresDbCredentials {
    pub url: String,
}

struct PgUser;
struct PgPassword;
struct PgHost;
struct PgPort;
struct PgName;

impl EnvParser for PostgresDbCredentials {
    const ENV_NAME: &'static str = POSTGRES_URL_ENV_NAME;
}

impl EnvParser for PgUser {
    const ENV_NAME: &'static str = POSTGRES_USER_ENV_NAME;
}

impl EnvParser for PgPassword {
    const ENV_NAME: &'static str = POSTGRES_PASSWORD_ENV_NAME;
}

impl EnvParser for PgHost {
    const ENV_NAME: &'static str = POSTGRES_HOST_ENV_NAME;
}

impl EnvParser for PgPort {
    const ENV_NAME: &'static str = POSTGRES_PORT_ENV_NAME;
}

impl EnvParser for PgName {
    const ENV_NAME: &'static str = POSTGRES_DB_ENV_NAME;
}

impl PostgresDbCredentials {
    /// Gets url from `DATABASE_URL` env variable
    pub fn from_db_url() -> Result<Self, EnvParserError> {
        Ok(Self {
            url: PostgresDbCredentials::obtain_env_value()?,
        })
    }

    /// Gathers url format manually, where it's beneficial to contain envs separately
    pub fn from_envs() -> Result<Self, EnvParserError> {
        Ok(Self {
            url: Self::obtain_postgres_url()?,
        })
    }

    fn obtain_postgres_url() -> Result<String, EnvParserError> {
        let (user, password, host, port, db) = (
            PgUser::obtain_env_value()?,
            PgPassword::obtain_env_value()?,
            PgHost::obtain_env_value()?,
            PgPort::obtain_env_value()?,
            PgName::obtain_env_value()?,
        );
        Ok(format!("postgres://{user}:{password}@{host}:{port}/{db}"))
    }
}
