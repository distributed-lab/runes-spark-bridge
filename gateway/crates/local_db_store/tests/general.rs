use gateway_local_db_store::{
    schemas::request::{Request, RequestStorage},
    schemas::user_state::{Key, KeyStorage},
};
use global_utils::common_types::get_uuid;
use persistent_storage::config::PostgresDbCredentials;
use persistent_storage::init::PostgresRepo;
use tokio;

#[tokio::test]
async fn test() {
    let url = "postgresql://postgres:postgres@localhost:5433/postgres".to_string();
    let storage = PostgresRepo::from_config(PostgresDbCredentials { url }).await.unwrap();

    let key_id = get_uuid();
    let key = Key { key_id };

    storage.insert_key(key).await.unwrap();

    let key = storage.get_key(key_id).await.unwrap();
    assert_eq!(key.key_id, key_id);

    let request_id = get_uuid();
    let request = Request { request_id, key_id };

    storage.insert_request(request).await.unwrap();

    let request = storage.get_request(request_id).await.unwrap();
    assert_eq!(request.request_id, request_id);
    assert_eq!(request.key_id, key_id);
}
