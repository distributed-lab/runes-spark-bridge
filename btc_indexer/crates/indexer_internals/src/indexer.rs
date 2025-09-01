use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use bitcoincore_rpc::{Client, RawTx, RpcApi, bitcoin, json};
use global_utils::common_types::{TxIdWrapped, UrlWrapped, get_uuid};
use indexer_config_parser::config::{BtcIndexerParams, BtcRpcCredentials};
use indexer_local_db_store::{
    PersistentRepoTrait,
    init::LocalDbIndexer,
    schemas::runes_spark::btc_indexer_work_checkpoint::{BtcIndexerWorkCheckpoint, StatusBtcIndexer, Update},
};
use sqlx::types::{Json, chrono::Utc};
use titan_client::{TitanApi, TitanClient};
use titan_types::{AddressTxOut, Transaction};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tracing::{error, instrument, log::debug, trace};
use uuid::Uuid;

use crate::api::{AccountReplenishmentEvent, BtcIndexerApi};

const BTC_INDEXER_LOG_PATH: &str = "btc_indexer";
const TX_TRACKING_LOG_PATH: &str = "btc_indexer:tx_tracking";
const ACCOUNT_TRACKING_LOG_PATH: &str = "btc_indexer:account_tracking";

pub struct BtcIndexer<C, Db> {
    btc_indexer_params: BtcIndexerParams,
    //todo: maybe move into traits?
    persistent_storage: Db,
    indexer_client: C,
    btc_core: Arc<Client>,
    cancellation_token: CancellationToken,
}

pub struct IndexerParamsWithApi<C, Db> {
    pub indexer_params: IndexerParams<Db>,
    pub titan_api_client: C,
}

pub struct IndexerParams<Db> {
    pub btc_rpc_creds: BtcRpcCredentials,
    pub db_pool: Db,
    pub btc_indexer_params: BtcIndexerParams,
}

impl BtcIndexer<TitanClient, LocalDbIndexer> {
    #[instrument(skip(params))]
    pub fn with_api(params: IndexerParams<LocalDbIndexer>) -> crate::error::Result<Self> {
        let titan_api_client = TitanClient::new(&params.btc_rpc_creds.url.to_string());
        Self::new(IndexerParamsWithApi {
            indexer_params: params,
            titan_api_client,
        })
    }
}

impl<C: Clone, Db: Clone> Clone for BtcIndexer<C, Db> {
    fn clone(&self) -> Self {
        BtcIndexer {
            btc_indexer_params: self.btc_indexer_params.clone(),
            persistent_storage: self.persistent_storage.clone(),
            indexer_client: self.indexer_client.clone(),
            btc_core: self.btc_core.clone(),
            cancellation_token: self.cancellation_token.clone(),
        }
    }
}

impl<C: TitanApi, Db: PersistentRepoTrait> BtcIndexer<C, Db> {
    #[instrument(skip(params))]
    pub fn new(params: IndexerParamsWithApi<C, Db>) -> crate::error::Result<Self> {
        let cancellation_token = CancellationToken::new();
        let btc_rpc_client = Arc::new(Client::new(
            &params.indexer_params.btc_rpc_creds.url.to_string(),
            params.indexer_params.btc_rpc_creds.get_btc_creds(),
        )?);
        debug!(
            "[Btc indexer] Initialization passed with configuration, {:?}",
            params.indexer_params.btc_indexer_params
        );
        let indexer = BtcIndexer {
            btc_indexer_params: params.indexer_params.btc_indexer_params,
            persistent_storage: params.indexer_params.db_pool,
            indexer_client: params.titan_api_client,
            btc_core: btc_rpc_client,
            cancellation_token,
        };
        Ok(indexer)
    }

    pub fn create_default_titan_api(btc_rpc_creds: BtcRpcCredentials) -> TitanClient {
        TitanClient::new(&btc_rpc_creds.url.to_string())
    }

