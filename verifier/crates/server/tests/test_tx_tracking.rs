mod utils;

mod mocked_tx_tracking {
    use std::str::FromStr;

    use axum_test::TestServer;
    use bitcoin::Txid;
    use btc_indexer_internals::indexer::{BtcIndexer, IndexerParams, IndexerParamsWithApi};
    use btc_indexer_server::routes::track_tx::TrackTxRequest;
    use global_utils::common_types::{TxIdWrapped, UrlWrapped};
    use indexer_config_parser::config::{ConfigVariant, ServerConfig};
    use tracing::{info, instrument};

    use crate::utils::{
        init::{TEST_LOGGER, obtain_random_localhost_socket_addr},
        mock::{
            create_app_mocked, generate_mock_titan_indexer_tx_tracking, generate_mock_titan_indexer_wallet_tracking,
        },
        test_notifier::spawn_notify_server_track_tx,
    };

    pub async fn init_mocked_tx_tracking_test_server() -> anyhow::Result<TestServer> {
        Ok(crate::utils::mock::init_mocked_test_server(|| generate_mock_titan_indexer_tx_tracking()).await?)
    }

    #[tokio::test]
    #[instrument]
    async fn test_invocation_tx_tracking() -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;
        let test_server = init_mocked_tx_tracking_test_server().await?;
        let (url_to_listen, oneshot_chan, _notify_server) =
            spawn_notify_server_track_tx(obtain_random_localhost_socket_addr()?).await?;
        let response = test_server
            .post("/track_tx")
            .json(&TrackTxRequest {
                tx_id: TxIdWrapped(Txid::from_str(
                    "fb0c9ab881331ec7acdd85d79e3197dcaf3f95055af1703aeee87e0d853e81ec",
                )?),
                callback_url: UrlWrapped(url_to_listen),
            })
            .await;
        info!("First subscription [track_tx] response: {:?}", response);

        let result = oneshot_chan.await?;
        info!("ApiResponseOwned result: {:?}", result);
        Ok(())
    }
}
