mod utils;

static TEST_LOGGER: LazyLock<LoggerGuard> = LazyLock::new(|| init_logger());

use std::{str::FromStr, sync::LazyLock};

use bitcoin::{BlockHash, hashes::Hash};
use bitcoincore_rpc::{RawTx, bitcoin::Txid};
use btc_indexer_internals::{
    api::BtcIndexerApi,
    indexer::{BtcIndexer, IndexerParams, IndexerParamsWithApi},
};
use global_utils::{
    common_types::get_uuid,
    logger::{LoggerGuard, init_logger},
};
use indexer_config_parser::config::{BtcRpcCredentials, ConfigVariant, ServerConfig};
use indexer_local_db_store::PostgresDbCredentials;
use titan_client::TitanApi;
use titan_types::{AddressData, AddressTxOut, SpentStatus, Transaction, TransactionStatus};
use tracing::debug;

use crate::utils::{
    common::{compare_address_tx, compare_address_tx_outs},
    mock::MockTitanIndexer,
};

mod mock_testing {
    use indexer_local_db_store::init::LocalDbIndexer;

    use super::*;

    // Test requires to run Postgres & Docker files (bitcoind + titan)
    #[ignore]
    #[tokio::test]
    async fn init_btc_indexer() -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;
        let btc_rpc_creds = BtcRpcCredentials::new()?;
        let db_pool = LocalDbIndexer::from_config(PostgresDbCredentials::from_envs()?).await?;
        let app_config = ServerConfig::init_config(ConfigVariant::Local)?;
        let indexer = BtcIndexer::with_api(IndexerParams {
            btc_rpc_creds,
            db_pool,
            btc_indexer_params: app_config.btc_indexer_config,
        })?;
        println!("Blockchain info: {:?}", indexer.get_blockchain_info()?);
        Ok(())
    }

    #[tokio::test]
    async fn test_retrieving_of_finalized_account_data() -> anyhow::Result<()> {
        const MAX_I_INDEX: u64 = 5;
        const TX_VALUE: u64 = 100;
        const ADDR: &str = "<some_account>";

        let uuid = get_uuid();

        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;
        let btc_rpc_creds = BtcRpcCredentials::new()?;
        let db_pool = LocalDbIndexer::from_config(PostgresDbCredentials::from_envs()?).await?;
        let app_config = ServerConfig::init_config(ConfigVariant::Local)?;

        let generate_utxos = |addr: &str, index: u64| {
            let generate_utxo = || AddressTxOut {
                txid: Txid::from_str("f74516e3b24af90fc2da8251d2c1e3763252b15c7aec3c1a42dde7116138caee").unwrap(),
                vout: 0,
                value: 100,
                runes: vec![],
                risky_runes: vec![],
                spent: SpentStatus::Unspent,
                status: TransactionStatus::unconfirmed(),
            };
            let mut utxos = vec![];
            // fill unconfirmed
            for j in 0..(MAX_I_INDEX - index) {
                let mut utxo = generate_utxo();
                utxo.status = TransactionStatus::unconfirmed();
                utxos.push(utxo);
            }
            // fill confirmed
            for j in 0..index {
                let mut utxo = generate_utxo();
                utxo.status = TransactionStatus::confirmed(j as u64, BlockHash::all_zeros());
                utxos.push(utxo);
            }
            utxos
        };
        let generate_mocking_invocations = |indexer: &mut MockTitanIndexer| {
            let mut i = 0;
            indexer.expect_get_address().returning(move |addr| {
                let utxos = generate_utxos(addr, i);
                i += 1;
                Ok(AddressData {
                    value: i * TX_VALUE,
                    runes: vec![],
                    outputs: utxos,
                })
            });
            indexer.expect_clone().returning(move || {
                let mut cloned_mocked_indexer = MockTitanIndexer::new();
                let mut i = 0;
                cloned_mocked_indexer.expect_get_address().returning(move |addr| {
                    let utxos = generate_utxos(addr, i);
                    i += 1;
                    Ok(AddressData {
                        value: i * TX_VALUE,
                        runes: vec![],
                        outputs: utxos,
                    })
                });
                cloned_mocked_indexer
                    .expect_clone()
                    .returning(|| MockTitanIndexer::new());
                cloned_mocked_indexer
            });
        };

        debug!("Initializing mocked indexer");
        let mut mocked_indexer = MockTitanIndexer::new();
        generate_mocking_invocations(&mut mocked_indexer);
        debug!("Building BtcIndexer...");
        let indexer = BtcIndexer::new(IndexerParamsWithApi {
            indexer_params: IndexerParams {
                btc_rpc_creds,
                db_pool,
                btc_indexer_params: app_config.btc_indexer_config,
            },
            titan_api_client: mocked_indexer,
        })?;
        debug!("Tracking account changes..");
        let oneshot = indexer.track_account_changes(ADDR, uuid).await?;
        debug!("Receiving oneshot event..");
        let result = oneshot.await?;
        debug!("Event: {result:?}");
        assert!(compare_address_tx_outs(
            &result.account_data.outputs,
            &generate_utxos(ADDR, 5)
        ));
        assert_eq!(result.address, ADDR.to_string());
        Ok(())
    }

    #[tokio::test]
    async fn test_retrieving_of_finalized_tx() -> anyhow::Result<()> {
        const MAX_I_INDEX: u64 = 5;

        let tx_id = Txid::from_str("f74516e3b24af90fc2da8251d2c1e3763252b15c7aec3c1a42dde7116138caee")?;
        let uuid = get_uuid();

        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;
        let btc_rpc_creds = BtcRpcCredentials::new()?;
        let db_pool = LocalDbIndexer::from_config(PostgresDbCredentials::from_envs()?).await?;
        let app_config = ServerConfig::init_config(ConfigVariant::Local)?;

        let generate_transaction = |tx_id: &Txid, index: u64| Transaction {
            txid: tx_id.clone(),
            version: 0,
            lock_time: 0,
            input: vec![],
            output: vec![],
            status: if index == MAX_I_INDEX {
                TransactionStatus::confirmed(index, BlockHash::all_zeros())
            } else {
                TransactionStatus::unconfirmed()
            },
            size: 0,
            weight: 0,
        };
        let generate_mocking_invocations = |indexer: &mut MockTitanIndexer| {
            let mut i = 0;
            indexer.expect_get_transaction().returning(move |tx_id| {
                let utxos = generate_transaction(tx_id, i);
                i += 1;
                Ok(generate_transaction(tx_id, i))
            });
            indexer.expect_clone().returning(move || {
                let mut cloned_mocked_indexer = MockTitanIndexer::new();
                let mut i = 0;
                cloned_mocked_indexer.expect_get_transaction().returning(move |tx_id| {
                    let utxos = generate_transaction(tx_id, i);
                    i += 1;
                    Ok(generate_transaction(tx_id, i))
                });
                cloned_mocked_indexer
                    .expect_clone()
                    .returning(|| MockTitanIndexer::new());
                cloned_mocked_indexer
            });
        };

        debug!("Initializing mocked indexer");
        let mut mocked_indexer = MockTitanIndexer::new();
        generate_mocking_invocations(&mut mocked_indexer);
        debug!("Building BtcIndexer...");
        let indexer = BtcIndexer::new(IndexerParamsWithApi {
            indexer_params: IndexerParams {
                btc_rpc_creds,
                db_pool,
                btc_indexer_params: app_config.btc_indexer_config,
            },
            titan_api_client: mocked_indexer,
        })?;
        debug!("Tracking tx changes..");
        let oneshot = indexer.track_tx_changes(tx_id, uuid).await?;
        debug!("Receiving oneshot event..");
        let result = oneshot.await?;
        debug!("Event: {result:?}");
        assert!(compare_address_tx(&result, &generate_transaction(&tx_id, MAX_I_INDEX)));
        Ok(())
    }
}
