use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, LazyLock},
};

use async_trait::async_trait;
use axum::{Router, routing::post};
use axum_test::TestServer;
use bitcoin::{BlockHash, OutPoint, hashes::Hash};
use bitcoincore_rpc::{RawTx, bitcoin::Txid};
use btc_indexer_internals::indexer::{BtcIndexer, IndexerParams, IndexerParamsWithApi};
use btc_indexer_server::AppState;
use global_utils::logger::{LoggerGuard, init_logger};
use indexer_config_parser::config::{BtcRpcCredentials, ConfigVariant, ServerConfig};
use indexer_local_db_store::{PostgresDbCredentials, init::LocalDbIndexer};
use mockall::mock;
use reqwest::header::HeaderMap;
use titan_client::{Error, TitanApi, TitanClient};
use titan_types::{
    AddressData, AddressTxOut, Block, BlockTip, InscriptionId, MempoolEntry, Pagination, PaginationResponse,
    RuneResponse, SpentStatus, Status, Subscription, Transaction, TransactionStatus, TxOut, query,
};
use tracing::{debug, info, instrument};
use utoipa_swagger_ui::SwaggerUi;

use crate::utils::init::TEST_LOGGER;

mock! {
    pub TitanIndexer {}
    impl Clone for TitanIndexer {
        fn clone(&self) -> Self;
    }

    #[async_trait]
    impl TitanApi for TitanIndexer {
        async fn get_status(&self) -> Result<Status, Error>;
        async fn get_tip(&self) -> Result<BlockTip, Error>;
        async fn get_block(&self, query: &query::Block) -> Result<Block, Error>;
        async fn get_block_hash_by_height(&self, height: u64) -> Result<String, Error>;
        async fn get_block_txids(&self, query: &query::Block) -> Result<Vec<String>, Error>;
        async fn get_address(&self, address: &str) -> Result<AddressData, Error>;
        async fn get_transaction(&self, txid: &Txid) -> Result<Transaction, Error>;
        async fn get_transaction_raw(&self, txid: &Txid) -> Result<Vec<u8>, Error>;
        async fn get_transaction_hex(&self, txid: &Txid) -> Result<String, Error>;
        async fn get_transaction_status(&self, txid: &Txid) -> Result<TransactionStatus, Error>;
        async fn send_transaction(&self, tx_hex: String) -> Result<Txid, Error>;
        async fn get_output(&self, outpoint: &OutPoint) -> Result<TxOut, Error>;
        async fn get_inscription(
            &self,
            inscription_id: &InscriptionId,
        ) -> Result<(HeaderMap, Vec<u8>), Error>;
        async fn get_runes(
            &self,
            pagination: Option<Pagination>,
        ) -> Result<PaginationResponse<RuneResponse>, Error>;
        async fn get_rune(&self, rune: &query::Rune) -> Result<RuneResponse, Error>;
        async fn get_rune_transactions(
            &self,
            rune: &query::Rune,
            pagination: Option<Pagination>,
        ) -> Result<PaginationResponse<Txid>, Error>;
        async fn get_mempool_txids(&self) -> Result<Vec<Txid>, Error>;
        async fn get_mempool_entry(&self, txid: &Txid) -> Result<MempoolEntry, Error>;
        async fn get_mempool_entries(
            &self,
            txids: &[Txid],
        ) -> Result<HashMap<Txid, Option<MempoolEntry>>, Error>;
        async fn get_all_mempool_entries(&self) -> Result<HashMap<Txid, MempoolEntry>, Error>;
        async fn get_mempool_entries_with_ancestors(
            &self,
            txids: &[Txid],
        ) -> Result<HashMap<Txid, MempoolEntry>, Error>;
        async fn get_subscription(&self, id: &str) -> Result<Subscription, Error>;
        async fn list_subscriptions(&self) -> Result<Vec<Subscription>, Error>;
        async fn add_subscription(&self, subscription: &Subscription) -> Result<Subscription, Error>;
        async fn delete_subscription(&self, id: &str) -> Result<(), Error>;
    }
}

