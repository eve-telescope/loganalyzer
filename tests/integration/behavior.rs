//! Behavior around the edges: the uuid forms a link may use, the event
//! order, the request limit and odd multipart bodies, a failed save and
//! the sample logs.

use axum::body::Body;
use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::common::{TestApp, assert_json_eq, at, multipart, testlog};

#[tokio::test]
async fn a_log_is_found_by_its_uuid_in_any_case_only_in_the_canonical_form() {
    let app = TestApp::new().await;
    let uuid = app.upload_log("combat.txt", &testlog()).await.to_string();
    for path in [uuid.clone(), uuid.to_uppercase()] {
        assert_eq!(
            app.get(&format!("/api/pages/logs/{path}")).await.status,
            StatusCode::OK
        );
    }
    for path in [
        format!("%7B{uuid}%7D"),
        uuid.replace('-', ""),
        format!("urn:uuid:{uuid}"),
        uuid[..35].to_owned(),
        format!("{uuid}0"),
    ] {
        assert_eq!(
            app.get(&format!("/api/pages/logs/{path}")).await.status,
            StatusCode::NOT_FOUND,
            "{path}"
        );
        assert_eq!(
            app.get(&format!("/logs/{path}/download")).await.status,
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
}

#[tokio::test]
async fn events_come_in_the_order_they_were_logged() {
    let app = TestApp::new().await;
    let uuid = app.upload_log("combat.txt", &testlog()).await;
    let props = app.props(&format!("/logs/{uuid}")).await;
    let parsed = loganalyzer::parser::parse(&testlog());
    assert_json_eq(
        &serde_json::to_value(&parsed.events).expect("JSON"),
        at(&props, "analysis.events"),
    );
}

#[tokio::test]
async fn a_body_over_the_request_limit_gets_the_size_message() {
    let mut app = TestApp::new().await;
    app.reconfigure(|config| config.upload_max_size_mb = 1);
    let response = app.upload("big.txt", &vec![b'a'; 3 * 1024 * 1024]).await;
    assert_json_eq(
        &json!({ "log_file": "The log file field must not be greater than 1024 kilobytes." }),
        &response.errors(),
    );
    // Just under the limit, with the multipart framing, gets through.
    let response = app.upload("big.txt", &vec![b'a'; 1024 * 1024]).await;
    response.redirect();
}

#[tokio::test]
async fn a_body_that_is_not_multipart_has_no_file() {
    let app = TestApp::new().await;
    let response = app
        .send(
            Method::POST,
            "/analyze",
            Some("application/json"),
            Body::from(r#"{"log_file": "x"}"#),
        )
        .await;
    assert_json_eq(
        &json!({ "log_file": "The log file field is required." }),
        &response.errors(),
    );
}

#[tokio::test]
async fn a_cut_off_upload_failed_to_upload() {
    let app = TestApp::new().await;
    let (content_type, body) = multipart(&[], &[("log_file", "combat.txt", &testlog())]);
    let response = app
        .send(
            Method::POST,
            "/analyze",
            Some(&content_type),
            Body::from(body[..body.len() - 40].to_vec()),
        )
        .await;
    assert_json_eq(
        &json!({ "log_file": "The log file failed to upload." }),
        &response.errors(),
    );
}

#[tokio::test]
async fn the_last_log_file_field_counts() {
    let app = TestApp::new().await;
    let response = app
        .post_multipart(
            "/analyze",
            &[],
            &[
                ("log_file", "first.pdf", b"x"),
                ("log_file", "second.txt", &testlog()),
            ],
        )
        .await;
    response.redirect();
    let (.., original_filename) = app.first_log().await.expect("a log");
    assert_eq!(original_filename.as_deref(), Some("second.txt"));
}

#[tokio::test]
async fn a_failed_save_leaves_no_raw_file() {
    let app = TestApp::new().await;
    // The save fails: a trigger refuses the events.
    sqlx::query(
        "create or replace function refuse_events() returns trigger language plpgsql as
         $$ begin raise exception 'refused'; end $$",
    )
    .execute(&app.pool)
    .await
    .expect("function");
    sqlx::query(
        "create trigger refuse_events before insert on combat_events
         for each statement execute function refuse_events()",
    )
    .execute(&app.pool)
    .await
    .expect("trigger");
    let response = app.upload("combat.txt", &testlog()).await;
    sqlx::query("drop trigger refuse_events on combat_events")
        .execute(&app.pool)
        .await
        .expect("trigger goes");
    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(app.count("combat_logs").await, 0);
    assert_eq!(
        std::fs::read_dir(app.storage.path())
            .map(|entries| entries.count())
            .unwrap_or(0),
        0
    );
}

#[tokio::test]
async fn the_sample_logs_seed_once() {
    let app = TestApp::new().await;
    assert_eq!(
        loganalyzer::samples::seed(&app.state)
            .await
            .expect("seeded"),
        3
    );
    assert_eq!(
        loganalyzer::samples::seed(&app.state)
            .await
            .expect("seeded"),
        0
    );
    for sample in &loganalyzer::samples::SAMPLES {
        let props = app.props(&format!("/logs/{}", sample.uuid)).await;
        assert_eq!(at(&props, "rawLogAvailable"), &json!(true));
    }
    let listeners: Vec<String> = sqlx::query_scalar("select listener from combat_logs order by id")
        .fetch_all(&app.pool)
        .await
        .expect("logs");
    assert_eq!(listeners, ["Nicolas Kion", "Nicolas Kion", "De BuG"]);
}
