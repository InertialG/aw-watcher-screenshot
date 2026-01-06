use aw_client_lite::AwClient;
use aw_models::Event;
use chrono::{Duration, Utc};
use serde_json::Map;

// This test requires a running aw-server at localhost:5600
#[test]
#[ignore]
fn test_full_flow() {
    let client = AwClient::new("localhost", 5600);
    let bucket_id = "test-aw-client-lite-bucket";

    // Create bucket
    let bucket = serde_json::json!({
        "client": "aw-client-lite-test",
        "hostname": "localhost",
        "id": bucket_id,
        "type": "test"
    });
    client
        .create_bucket(&bucket)
        .expect("Failed to create bucket");

    // Heartbeat
    let event = Event {
        id: None,
        timestamp: Utc::now(),
        duration: Duration::seconds(0),
        data: {
            let mut m = Map::new();
            m.insert("label".to_string(), "test-event".into());
            m
        },
    };
    client
        .heartbeat(bucket_id, &event, 5.0)
        .expect("Failed to heartbeat");

    // Get events
    let events = client
        .get_events(bucket_id, None, None, Some(10))
        .expect("Failed to get events");
    assert!(!events.is_empty());

    // Get buckets
    let buckets = client.get_buckets().expect("Failed to get buckets");
    assert!(buckets.contains_key(bucket_id));

    // Cleanup
    client
        .delete_bucket(bucket_id)
        .expect("Failed to delete bucket");
}
