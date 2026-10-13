//! The harness itself: every route of the Laravel app is registered
//! (tests/fixtures/routes.json), the mock ESI answers from its table and
//! records what it was asked, and every app starts on an empty database.

use axum::body::Body;
use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::common::esi::answer;
use crate::common::golden::read_json;
use crate::common::{HOST, MockEsi, TestApp, header_map};

#[tokio::test]
async fn every_laravel_route_is_registered() {
    let app = TestApp::new().await;
    let uuid = "5a3b6e1c-0d4f-4a2b-8c7d-1e2f3a4b5c01";
    for route in read_json("routes.json").as_array().expect("routes") {
        let uri = route["uri"].as_str().expect("a uri");
        if uri.starts_with("storage/") {
            continue;
        }
        let path = match format!("/{}", uri.trim_start_matches('/')).as_str() {
            "/" => "/api/pages/home".to_owned(),
            "/logs/{combatLog}" => format!("/api/pages/logs/{uuid}"),
            other => other.replace("{combatLog}", uuid),
        };
        for method in route["method"].as_str().expect("methods").split('|') {
            let response = app
                .request(
                    method.parse::<Method>().expect("a method"),
                    &path,
                    header_map(&[("host", HOST)]),
                    Body::empty(),
                )
                .await;
            assert!(
                response.status != StatusCode::METHOD_NOT_ALLOWED
                    && (response.status != StatusCode::NOT_FOUND || path.contains(uuid)),
                "{method} {path}: {}",
                response.status
            );
        }
    }
}

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