#[instrument(level = "debug", skip(generate_mocked_titan_indexer), ret)]
pub async fn init_mocked_test_server(
    generate_mocked_titan_indexer: impl Fn() -> MockTitanIndexer,
) -> anyhow::Result<TestServer> {
    let _logger_guard = &*TEST_LOGGER;
    let (btc_creds, postgres_creds, config_variant) = (
        BtcRpcCredentials::new()?,
        PostgresDbCredentials::from_envs()?,
        ConfigVariant::Local,
    );
    let app_config = ServerConfig::init_config(config_variant)?;
    let db_pool = LocalDbIndexer::from_config(postgres_creds).await?;
    let mocked_titan_indexer = generate_mocked_titan_indexer();
    let btc_indexer = BtcIndexer::new(IndexerParamsWithApi {
        indexer_params: IndexerParams {
            btc_rpc_creds: btc_creds,
            db_pool: db_pool.clone(),
            btc_indexer_params: app_config.btc_indexer_config,
        },
        titan_api_client: mocked_titan_indexer,
    })?;
    let app = create_app_mocked(db_pool, btc_indexer).await;
    let test_server = TestServer::builder().http_transport().build(app.into_make_service())?;
    tracing::info!("Serving local axum test server on {:?}", test_server.server_address());
    Ok(test_server)
}

pub fn generate_mock_titan_indexer_tx_tracking() -> MockTitanIndexer {
    let generate_transaction = |tx_id: &Txid, index: u64| Transaction {
        txid: tx_id.clone(),
        version: 0,
        lock_time: 0,
        input: vec![],
        output: vec![],
        status: TransactionStatus::confirmed(index, BlockHash::all_zeros()),
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
    mocked_indexer
}

pub fn generate_mock_titan_indexer_wallet_tracking() -> MockTitanIndexer {
    const TX_VALUE: u64 = 100;

    let generate_transaction = |tx_id: &Txid, index: u64| Transaction {
        txid: tx_id.clone(),
        version: 0,
        lock_time: 0,
        input: vec![],
        output: vec![],
        status: TransactionStatus::confirmed(index, BlockHash::all_zeros()),
        size: 0,
        weight: 0,
    };

    let generate_utxos = |addr: &str, amount_of_utxos: u64| {
        let generate_utxo = || AddressTxOut {
            txid: Txid::from_str("f74516e3b24af90fc2da8251d2c1e3763252b15c7aec3c1a42dde7116138caee").unwrap(),
            vout: 0,
            value: 100,
            runes: vec![],
            risky_runes: vec![],
            spent: SpentStatus::Unspent,
            status: TransactionStatus::confirmed(100, BlockHash::all_zeros()),
        };
        let mut utxos = vec![];
        for j in 0..amount_of_utxos {
            let mut utxo = generate_utxo();
            utxo.status = TransactionStatus::confirmed(j as u64, BlockHash::all_zeros());
            utxos.push(utxo);
        }
        utxos
    };
    let generate_mocking_invocations = |indexer: &mut MockTitanIndexer| {
        let mut i = 3;
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
    mocked_indexer
}

#[instrument(skip(db_pool, btc_indexer))]
pub async fn create_app_mocked(
    db_pool: LocalDbIndexer,
    btc_indexer: BtcIndexer<MockTitanIndexer, LocalDbIndexer>,
) -> Router {
    let state = AppState {
        http_client: reqwest::Client::new(),
        persistent_storage: db_pool,
        btc_indexer: Arc::new(btc_indexer),
        cached_tasks: Arc::new(Default::default()),
    };
    let app = Router::new()
        .route("/track_tx", post(btc_indexer_server::routes::track_tx::handler))
        .route("/track_wallet", post(btc_indexer_server::routes::track_wallet::handler))
        .with_state(state);
    app
}
