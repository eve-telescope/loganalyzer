//! The paths the api owns besides the page props: the health checks, the
//! raw log download, the error pages and the redirects to one URL per
//! page.

use axum::body::Body;
use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::common::{HOST, TestApp, TestResponse, assert_json_eq, fixture, header_map};

async fn request(app: &TestApp, method: Method, path: &str) -> TestResponse {
    app.request(method, path, header_map(&[("host", HOST)]), Body::empty())
        .await
}

/// The minimal HTML error page with its status and message.
#[track_caller]
fn assert_error_page(response: &TestResponse, status: StatusCode, message: &str) {
    assert_eq!(response.status, status);
    assert_eq!(
        response.header("content-type"),
        Some("text/html; charset=utf-8")
    );
    assert_eq!(response.header("cache-control"), Some("no-cache, private"));
    let body = response.text();
    assert!(
        body.starts_with("<!DOCTYPE html>\n<html lang=\"en\">"),
        "{body}"
    );
    assert!(
        body.contains(&format!("<title>{message}</title>")),
        "{body}"
    );
    assert!(
        body.contains(&format!("{}                    </h1>", status.as_u16())),
        "{body}"
    );
    assert!(
        body.contains(&format!("{message}                    </div>")),
        "{body}"
    );
}

/// The plain error page of the statuses the minimal page has no design
/// for (405, 422).
#[track_caller]
fn assert_plain_error_page(response: &TestResponse, status: StatusCode, message: &str) {
    assert_eq!(response.status, status);
    assert_eq!(
        response.header("content-type"),
        Some("text/html; charset=utf-8")
    );
    assert_eq!(response.header("cache-control"), Some("no-cache, private"));
    let body = response.text();
    assert!(
        body.contains(&format!("<title>An Error Occurred: {message}</title>")),
        "{body}"
    );
    assert!(
        body.contains(&format!(
            "<h2>The server returned a \"{} {message}\".</h2>",
            status.as_u16()
        )),
        "{body}"
    );
}

/// The health page: the app name, "Application up" and the render time.
#[tokio::test]
async fn up_answers_the_health_page() {
    let app = TestApp::new().await;
    let response = app.get("/up").await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(
        response.header("content-type"),
        Some("text/html; charset=utf-8")
    );
    let body = response.text();
    for part in [
        "<!DOCTYPE html>\n<html lang=\"en\">",
        "<title>EVE Log Analyzer</title>",
        "Application up",
        "HTTP request received.",
        "Response rendered in ",
    ] {
        assert!(body.contains(part), "{part} in\n{body}");
    }
}

#[tokio::test]
async fn health_reports_ok_with_the_database() {
    let app = TestApp::new().await;
    assert_json_eq(
        &json!({ "status": "ok", "database": "ok" }),
        &app.get("/api/health").await.json(),
    );
}

#[tokio::test]
async fn unknown_paths_answer_the_not_found_page() {
    let app = TestApp::new().await;
    assert_error_page(&app.get("/nope").await, StatusCode::NOT_FOUND, "Not Found");
    let put = request(&app, Method::PUT, "/nope").await;
    assert_error_page(&put, StatusCode::NOT_FOUND, "Not Found");
}

#[tokio::test]
async fn unknown_api_paths_answer_json() {
    let app = TestApp::new().await;
    let response = app.get("/api/nope").await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert_json_eq(&json!({ "message": "Not Found" }), &response.json());
}

