mod utils;

mod test_btc_indexer_requests {
    use std::str::FromStr;

    use bitcoin::Txid;
    use global_utils::common_types::{TxIdWrapped, UrlWrapped, get_uuid};
    use indexer_local_db_store::schemas::runes_spark::btc_indexer_work_checkpoint::{
        BtcIndexerWorkCheckpoint, Filter, StatusBtcIndexer, Task, Update,
    };
    use persistent_storage::init::PersistentDbConn;
    use sqlx::{
        Row,
        types::{Json, chrono::Utc},
    };
    use url::Url;

    use crate::utils::{TEST_LOGGER, vecs_equal_unordered};

    #[sqlx::test]
    async fn test_one_inserting(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;
        let value = BtcIndexerWorkCheckpoint {
            uuid: get_uuid(),
            status: StatusBtcIndexer::Created,
            task: Json::from(Task::TrackTx(TxIdWrapped(Txid::from_str(
                "06b6af9af2e1708335add6c5e99f5ed03e26f3392ce2a3325a3aa7d5588a3983",
            )?))),
            created_at: Utc::now(),
            callback_url: UrlWrapped(Url::from_str("https://example.com/callback")?),
            error: None,
            updated_at: Utc::now(),
        };
        value.clone().insert(&mut pool).await?;
        BtcIndexerWorkCheckpoint::remove(
            &mut pool,
            Some(&Filter {
                uuid: Some(value.uuid),
                status: None,
                task: None,
                callback_url: None,
            }),
        )
        .await?;
        assert_eq!(0, BtcIndexerWorkCheckpoint::count(&mut pool, None).await?);
        Ok(())
    }

