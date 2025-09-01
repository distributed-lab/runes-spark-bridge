mod utils;

mod mocked_tx_tracking {
    use axum_test::TestServer;
    use btc_indexer_internals::indexer::{BtcIndexer, IndexerParams, IndexerParamsWithApi};
    use btc_indexer_server::routes::track_wallet::TrackWalletRequest;
    use global_utils::common_types::UrlWrapped;
    use indexer_config_parser::config::{BtcRpcCredentials, ConfigVariant, ServerConfig};
    use tracing::{info, instrument};

    use crate::utils::{
        init::{TEST_LOGGER, obtain_random_localhost_socket_addr},
        mock::{
            MockTitanIndexer, create_app_mocked, generate_mock_titan_indexer_wallet_tracking, init_mocked_test_server,
        },
        test_notifier::spawn_notify_server_track_wallet,
    };

    pub async fn init_mocked_wallet_tracking_test_server() -> anyhow::Result<TestServer> {
        Ok(init_mocked_test_server(|| generate_mock_titan_indexer_wallet_tracking()).await?)
    }

    #[tokio::test]
    #[instrument]
    async fn test_invocation_wallet_tracking() -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;
        let test_server = init_mocked_wallet_tracking_test_server().await?;
        let (url_to_listen, oneshot_chan, _notify_server) =
            spawn_notify_server_track_wallet(obtain_random_localhost_socket_addr()?).await?;
        let response = test_server
            .post("/track_wallet")
            .json(&TrackWalletRequest {
                wallet_id: "bc1qvvwhefadjpsnynen8e4n2g3tc3d3hvtraemaxw".to_string(),
                callback_url: UrlWrapped(url_to_listen),
            })
            .await;
        info!("response: {:?}", response);
        info!("First subscription [track_tx] response: {:?}", response);

        let result = oneshot_chan.await?;
        info!("ApiResponseOwned result: {:?}", result);
        Ok(())
    }
}
