//! Behavior the PHP suite never covered: the uuid forms a link may use,
//! the request limit, the cross-cutting layers (open Inertia tabs, one
//! URL per page), the download's file name and the health checks.

use axum::body::Body;
use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::common::{APP_URL, HOST, TestApp, assert_json_eq, at, header_map, multipart, testlog};

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
async fn open_inertia_tabs_reload_the_page() {
    let app = TestApp::new().await;
    let response = app
        .get_with("/api/pages/home", &[("x-inertia", "true")])
        .await;
    assert_eq!(response.status, StatusCode::CONFLICT);
    assert_eq!(
        response.header("x-inertia-location"),
        Some("https://loganalyzer.eve-telescope.com/api/pages/home")
    );
    // A form sent from an old tab reloads the page it was on.
    let response = app
        .request(
            Method::POST,
            "/analyze",
            header_map(&[
                ("host", HOST),
                ("x-inertia", "true"),
                ("referer", &format!("{APP_URL}/logs/x")),
            ]),
            Body::empty(),
        )
        .await;
    assert_eq!(response.status, StatusCode::CONFLICT);
    assert_eq!(
        response.header("x-inertia-location"),
        Some("https://loganalyzer.eve-telescope.com/logs/x")
    );
    let response = app
        .request(
            Method::POST,
            "/analyze",
            header_map(&[
                ("host", HOST),
                ("x-inertia", "true"),
                ("referer", "https://evil.example/x"),
            ]),
            Body::empty(),
        )
        .await;
    assert_eq!(
        response.header("x-inertia-location"),
        Some("https://loganalyzer.eve-telescope.com/")
    );
    assert_eq!(app.count("combat_logs").await, 0);
}

#[tokio::test]
async fn trailing_slashes_and_the_front_controller_redirect() {
    let app = TestApp::new().await;
    for (path, status, location) in [
        ("/up/", StatusCode::PERMANENT_REDIRECT, "/up"),
        ("/up//?x=1", StatusCode::PERMANENT_REDIRECT, "/up?x=1"),
        ("/index.php", StatusCode::MOVED_PERMANENTLY, "/"),
        ("/index.php/", StatusCode::MOVED_PERMANENTLY, "/"),
        (
            "/index.php/logs/x?a=b",
            StatusCode::MOVED_PERMANENTLY,
            "/logs/x?a=b",
        ),
    ] {
        let response = app.get(path).await;
        assert_eq!(response.status, status, "{path}");
        assert_eq!(response.location(), location, "{path}");
    }
    assert_eq!(app.get("/index.phpx").await.status, StatusCode::NOT_FOUND);
    // A form post keeps its method through the 308.
    let response = app.post("/analyze/").await;
    assert_eq!(response.status, StatusCode::PERMANENT_REDIRECT);
    assert_eq!(response.location(), "/analyze");
}

#[tokio::test]
async fn a_download_without_a_file_name_is_combat_log_txt() {
    let app = TestApp::new().await;
    let (_, uuid) = app.create_log("Listener", "", None).await;
    std::fs::write(app.raw_path(uuid), crate::common::gzip(b"log")).expect("a raw log");
    let response = app.get(&format!("/logs/{uuid}/download")).await;
    assert_eq!(
        response.header("content-disposition"),
        Some("attachment; filename=combat-log.txt")
    );
    assert_eq!(response.body, b"log");
    assert_eq!(response.header("content-length"), Some("3"));
}

#[tokio::test]
async fn the_health_checks_answer() {
    let app = TestApp::new().await;
    let up = app.get("/up").await;
    assert_eq!(up.status, StatusCode::OK);
    assert!(up.text().contains("Application up"));
    assert!(up.text().contains("<title>EVE Log Analyzer</title>"));
    let health = app.get("/api/health").await;
    assert_json_eq(&json!({ "status": "ok", "database": "ok" }), &health.json());
}

#[tokio::test]
async fn unknown_api_paths_answer_json() {
    let app = TestApp::new().await;
    let response = app.get("/api/nope").await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert_json_eq(&json!({ "message": "Not Found" }), &response.json());
    let page = app.get("/nope").await;
    assert_eq!(page.status, StatusCode::NOT_FOUND);
    assert!(page.text().contains("Not Found"));
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
