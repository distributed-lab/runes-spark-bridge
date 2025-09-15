use crate::storage::LocalDbStorage;
use async_trait::async_trait;
use frost::types::MusigId;
use gateway_utils::tweak_generation::Nonce;
use persistent_storage::error::DbError;
use serde::{Deserialize, Serialize};
use sqlx::Connection;
use sqlx::types::Json;
use tracing::instrument;

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DepositStatus {
    InitializedSparkRunes,
    ReplenishedSparkRunes,
    BridgedSparkRunes,
    InitializedRunesSpark,
    ReplenishedRunesSpark,
    BridgedRunesSpark,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct DepositAddrInfo {
    pub nonce_tweak: Vec<u8>,
    pub address: Option<String>,
    pub is_btc: bool,
    pub amount: u64,
    pub confirmation_status: DepositStatus,
}

#[async_trait]
pub trait DepositAddressStorage {
    async fn get_deposit_addr_info(&self, musig_id: &MusigId) -> Result<Option<DepositAddrInfo>, DbError>;
    async fn set_deposit_addr_info(
        &self,
        musig_id: &MusigId,
        deposit_addr_info: DepositAddrInfo,
    ) -> Result<(), DbError>;
    async fn update_confirmation_status_in_deposit_addr_info(
        &self,
        musig_id: &MusigId,
        status: DepositStatus,
    ) -> Result<(), DbError>;
    async fn get_confirmation_status_in_deposit_addr_info(
        &self,
        musig_id: &MusigId,
    ) -> Result<Option<DepositStatus>, DbError>;
    async fn update_address_in_deposit_addr_info(
        &self,
        musig_id: &MusigId,
        address: String,
        is_btc: bool,
    ) -> Result<(), DbError>;
    async fn get_address_in_deposit_addr_info(&self, musig_id: &MusigId) -> Result<(Option<String>, bool), DbError>;
    async fn update_nonce_in_deposit_addr_info(&self, musig_id: &MusigId, nonce: Nonce) -> Result<(), DbError>;
    async fn get_nonce_in_deposit_addr_info(&self, musig_id: &MusigId) -> Result<Option<Nonce>, DbError>;
    async fn update_deposit_addr_info(
        &self,
        musig_id: &MusigId,
        deposit_addr_info: DepositAddrInfo,
    ) -> Result<(), DbError>;
}

#[async_trait]
impl DepositAddressStorage for LocalDbStorage {
    #[instrument(level = "trace", skip(self), ret)]
    async fn get_deposit_addr_info(&self, musig_id: &MusigId) -> Result<Option<DepositAddrInfo>, DbError> {
        let public_key = musig_id.get_public_key();
        let rune_id = musig_id.get_rune_id();

        let result: Option<(Vec<u8>, Option<String>, bool, i32, Json<DepositStatus>)> = sqlx::query_as(
            "SELECT nonce_tweak, address, is_btc, amount, confirmation_status
            FROM gateway.deposit_address
            WHERE public_key = $1 AND rune_id = $2",
        )
        .bind(public_key.to_string())
        .bind(rune_id)
        .fetch_optional(&self.get_conn().await?)
        .await
        .map_err(|e| DbError::BadRequest(e.to_string()))?;

        Ok(result.map(
            |(nonce_tweak, address, is_btc, amount, confirmation_status)| DepositAddrInfo {
                nonce_tweak,
                address,
                is_btc,
                amount: amount as u64,
                confirmation_status: confirmation_status.0,
            },
        ))
    }

    #[instrument(level = "trace", skip(self), ret)]
    async fn set_deposit_addr_info(
        &self,
        musig_id: &MusigId,
        deposit_addr_info: DepositAddrInfo,
    ) -> Result<(), DbError> {
        let public_key = musig_id.get_public_key();
        let rune_id = musig_id.get_rune_id();

        let _ = sqlx::query(
            "INSERT INTO gateway.deposit_address (nonce_tweak, public_key, rune_id, address, is_btc, amount, confirmation_status)
            VALUES ($1, $2, $3, $4, $5, $6, $7) 
            ON CONFLICT (public_key, rune_id, nonce_tweak) DO UPDATE SET confirmation_status = $7",
        )
            .bind(deposit_addr_info.nonce_tweak)
            .bind(public_key.to_string())
            .bind(rune_id)
            .bind(deposit_addr_info.address)
            .bind(deposit_addr_info.is_btc)
            .bind(deposit_addr_info.amount as i32)
            .bind(Json(deposit_addr_info.confirmation_status))
            .execute(&self.get_conn().await?)
            .await
            .map_err(|e| DbError::BadRequest(e.to_string()))?;

        Ok(())
    }

    #[instrument(level = "trace", skip(self), ret)]
    async fn update_deposit_addr_info(
        &self,
        musig_id: &MusigId,
        deposit_addr_info: DepositAddrInfo,
    ) -> Result<(), DbError> {
        let public_key = musig_id.get_public_key();
        let rune_id = musig_id.get_rune_id();
        let mut conn = self.get_conn().await?.acquire().await?;
        let mut transaction = conn.begin().await?;

        sqlx::query("DELETE FROM gateway.deposit_address WHERE public_key = $1 AND rune_id = $2;")
            .bind(public_key.to_string())
            .bind(&rune_id)
            .execute(&mut *transaction)
            .await
            .map_err(|e| DbError::BadRequest(e.to_string()))?;

        let _ = sqlx::query(
            "INSERT INTO gateway.deposit_address (nonce_tweak, public_key, rune_id, address, is_btc, amount, confirmation_status)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (public_key, rune_id, nonce_tweak) DO UPDATE SET confirmation_status = $7",
        )
            .bind(deposit_addr_info.nonce_tweak)
            .bind(public_key.to_string())
            .bind(rune_id)
            .bind(deposit_addr_info.address)
            .bind(deposit_addr_info.is_btc)
            .bind(deposit_addr_info.amount as i32)
            .bind(Json(deposit_addr_info.confirmation_status))
            .execute(&mut *transaction)
            .await
            .map_err(|e| DbError::BadRequest(e.to_string()))?;

        transaction.commit().await?;
        Ok(())
    }

    #[instrument(level = "trace", skip(self), ret)]
    async fn update_confirmation_status_in_deposit_addr_info(
        &self,
        musig_id: &MusigId,
        status: DepositStatus,
    ) -> Result<(), DbError> {
        let public_key = musig_id.get_public_key();
        let rune_id = musig_id.get_rune_id();

        let _ = sqlx::query(
            "UPDATE gateway.deposit_address SET confirmation_status = $1 WHERE public_key = $2 AND rune_id = $3;",
        )
        .bind(Json(status))
        .bind(public_key.to_string())
        .bind(rune_id)
        .execute(&self.get_conn().await?)
        .await
        .map_err(|e| DbError::BadRequest(e.to_string()))?;

        Ok(())
    }

    #[instrument(level = "trace", skip(self), ret)]
    async fn get_confirmation_status_in_deposit_addr_info(
        &self,
        musig_id: &MusigId,
    ) -> Result<Option<DepositStatus>, DbError> {
        let public_key = musig_id.get_public_key();
        let rune_id = musig_id.get_rune_id();

        let result: Option<(Json<DepositStatus>,)> = sqlx::query_as(
            "SELECT confirmation_status FROM gateway.deposit_address WHERE public_key = $1 AND rune_id = $2;",
        )
        .bind(public_key.to_string())
        .bind(rune_id)
        .fetch_optional(&self.get_conn().await?)
        .await
        .map_err(|e| DbError::BadRequest(e.to_string()))?;

        Ok(result.map(|confirmation_status| confirmation_status.0.0))
    }

    #[instrument(level = "trace", skip(self), ret)]
    async fn update_address_in_deposit_addr_info(
        &self,
        musig_id: &MusigId,
        address: String,
        is_btc: bool,
    ) -> Result<(), DbError> {
        let public_key = musig_id.get_public_key();
        let rune_id = musig_id.get_rune_id();

        sqlx::query(
            "UPDATE gateway.deposit_address SET address = $1, is_btc = $2 WHERE public_key = $3 AND rune_id = $4;",
        )
        .bind(address)
        .bind(is_btc)
        .bind(public_key.to_string())
        .bind(rune_id)
        .execute(&self.get_conn().await?)
        .await
        .map_err(|e| DbError::BadRequest(e.to_string()))?;
        Ok(())
    }

    #[instrument(level = "trace", skip(self), ret)]
    async fn get_address_in_deposit_addr_info(&self, musig_id: &MusigId) -> Result<(Option<String>, bool), DbError> {
        let public_key = musig_id.get_public_key();
        let rune_id = musig_id.get_rune_id();

        let result: Option<(Option<String>, bool)> = sqlx::query_as(
            "SELECT address, is_btc FROM gateway.deposit_address WHERE public_key = $1 AND rune_id = $2;",
        )
        .bind(public_key.to_string())
        .bind(rune_id)
        .fetch_optional(&self.get_conn().await?)
        .await
        .map_err(|e| DbError::BadRequest(e.to_string()))?;

        Ok(result.unwrap_or((None, false)))
    }

    #[instrument(level = "trace", skip(self), ret)]
    async fn update_nonce_in_deposit_addr_info(&self, musig_id: &MusigId, nonce: Nonce) -> Result<(), DbError> {
        let public_key = musig_id.get_public_key();
        let rune_id = musig_id.get_rune_id();
        sqlx::query("UPDATE gateway.deposit_address SET nonce_tweak = $1 WHERE public_key = $2 AND rune_id = $3;")
            .bind(nonce)
            .bind(public_key.to_string())
            .bind(rune_id)
            .execute(&self.get_conn().await?)
            .await
            .map_err(|e| DbError::BadRequest(e.to_string()))?;
        Ok(())
    }

    #[instrument(level = "trace", skip(self), ret)]
    async fn get_nonce_in_deposit_addr_info(&self, musig_id: &MusigId) -> Result<Option<Nonce>, DbError> {
        let public_key = musig_id.get_public_key();
        let rune_id = musig_id.get_rune_id();
        let result: Option<(Vec<u8>,)> =
            sqlx::query_as("SELECT nonce_tweak FROM gateway.deposit_address WHERE public_key = $1 AND rune_id = $2;")
                .bind(public_key.to_string())
                .bind(rune_id)
                .fetch_optional(&self.get_conn().await?)
                .await
                .map_err(|e| DbError::BadRequest(e.to_string()))?;
        match result {
            None => Ok(None),
            Some(nonce_bytes) => {
                let nonce_len = std::mem::size_of::<Nonce>();
                if nonce_bytes.0.len() != nonce_len {
                    return Err(DbError::BadResponse(
                        "nonce_tweak must be equal to 32 bytes".to_string(),
                    ));
                }
                let mut nonce = Nonce::default();
                nonce.copy_from_slice(&nonce_bytes.0[..nonce_len]);
                Ok(Some(nonce as Nonce))
            }
        }
    }
}

#[cfg(test)]
mod test_deposit_address_info {
    use super::*;
    use bitcoin::secp256k1::PublicKey;
    use frost::traits::AggregatorMusigIdStorage;
    use frost::types::{AggregatorDkgState, AggregatorMusigIdData};
    use gateway_utils::tweak_generation::TweakGenerator;
    use global_utils::logger::{LoggerGuard, init_logger};
    use persistent_storage::init::{PostgresPool, PostgresRepo};
    use std::str::FromStr;
    use std::sync::{Arc, LazyLock};

    static TEST_LOGGER: LazyLock<LoggerGuard> = LazyLock::new(|| init_logger());
    pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn test_updating(db: PostgresPool) -> anyhow::Result<()> {
        let _ = *TEST_LOGGER;
        let storage = Arc::new(LocalDbStorage {
            postgres_repo: PostgresRepo { pool: db },
        });

        let musig_id = MusigId::User {
            user_public_key: PublicKey::from_str("038144ac71b61ab0e0a56967696a4f31a0cdd492cd3753d59aa978e0c8eaa5a60e")?,
            rune_id: "RANDOM_1D".to_string(),
        };

        storage
            .set_musig_id_data(
                &musig_id,
                AggregatorMusigIdData {
                    dkg_state: AggregatorDkgState::DkgRound1 {
                        round1_packages: Default::default(),
                    },
                },
            )
            .await?;

        // storage.set_sign_data()
        let deposit_addr_info = DepositAddrInfo {
            nonce_tweak: TweakGenerator::generate_nonce().to_vec(),
            address: Some("bc1qe9qdjtnd209a6ygxrc49j7t8hm825v322uqfay".to_string()),
            is_btc: true,
            amount: 1000,
            confirmation_status: DepositStatus::InitializedSparkRunes,
        };
        storage
            .set_deposit_addr_info(&musig_id, deposit_addr_info.clone())
            .await?;
        assert_eq!(
            storage.get_deposit_addr_info(&musig_id).await?,
            Some(deposit_addr_info.clone())
        );

        let deposit_addr_info = DepositAddrInfo {
            nonce_tweak: deposit_addr_info.nonce_tweak,
            address: Some("bc1qe9qdjtnd209a6ygxrc49j7t8hm825v322uqfay".to_string()),
            is_btc: true,
            amount: deposit_addr_info.amount,
            confirmation_status: DepositStatus::ReplenishedSparkRunes,
        };
        storage
            .set_deposit_addr_info(&musig_id, deposit_addr_info.clone())
            .await?;
        assert_eq!(storage.get_deposit_addr_info(&musig_id).await?, Some(deposit_addr_info));
        Ok(())
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn test_statuses_change(db: PostgresPool) -> anyhow::Result<()> {
        let _ = *TEST_LOGGER;
        let storage = Arc::new(LocalDbStorage {
            postgres_repo: PostgresRepo { pool: db },
        });

        let musig_id = MusigId::User {
            user_public_key: PublicKey::from_str("038144ac71b61ab0e0a56967696a4f31a0cdd492cd3753d59aa978e0c8eaa5a60e")?,
            rune_id: "RANDOM_1D".to_string(),
        };

        storage
            .set_musig_id_data(
                &musig_id,
                AggregatorMusigIdData {
                    dkg_state: AggregatorDkgState::DkgRound1 {
                        round1_packages: Default::default(),
                    },
                },
            )
            .await?;

        // storage.set_sign_data()
        let deposit_addr_info = DepositAddrInfo {
            nonce_tweak: TweakGenerator::generate_nonce().to_vec(),
            address: Some("bc1qe9qdjtnd209a6ygxrc49j7t8hm825v322uqfay".to_string()),
            is_btc: true,
            amount: 1000,
            confirmation_status: DepositStatus::InitializedSparkRunes,
        };
        storage
            .set_deposit_addr_info(&musig_id, deposit_addr_info.clone())
            .await?;
        assert_eq!(
            storage.get_deposit_addr_info(&musig_id).await?,
            Some(deposit_addr_info.clone())
        );

        storage
            .update_confirmation_status_in_deposit_addr_info(&musig_id, DepositStatus::InitializedSparkRunes)
            .await?;
        assert_eq!(
            storage.get_confirmation_status_in_deposit_addr_info(&musig_id).await?,
            Some(DepositStatus::InitializedSparkRunes)
        );
        storage
            .update_confirmation_status_in_deposit_addr_info(&musig_id, DepositStatus::ReplenishedSparkRunes)
            .await?;
        assert_eq!(
            storage.get_confirmation_status_in_deposit_addr_info(&musig_id).await?,
            Some(DepositStatus::ReplenishedSparkRunes)
        );
        storage
            .update_confirmation_status_in_deposit_addr_info(&musig_id, DepositStatus::BridgedSparkRunes)
            .await?;
        assert_eq!(
            storage.get_confirmation_status_in_deposit_addr_info(&musig_id).await?,
            Some(DepositStatus::BridgedSparkRunes)
        );
        storage
            .update_confirmation_status_in_deposit_addr_info(&musig_id, DepositStatus::InitializedRunesSpark)
            .await?;
        assert_eq!(
            storage.get_confirmation_status_in_deposit_addr_info(&musig_id).await?,
            Some(DepositStatus::InitializedRunesSpark)
        );
        storage
            .update_confirmation_status_in_deposit_addr_info(&musig_id, DepositStatus::ReplenishedRunesSpark)
            .await?;
        assert_eq!(
            storage.get_confirmation_status_in_deposit_addr_info(&musig_id).await?,
            Some(DepositStatus::ReplenishedRunesSpark)
        );
        storage
            .update_confirmation_status_in_deposit_addr_info(&musig_id, DepositStatus::BridgedRunesSpark)
            .await?;
        assert_eq!(
            storage.get_confirmation_status_in_deposit_addr_info(&musig_id).await?,
            Some(DepositStatus::BridgedRunesSpark)
        );

        Ok(())
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    async fn test_address_update(db: PostgresPool) -> anyhow::Result<()> {
        let _ = *TEST_LOGGER;
        let storage = Arc::new(LocalDbStorage {
            postgres_repo: PostgresRepo { pool: db },
        });

        let musig_id = MusigId::User {
            user_public_key: PublicKey::from_str("038144ac71b61ab0e0a56967696a4f31a0cdd492cd3753d59aa978e0c8eaa5a60e")?,
            rune_id: "RANDOM_1D".to_string(),
        };

        storage
            .set_musig_id_data(
                &musig_id,
                AggregatorMusigIdData {
                    dkg_state: AggregatorDkgState::DkgRound1 {
                        round1_packages: Default::default(),
                    },
                },
            )
            .await?;

        // storage.set_sign_data()
        let mut deposit_addr_info = DepositAddrInfo {
            nonce_tweak: TweakGenerator::generate_nonce().to_vec(),
            address: None,
            is_btc: true,
            amount: 1000,
            confirmation_status: DepositStatus::InitializedSparkRunes,
        };
        storage
            .set_deposit_addr_info(&musig_id, deposit_addr_info.clone())
            .await?;
        assert_eq!(
            storage.get_deposit_addr_info(&musig_id).await?,
            Some(deposit_addr_info.clone())
        );

        deposit_addr_info.is_btc = true;
        deposit_addr_info.address = Some("bc1qe9qdjtnd209a6ygxrc49j7t8hm825v322uqfay".to_string());
        storage
            .update_address_in_deposit_addr_info(&musig_id, deposit_addr_info.address.clone().unwrap(), true)
            .await?;

        assert_eq!(
            storage.get_deposit_addr_info(&musig_id).await?,
            Some(deposit_addr_info.clone())
        );

        Ok(())
    }
}
