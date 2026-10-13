//! The harness itself: the mock ESI answers from its table and records
//! what it was asked, and every app starts on an empty database.

use serde_json::json;

use crate::common::esi::answer;
use crate::common::{MockEsi, TestApp};

#[tokio::test]
async fn the_mock_esi_answers_from_its_table_and_records_requests() {
    let esi = MockEsi::start().await;
    let response = reqwest::Client::new()
        .post(format!("{}/universe/ids/", esi.url()))
        .header("user-agent", "harness")
        .json(&json!(["aria vex", "Drake", "Hammerhead II", "Nobody"]))
        .send()
        .await
        .expect("the mock answers");
    let body: serde_json::Value = response.json().await.expect("JSON");
    assert_eq!(
        body,
        json!({
            "characters": [
                { "id": 93000001, "name": "Aria Vex" },
                { "id": 95194769, "name": "Hammerhead II" },
            ],
            "inventory_types": [
                { "id": 24698, "name": "Drake" },
                { "id": 2185, "name": "Hammerhead II" },
            ],
        })
    );
    let requests = esi.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].names,
        ["aria vex", "Drake", "Hammerhead II", "Nobody"]
    );
    assert_eq!(requests[0].user_agent.as_deref(), Some("harness"));
    assert_eq!(
        answer(&json!({ "characters": {} }), &["x".to_owned()]),
        json!({})
    );

    esi.fail_next(&[503]);
    let failed = reqwest::Client::new()
        .post(format!("{}/universe/ids/", esi.url()))
        .json(&json!(["Drake"]))
        .send()
        .await
        .expect("the mock answers");
    assert_eq!(failed.status(), 503);
}

#[tokio::test]
async fn every_app_starts_on_an_empty_database() {
    let app = TestApp::new().await;
    for table in ["combat_logs", "combat_events", "eve_entities"] {
        assert_eq!(app.count(table).await, 0, "{table}");
    }
    assert!(app.storage.path().exists());
}