    #[sqlx::test]
    async fn test_removing(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;
        let value = (0..5)
            .into_iter()
            .map(|_| BtcIndexerWorkCheckpoint {
                uuid: get_uuid(),
                status: StatusBtcIndexer::Created,
                task: Json::from(Task::TrackTx(TxIdWrapped(
                    Txid::from_str("06b6af9af2e1708335add6c5e99f5ed03e26f3392ce2a3325a3aa7d5588a3983").unwrap(),
                ))),
                created_at: Utc::now(),
                callback_url: UrlWrapped(Url::from_str("https://example.com/callback").unwrap()),
                error: None,
                updated_at: Utc::now(),
            })
            .collect::<Vec<BtcIndexerWorkCheckpoint>>();
        for x in value.iter().cloned() {
            x.insert(&mut pool).await?;
        }

        for (i, el) in value.iter().enumerate().rev() {
            BtcIndexerWorkCheckpoint::remove(
                &mut pool,
                Some(&Filter {
                    uuid: Some(el.uuid),
                    status: None,
                    task: None,
                    callback_url: None,
                }),
            )
            .await?;
            assert_eq!(i as u64, BtcIndexerWorkCheckpoint::count(&mut pool, None).await?);
        }
        assert_eq!(0, BtcIndexerWorkCheckpoint::remove_all(&mut pool).await?);
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("btc_indexer_work_checkpoints")))]
    async fn test_filtering_callback_url(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;
        let all_entries = BtcIndexerWorkCheckpoint::get_all(&mut pool).await?;

        let filter = Filter::new().callback_url(UrlWrapped(Url::from_str("https://example.com/callback")?));
        let filtered_entries_db = BtcIndexerWorkCheckpoint::filter(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        assert!(vecs_equal_unordered(&filtered_entries_db, &filtered_entries_local));

        let filter = Filter::new()
            .callback_url(UrlWrapped(Url::from_str("https://example.com/callback")?))
            .uuid(all_entries[0].uuid);
        let filtered_entries_db = BtcIndexerWorkCheckpoint::filter(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        assert!(vecs_equal_unordered(&filtered_entries_db, &filtered_entries_local));
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("btc_indexer_work_checkpoints")))]
    async fn test_filtering_uuid(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;
        let all_entries = BtcIndexerWorkCheckpoint::get_all(&mut pool).await?;

        let filter = Filter::new().uuid(all_entries[0].uuid);
        let filtered_entries_db = BtcIndexerWorkCheckpoint::filter(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        assert!(vecs_equal_unordered(&filtered_entries_db, &filtered_entries_local));

        let filter = Filter::new().uuid(all_entries.last().unwrap().uuid);
        let filtered_entries_db = BtcIndexerWorkCheckpoint::filter(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        assert!(vecs_equal_unordered(&filtered_entries_db, &filtered_entries_local));
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("btc_indexer_work_checkpoints")))]
    async fn test_filtering_status(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;
        let all_entries = BtcIndexerWorkCheckpoint::get_all(&mut pool).await?;

        let filter = Filter::new().status(StatusBtcIndexer::Created);
        let filtered_entries_db = BtcIndexerWorkCheckpoint::filter(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        assert!(vecs_equal_unordered(&filtered_entries_db, &filtered_entries_local));

        let filter = Filter::new().status(StatusBtcIndexer::FinishedSuccess);
        let filtered_entries_db = BtcIndexerWorkCheckpoint::filter(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        assert!(vecs_equal_unordered(&filtered_entries_db, &filtered_entries_local));

        let filter = Filter::new().status(StatusBtcIndexer::Processing);
        let filtered_entries_db = BtcIndexerWorkCheckpoint::filter(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        assert!(vecs_equal_unordered(&filtered_entries_db, &filtered_entries_local));

        let filter = Filter::new().status(StatusBtcIndexer::FinishedError);
        let filtered_entries_db = BtcIndexerWorkCheckpoint::filter(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        assert!(vecs_equal_unordered(&filtered_entries_db, &filtered_entries_local));
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("btc_indexer_work_checkpoints")))]
    async fn test_filtering(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;
        let all_entries = BtcIndexerWorkCheckpoint::get_all(&mut pool).await?;

        let filter = Filter::new().task(Json::from(Task::TrackWallet(
            "bc1q32jjwe930uy6q9r4l8mcryfe78usqaj9e5ujwr".to_string(),
        )));
        let filtered_entries_db = BtcIndexerWorkCheckpoint::filter(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        assert!(vecs_equal_unordered(&filtered_entries_db, &filtered_entries_local));

        let filter = Filter::new().task(Json::from(Task::TrackTx(TxIdWrapped(Txid::from_str(
            "06b6af9af2e1708335add6c5e99f5ed03e26f3392ce2a3325a3aa7d5588a3983",
        )?))));
        let filtered_entries_db = BtcIndexerWorkCheckpoint::filter(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        assert!(vecs_equal_unordered(&filtered_entries_db, &filtered_entries_local));
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("btc_indexer_work_checkpoints")))]
    async fn test_updating_updated_at(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;

        for mut x in BtcIndexerWorkCheckpoint::get_all(&mut pool).await? {
            let date = Utc::now();
            BtcIndexerWorkCheckpoint::update(&mut pool, &x.uuid, &Update::new().updated_at(date)).await?;
            x.updated_at = date;
            let mut updated_value =
                BtcIndexerWorkCheckpoint::get_with_filter(&mut pool, &Filter::new().uuid(x.uuid)).await?;
            let updated_value = updated_value.remove(0);
            assert_eq!(x, updated_value);
        }
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("btc_indexer_work_checkpoints")))]
    async fn test_updating_status(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;

        for mut x in BtcIndexerWorkCheckpoint::get_all(&mut pool).await? {
            let status = StatusBtcIndexer::Created;
            BtcIndexerWorkCheckpoint::update(&mut pool, &x.uuid, &Update::new().status(status)).await?;
            x.status = status;
            let mut updated_value =
                BtcIndexerWorkCheckpoint::get_with_filter(&mut pool, &Filter::new().uuid(x.uuid)).await?;
            let updated_value = updated_value.remove(0);
            assert_eq!(x, updated_value, "Updating value on Created");

            let status = StatusBtcIndexer::Processing;
            BtcIndexerWorkCheckpoint::update(&mut pool, &x.uuid, &Update::new().status(status)).await?;
            x.status = status;
            let mut updated_value =
                BtcIndexerWorkCheckpoint::get_with_filter(&mut pool, &Filter::new().uuid(x.uuid)).await?;
            let updated_value = updated_value.remove(0);
            assert_eq!(x, updated_value, "Updating value on Processing");

            let status = StatusBtcIndexer::FinishedError;
            BtcIndexerWorkCheckpoint::update(&mut pool, &x.uuid, &Update::new().status(status)).await?;
            x.status = status;
            let mut updated_value =
                BtcIndexerWorkCheckpoint::get_with_filter(&mut pool, &Filter::new().uuid(x.uuid)).await?;
            let updated_value = updated_value.remove(0);
            assert_eq!(x, updated_value, "Updating value on FinishedError");

            let status = StatusBtcIndexer::FinishedSuccess;
            BtcIndexerWorkCheckpoint::update(&mut pool, &x.uuid, &Update::new().status(status)).await?;
            x.status = status;
            let mut updated_value =
                BtcIndexerWorkCheckpoint::get_with_filter(&mut pool, &Filter::new().uuid(x.uuid)).await?;
            let updated_value = updated_value.remove(0);
            assert_eq!(x, updated_value, "Updating value on FinishedSuccess");
        }
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("btc_indexer_work_checkpoints")))]
    async fn test_updating_error(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;

        for mut x in BtcIndexerWorkCheckpoint::get_all(&mut pool).await? {
            let error = "Some error.........".to_string();
            BtcIndexerWorkCheckpoint::update(&mut pool, &x.uuid, &Update::new().error(error.clone())).await?;
            x.error = Some(error);
            let mut updated_value =
                BtcIndexerWorkCheckpoint::get_with_filter(&mut pool, &Filter::new().uuid(x.uuid)).await?;
            let updated_value = updated_value.remove(0);
            assert_eq!(x, updated_value, "Updating error value");
        }
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("btc_indexer_work_checkpoints")))]
    async fn test_insert_and_remove_by_filter_status(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;

        let all_entries = BtcIndexerWorkCheckpoint::get_all(&mut pool).await?;
        let filter = Filter::new().status(StatusBtcIndexer::Created);
        let count_on_db = BtcIndexerWorkCheckpoint::count(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        let removed_entries = BtcIndexerWorkCheckpoint::remove(&mut pool, Some(&filter)).await?;
        assert_eq!(filtered_entries_local.len(), removed_entries as usize);
        assert_eq!(count_on_db, removed_entries);

        let all_entries = BtcIndexerWorkCheckpoint::get_all(&mut pool).await?;
        let filter = Filter::new().status(StatusBtcIndexer::Processing);
        let count_on_db = BtcIndexerWorkCheckpoint::count(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        let removed_entries = BtcIndexerWorkCheckpoint::remove(&mut pool, Some(&filter)).await?;
        assert_eq!(filtered_entries_local.len(), removed_entries as usize);
        assert_eq!(count_on_db, removed_entries);

        let all_entries = BtcIndexerWorkCheckpoint::get_all(&mut pool).await?;
        let filter = Filter::new().status(StatusBtcIndexer::FinishedError);
        let count_on_db = BtcIndexerWorkCheckpoint::count(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        let removed_entries = BtcIndexerWorkCheckpoint::remove(&mut pool, Some(&filter)).await?;
        assert_eq!(filtered_entries_local.len(), removed_entries as usize);
        assert_eq!(count_on_db, removed_entries);

        let all_entries = BtcIndexerWorkCheckpoint::get_all(&mut pool).await?;
        let filter = Filter::new().status(StatusBtcIndexer::FinishedSuccess);
        let count_on_db = BtcIndexerWorkCheckpoint::count(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        let removed_entries = BtcIndexerWorkCheckpoint::remove(&mut pool, Some(&filter)).await?;
        assert_eq!(filtered_entries_local.len(), removed_entries as usize);
        assert_eq!(count_on_db, removed_entries);
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("btc_indexer_work_checkpoints")))]
    async fn test_insert_and_remove_by_filter_uids(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;

        for x in BtcIndexerWorkCheckpoint::get_all(&mut pool).await? {
            let all_entries = BtcIndexerWorkCheckpoint::get_all(&mut pool).await?;
            let filter = Filter::new().uuid(x.uuid);
            let count_on_db = BtcIndexerWorkCheckpoint::count(&mut pool, Some(&filter)).await?;
            let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
            let removed_entries = BtcIndexerWorkCheckpoint::remove(&mut pool, Some(&filter)).await?;
            assert_eq!(filtered_entries_local.len(), removed_entries as usize);
            assert_eq!(count_on_db, removed_entries);
        }
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("btc_indexer_work_checkpoints")))]
    async fn test_insert_and_remove_by_filter_task(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;

        for x in BtcIndexerWorkCheckpoint::get_all(&mut pool).await? {
            let all_entries = BtcIndexerWorkCheckpoint::get_all(&mut pool).await?;
            let filter = Filter::new().task(x.task);
            let count_on_db = BtcIndexerWorkCheckpoint::count(&mut pool, Some(&filter)).await?;
            let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
            let removed_entries = BtcIndexerWorkCheckpoint::remove(&mut pool, Some(&filter)).await?;
            assert_eq!(filtered_entries_local.len(), removed_entries as usize);
            assert_eq!(count_on_db, removed_entries);
        }
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("btc_indexer_work_checkpoints")))]
    async fn test_insert_and_remove_by_filter_callback_url(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;

        for x in BtcIndexerWorkCheckpoint::get_all(&mut pool).await? {
            let all_entries = BtcIndexerWorkCheckpoint::get_all(&mut pool).await?;
            let filter = Filter::new().callback_url(x.callback_url);
            let count_on_db = BtcIndexerWorkCheckpoint::count(&mut pool, Some(&filter)).await?;
            let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
            let removed_entries = BtcIndexerWorkCheckpoint::remove(&mut pool, Some(&filter)).await?;
            assert_eq!(filtered_entries_local.len(), removed_entries as usize);
            assert_eq!(count_on_db, removed_entries);
        }
        Ok(())
    }

    pub fn filter_vec_with_filter<'a>(
        entries: impl IntoIterator<Item = &'a BtcIndexerWorkCheckpoint>,
        filter: &Filter,
    ) -> Vec<BtcIndexerWorkCheckpoint> {
        entries
            .into_iter()
            .filter(|entry| {
                filter.uuid.map_or(true, |uuid| entry.uuid == uuid)
                    && filter.status.map_or(true, |status| entry.status == status)
                    && filter.task.as_ref().map_or(true, |task| entry.task == *task)
                    && filter
                        .callback_url
                        .as_ref()
                        .map_or(true, |url| entry.callback_url == *url)
            })
            .cloned()
            .collect()
    }
}