/// The form endpoints only take a POST.
#[tokio::test]
async fn the_form_endpoints_refuse_other_methods() {
    let app = TestApp::new().await;
    for path in [
        "/analyze",
        "/logs/00000000-0000-4000-8000-000000000000/regenerate",
    ] {
        let response = app.get(path).await;
        assert_plain_error_page(
            &response,
            StatusCode::METHOD_NOT_ALLOWED,
            "Method Not Allowed",
        );
        assert_eq!(response.header("allow"), Some("POST"), "{path}");
    }
    let head = request(&app, Method::HEAD, "/analyze").await;
    assert_eq!(head.status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(head.header("allow"), Some("POST"));
}

/// The raw log comes back as it was uploaded, as an attachment named
/// after the uploaded file, with an ASCII fallback name.
#[tokio::test]
async fn downloads_the_raw_log_under_its_file_name() {
    for (file_name, fixture_name, disposition) in [
        (
            "my-fight.txt",
            "testlog",
            "attachment; filename=my-fight.txt",
        ),
        (
            "Kämpfe.txt",
            "edge-tricky-names",
            "attachment; filename=Kampfe.txt; filename*=utf-8''K%C3%A4mpfe.txt",
        ),
        (
            "with space.txt",
            "edge-drone-attribution",
            "attachment; filename=\"with space.txt\"",
        ),
        (
            "per%cent.txt",
            "edge-empty",
            "attachment; filename=percent.txt; filename*=utf-8''per%25cent.txt",
        ),
        (
            "日本語.txt",
            "edge-multiline-then-notify",
            "attachment; filename=.txt; filename*=utf-8''%E6%97%A5%E6%9C%AC%E8%AA%9E.txt",
        ),
    ] {
        let app = TestApp::new().await;
        let contents = fixture(fixture_name);
        let uuid = app.upload_log(file_name, &contents).await;
        let response = app.get(&format!("/logs/{uuid}/download")).await;
        assert_eq!(response.status, StatusCode::OK, "{file_name}");
        assert_eq!(
            response.header("content-type"),
            Some("text/plain; charset=utf-8")
        );
        assert_eq!(response.header("cache-control"), Some("no-cache, private"));
        assert_eq!(
            response.header("content-disposition"),
            Some(disposition),
            "{file_name}"
        );
        assert_eq!(response.body, contents, "{file_name}");

        let head = request(&app, Method::HEAD, &format!("/logs/{uuid}/download")).await;
        assert_eq!(head.status, StatusCode::OK);
        assert_eq!(head.header("content-disposition"), Some(disposition));
        assert!(head.body.is_empty());
    }
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
async fn a_download_without_a_raw_log_is_not_found() {
    let app = TestApp::new().await;
    assert_error_page(
        &app.get("/logs/00000000-0000-4000-8000-000000000000/download")
            .await,
        StatusCode::NOT_FOUND,
        "Not Found",
    );
    let uuid = app.upload_log("combat.txt", &fixture("testlog")).await;
    std::fs::remove_file(app.raw_path(uuid)).expect("the raw log goes");
    assert_error_page(
        &app.get(&format!("/logs/{uuid}/download")).await,
        StatusCode::NOT_FOUND,
        "Not Found",
    );
}

#[tokio::test]
async fn a_damaged_raw_log_is_unprocessable() {
    let app = TestApp::new().await;
    let uuid = app.upload_log("combat.txt", &fixture("testlog")).await;
    std::fs::write(app.raw_path(uuid), b"not gzip at all").expect("a broken file");
    assert_plain_error_page(
        &app.get(&format!("/logs/{uuid}/download")).await,
        StatusCode::UNPROCESSABLE_ENTITY,
        "Unprocessable Content",
    );
}

/// One URL per page: a trailing slash redirects to the path without it,
/// keeping the query; a form post keeps its method through the 308.
#[tokio::test]
async fn trailing_slashes_redirect() {
    let app = TestApp::new().await;
    let uuid = app.upload_log("combat.txt", &fixture("testlog")).await;
    for (path, location) in [
        ("/up/".to_owned(), "/up".to_owned()),
        ("/up//?x=1".to_owned(), "/up?x=1".to_owned()),
        (
            format!("/logs/{uuid}/download/"),
            format!("/logs/{uuid}/download"),
        ),
    ] {
        let response = app.get(&path).await;
        assert_eq!(response.status, StatusCode::PERMANENT_REDIRECT, "{path}");
        assert_eq!(response.location(), location, "{path}");
    }
    let response = app.post("/analyze/").await;
    assert_eq!(response.status, StatusCode::PERMANENT_REDIRECT);
    assert_eq!(response.location(), "/analyze");
}