    /// Spawns account replenishment tracking task
    #[instrument(level = "trace", skip(cancellation_token, titan_client), ret)]
    fn spawn_account_tracking_task(
        titan_client: C,
        uuid: Uuid,
        event_tx: oneshot::Sender<AccountReplenishmentEvent>,
        account_addr: String,
        update_interval_millis: u64,
        cancellation_token: CancellationToken,
    ) {
        let mut interval = tokio::time::interval(Duration::from_millis(update_interval_millis));
        let is_confirmed_outs = |tx_outs: &[AddressTxOut]| -> bool { tx_outs.iter().all(|out| out.status.confirmed) };
        tokio::spawn(async move {
            // todo: save about user that begun transaction to renew connection in bad cases
            //  change status on processing
            trace!("[{ACCOUNT_TRACKING_LOG_PATH}] Loop spawned..");
            'checking_loop: loop {
                tokio::select! {
                    _ = cancellation_token.cancelled() => {
                        debug!("[{ACCOUNT_TRACKING_LOG_PATH}] Closing [Btc indexer] account updating task, because of cancellation token");
                        break 'checking_loop;
                    },
                    _ = interval.tick() => {
                        trace!("[{ACCOUNT_TRACKING_LOG_PATH}] tick triggered");
                        match titan_client.get_address(&account_addr).await{
                            Ok(data) => {
                                trace!("[{ACCOUNT_TRACKING_LOG_PATH}] address data successfully received, {data:?}");
                                if  !data.outputs.is_empty() && is_confirmed_outs(&data.outputs) {
                                    let _ = event_tx.send(AccountReplenishmentEvent{address: account_addr,account_data:data});
                                    // todo: save data in db about processed tx
                                    break 'checking_loop;
                                }
                            }
                            Err(e) => {
                                error!("[{ACCOUNT_TRACKING_LOG_PATH}] Failed to retrieve account data by address: {e}")
                            }
                        };
                    }
                }
            }
        });
        trace!("[{ACCOUNT_TRACKING_LOG_PATH}] Account tracking task spawned");
    }

    #[instrument(level = "trace", skip(titan_client, event_tx), ret)]
    fn spawn_tx_tracking_task(
        titan_client: C,
        event_tx: oneshot::Sender<Transaction>,
        tx_id: bitcoin::Txid,
        update_interval_millis: u64,
        cancellation_token: CancellationToken,
    ) {
        let mut interval = tokio::time::interval(Duration::from_millis(update_interval_millis));
        tokio::spawn(async move {
            // todo: save about user that begun transaction to renew connection in bad cases
            'checking_loop: loop {
                tokio::select! {
                    _ = cancellation_token.cancelled() => {
                        debug!("[{TX_TRACKING_LOG_PATH}] Closing [Btc indexer] account updating task, because of cancellation token");
                        break 'checking_loop;
                    },
                    _ = interval.tick() => {
                        match titan_client.get_transaction(&tx_id).await{
                            Ok(data) => {
                                if data.status.confirmed {
                                    let _ = event_tx.send(data);
                                    // todo: save data in db
                                    break 'checking_loop;
                                }
                            }
                            Err(e) => {
                                error!("[{TX_TRACKING_LOG_PATH}] Failed to retrieve account data by address: {e}")
                            }
                        };
                    }
                }
            }
        });
        trace!("[{TX_TRACKING_LOG_PATH}] Transaction tracking task spawned");
    }

    async fn mark_entry_as_processing(&self, uuid: &Uuid) -> crate::error::Result<()> {
        {
            let mut conn = self.persistent_storage.get_conn().await?;
            BtcIndexerWorkCheckpoint::update(
                &mut conn,
                &uuid,
                &Update {
                    status: Some(StatusBtcIndexer::Processing),
                    error: None,
                    updated_at: Some(Utc::now()),
                },
            )
            .await?;
        };
        Ok(())
    }
}

#[async_trait]
impl<C: TitanApi, Db: PersistentRepoTrait> BtcIndexerApi for BtcIndexer<C, Db> {
    #[instrument(level = "debug", skip(self))]
    async fn track_tx_changes(
        &self,
        tx_id: bitcoin::Txid,
        uuid: Uuid,
    ) -> crate::error::Result<oneshot::Receiver<Transaction>> {
        let (event_tx, event_rx) = oneshot::channel::<Transaction>();
        self.mark_entry_as_processing(&uuid).await?;
        Self::spawn_tx_tracking_task(
            self.indexer_client.clone(),
            event_tx,
            tx_id,
            self.btc_indexer_params.update_interval_millis,
            self.cancellation_token.child_token(),
        );
        Ok(event_rx)
    }

    #[instrument(level = "debug", skip(self, account_id), fields(account_id=account_id.as_ref()))]
    async fn track_account_changes(
        &self,
        account_id: impl AsRef<str> + Send,
        uuid: Uuid,
    ) -> crate::error::Result<oneshot::Receiver<AccountReplenishmentEvent>> {
        let account_id = account_id.as_ref().to_string();
        let (event_tx, event_rx) = oneshot::channel::<AccountReplenishmentEvent>();
        self.mark_entry_as_processing(&uuid).await?;
        Self::spawn_account_tracking_task(
            self.indexer_client.clone(),
            uuid,
            event_tx,
            account_id,
            self.btc_indexer_params.update_interval_millis,
            self.cancellation_token.child_token(),
        );
        Ok(event_rx)
    }

    #[instrument(level = "debug", skip(self))]
    fn get_tx_info(&self, tx_id: bitcoin::Txid) -> crate::error::Result<bitcoin::transaction::Transaction> {
        Ok(self.btc_core.get_by_id(&tx_id)?)
    }

    #[instrument(level = "debug", skip(self))]
    fn get_blockchain_info(&self) -> crate::error::Result<json::GetBlockchainInfoResult> {
        Ok(self.btc_core.get_blockchain_info()?)
    }

    #[instrument(level = "debug", skip(self, tx))]
    fn broadcast_transaction(&self, tx: impl RawTx) -> crate::error::Result<bitcoin::blockdata::transaction::Txid> {
        let hex_tx = tx.raw_hex();
        trace!("[Btc indexer] broadcasting transaction, {hex_tx}");
        Ok(self.btc_core.send_raw_transaction(hex_tx)?)
    }
}

impl<C, Db> Drop for BtcIndexer<C, Db> {
    #[instrument(skip(self))]
    fn drop(&mut self) {
        debug!("[{BTC_INDEXER_LOG_PATH}] Closing indexer");
        self.cancellation_token.cancel()
    }
}
