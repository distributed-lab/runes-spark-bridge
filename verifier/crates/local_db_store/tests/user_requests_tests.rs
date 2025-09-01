mod utils;

mod test_user_requests {
    use global_utils::common_types::get_uuid;
    use indexer_local_db_store::schemas::runes_spark::user_request_stats::{
        Filter, StatusTransferring, Update, UserRequestStats,
    };
    use persistent_storage::init::PersistentDbConn;
    use sqlx::types::chrono::Utc;

    use crate::utils::{TEST_LOGGER, vecs_equal_unordered};

    #[sqlx::test]
    async fn test_one_inserting(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;
        let value = UserRequestStats {
            uuid: get_uuid(),
            status: StatusTransferring::Created,
            error: Some("Error....".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        value.clone().insert(&mut pool).await?;
        UserRequestStats::remove(
            &mut pool,
            Some(&Filter {
                uuid: Some(value.uuid),
                status: None,
                error: None,
            }),
        )
        .await?;
        assert_eq!(0, UserRequestStats::count(&mut pool, None).await?);
        Ok(())
    }

    #[sqlx::test]
    async fn test_removing(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;
        let value = (0..5)
            .into_iter()
            .map(|_| UserRequestStats {
                uuid: get_uuid(),
                status: StatusTransferring::Created,
                created_at: Utc::now(),
                error: None,
                updated_at: Utc::now(),
            })
            .collect::<Vec<UserRequestStats>>();
        for x in value.iter().cloned() {
            x.insert(&mut pool).await?;
        }

        for (i, el) in value.iter().enumerate().rev() {
            UserRequestStats::remove(
                &mut pool,
                Some(&Filter {
                    uuid: Some(el.uuid),
                    status: None,
                    error: None,
                }),
            )
            .await?;
            assert_eq!(i as u64, UserRequestStats::count(&mut pool, None).await?);
        }
        assert_eq!(0, UserRequestStats::remove_all(&mut pool).await?);
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("user_request_stats")))]
    async fn test_filtering_uuid(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;
        let all_entries = UserRequestStats::get_all(&mut pool).await?;

        let filter = Filter::new().uuid(all_entries[0].uuid);
        let filtered_entries_db = UserRequestStats::filter(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        assert!(vecs_equal_unordered(&filtered_entries_db, &filtered_entries_local));

        let filter = Filter::new().uuid(all_entries.last().unwrap().uuid);
        let filtered_entries_db = UserRequestStats::filter(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        assert!(vecs_equal_unordered(&filtered_entries_db, &filtered_entries_local));
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("user_request_stats")))]
    async fn test_filtering_status(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;
        let all_entries = UserRequestStats::get_all(&mut pool).await?;

        let filter = Filter::new().status(StatusTransferring::Created);
        let filtered_entries_db = UserRequestStats::filter(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        assert!(vecs_equal_unordered(&filtered_entries_db, &filtered_entries_local));

        let filter = Filter::new().status(StatusTransferring::FinishedSuccess);
        let filtered_entries_db = UserRequestStats::filter(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        assert!(vecs_equal_unordered(&filtered_entries_db, &filtered_entries_local));

        let filter = Filter::new().status(StatusTransferring::Processing);
        let filtered_entries_db = UserRequestStats::filter(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        assert!(vecs_equal_unordered(&filtered_entries_db, &filtered_entries_local));

        let filter = Filter::new().status(StatusTransferring::FinishedError);
        let filtered_entries_db = UserRequestStats::filter(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        assert!(vecs_equal_unordered(&filtered_entries_db, &filtered_entries_local));
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("user_request_stats")))]
    async fn test_updating_updated_at(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;

        for mut x in UserRequestStats::get_all(&mut pool).await? {
            let date = Utc::now();
            UserRequestStats::update(&mut pool, &x.uuid, &Update::new().updated_at(date)).await?;
            x.updated_at = date;
            let mut updated_value = UserRequestStats::get_with_filter(&mut pool, &Filter::new().uuid(x.uuid)).await?;
            let updated_value = updated_value.remove(0);
            assert_eq!(x, updated_value);
        }
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("user_request_stats")))]
    async fn test_updating_status(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;

        for mut x in UserRequestStats::get_all(&mut pool).await? {
            let status = StatusTransferring::Created;
            UserRequestStats::update(&mut pool, &x.uuid, &Update::new().status(status)).await?;
            x.status = status;
            let mut updated_value = UserRequestStats::get_with_filter(&mut pool, &Filter::new().uuid(x.uuid)).await?;
            let updated_value = updated_value.remove(0);
            assert_eq!(x, updated_value, "Updating value on Created");

            let status = StatusTransferring::Processing;
            UserRequestStats::update(&mut pool, &x.uuid, &Update::new().status(status)).await?;
            x.status = status;
            let mut updated_value = UserRequestStats::get_with_filter(&mut pool, &Filter::new().uuid(x.uuid)).await?;
            let updated_value = updated_value.remove(0);
            assert_eq!(x, updated_value, "Updating value on Processing");

            let status = StatusTransferring::FinishedError;
            UserRequestStats::update(&mut pool, &x.uuid, &Update::new().status(status)).await?;
            x.status = status;
            let mut updated_value = UserRequestStats::get_with_filter(&mut pool, &Filter::new().uuid(x.uuid)).await?;
            let updated_value = updated_value.remove(0);
            assert_eq!(x, updated_value, "Updating value on FinishedError");

            let status = StatusTransferring::FinishedSuccess;
            UserRequestStats::update(&mut pool, &x.uuid, &Update::new().status(status)).await?;
            x.status = status;
            let mut updated_value = UserRequestStats::get_with_filter(&mut pool, &Filter::new().uuid(x.uuid)).await?;
            let updated_value = updated_value.remove(0);
            assert_eq!(x, updated_value, "Updating value on FinishedSuccess");
        }
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("user_request_stats")))]
    async fn test_updating_error(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;

        for mut x in UserRequestStats::get_all(&mut pool).await? {
            let error = "Some error.........".to_string();
            UserRequestStats::update(&mut pool, &x.uuid, &Update::new().error(error.clone())).await?;
            x.error = Some(error);
            let mut updated_value = UserRequestStats::get_with_filter(&mut pool, &Filter::new().uuid(x.uuid)).await?;
            let updated_value = updated_value.remove(0);
            assert_eq!(x, updated_value, "Updating error value");
        }
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("user_request_stats")))]
    async fn test_insert_and_remove_by_filter_status(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;

        let all_entries = UserRequestStats::get_all(&mut pool).await?;
        let filter = Filter::new().status(StatusTransferring::Created);
        let count_on_db = UserRequestStats::count(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        let removed_entries = UserRequestStats::remove(&mut pool, Some(&filter)).await?;
        assert_eq!(filtered_entries_local.len(), removed_entries as usize);
        assert_eq!(count_on_db, removed_entries);

        let all_entries = UserRequestStats::get_all(&mut pool).await?;
        let filter = Filter::new().status(StatusTransferring::Processing);
        let count_on_db = UserRequestStats::count(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        let removed_entries = UserRequestStats::remove(&mut pool, Some(&filter)).await?;
        assert_eq!(filtered_entries_local.len(), removed_entries as usize);
        assert_eq!(count_on_db, removed_entries);

        let all_entries = UserRequestStats::get_all(&mut pool).await?;
        let filter = Filter::new().status(StatusTransferring::FinishedError);
        let count_on_db = UserRequestStats::count(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        let removed_entries = UserRequestStats::remove(&mut pool, Some(&filter)).await?;
        assert_eq!(filtered_entries_local.len(), removed_entries as usize);
        assert_eq!(count_on_db, removed_entries);

        let all_entries = UserRequestStats::get_all(&mut pool).await?;
        let filter = Filter::new().status(StatusTransferring::FinishedSuccess);
        let count_on_db = UserRequestStats::count(&mut pool, Some(&filter)).await?;
        let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
        let removed_entries = UserRequestStats::remove(&mut pool, Some(&filter)).await?;
        assert_eq!(filtered_entries_local.len(), removed_entries as usize);
        assert_eq!(count_on_db, removed_entries);
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("user_request_stats")))]
    async fn test_insert_and_remove_by_filter_uids(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;

        for x in UserRequestStats::get_all(&mut pool).await? {
            let all_entries = UserRequestStats::get_all(&mut pool).await?;
            let filter = Filter::new().uuid(x.uuid);
            let count_on_db = UserRequestStats::count(&mut pool, Some(&filter)).await?;
            let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
            let removed_entries = UserRequestStats::remove(&mut pool, Some(&filter)).await?;
            assert_eq!(filtered_entries_local.len(), removed_entries as usize);
            assert_eq!(count_on_db, removed_entries);
        }
        Ok(())
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("user_request_stats")))]
    async fn test_insert_and_remove_by_filter_error(mut pool: PersistentDbConn) -> anyhow::Result<()> {
        dotenv::dotenv()?;
        let _logger_guard = &*TEST_LOGGER;

        for x in UserRequestStats::get_all(&mut pool).await? {
            let all_entries = UserRequestStats::get_all(&mut pool).await?;
            let filter = Filter::new().error(x.error);
            let count_on_db = UserRequestStats::count(&mut pool, Some(&filter)).await?;
            let filtered_entries_local = filter_vec_with_filter(&all_entries, &filter);
            let removed_entries = UserRequestStats::remove(&mut pool, Some(&filter)).await?;
            assert_eq!(filtered_entries_local.len(), removed_entries as usize);
            // assert_eq!(count_on_db, removed_entries);
        }
        Ok(())
    }

    pub fn filter_vec_with_filter<'a>(
        entries: impl IntoIterator<Item = &'a UserRequestStats>,
        filter: &Filter,
    ) -> Vec<UserRequestStats> {
        entries
            .into_iter()
            .filter(|entry| {
                filter.uuid.map_or(true, |uuid| entry.uuid == uuid)
                    && filter.status.map_or(true, |status| entry.status == status)
                    && filter.error.as_ref().map_or(true, |error| entry.error == *error)
            })
            .cloned()
            .collect()
    }
}
