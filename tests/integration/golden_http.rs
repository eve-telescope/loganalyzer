//! The answers of the Laravel reference that are not Inertia pages
//! (tests/fixtures/golden/http): the health page, the error pages, the
//! raw log downloads with their headers and bytes, and the static files,
//! against what the api (and frontend/static) answer now.

use axum::http::{Method, StatusCode};
use serde_json::Value;

use crate::common::golden::{RECORDED_LOGS, http, read_text, recorded, recorded_uuids};
use crate::common::{HOST, TestApp, TestResponse, fixture, header_map, repo_path};

fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

async fn request(app: &TestApp, method: &str, path: &str) -> TestResponse {
    app.request(
        method.parse::<Method>().expect("a method"),
        path,
        header_map(&[("host", HOST)]),
        axum::body::Body::empty(),
    )
    .await
}

/// The recorded answer and the api's for the same request, with the
/// recorded log of the path uploaded again.
async fn replay(name: &str) -> (Value, TestResponse, TestApp, String) {
    let recorded = http(name);
    let app = TestApp::new().await;
    let mut path = recorded["path"].as_str().expect("a path").to_owned();
    let uuids = recorded_uuids();
    for (key, file_name, fixture_name) in RECORDED_LOGS {
        let recorded_uuid = uuids[key].as_str().expect("a uuid");
        if path.contains(recorded_uuid) {
            let uuid = app.upload_log(file_name, &fixture(fixture_name)).await;
            path = path.replace(recorded_uuid, &uuid.to_string());
            match name {
                "download-without-raw-file" => {
                    std::fs::remove_file(app.raw_path(uuid)).expect("the raw log goes");
                }
                "download-corrupt" => {
                    std::fs::write(app.raw_path(uuid), b"not gzip at all").expect("a broken file");
                }
                _ => {}
            }
        }
    }
    let method = recorded["method"].as_str().expect("a method");
    let response = request(&app, method, &path).await;
    (recorded, response, app, path)
}

fn header<'a>(recorded: &'a Value, name: &str) -> Option<&'a str> {
    recorded["headers"][name].as_str()
}

/// Same status, content type and the headers that matter, same body.
async fn same_answer(name: &str) {
    let (recorded, response, ..) = replay(name).await;
    assert_eq!(response.status.as_u16(), recorded["status"], "{name}");
    assert_eq!(
        response.header("content-type"),
        recorded["content_type"].as_str(),
        "{name}"
    );
    for key in ["content-disposition", "cache-control", "allow"] {
        assert_eq!(
            response.header(key),
            header(&recorded, key),
            "{name}: {key}"
        );
    }
    assert_eq!(response.body.len() as u64, recorded["length"], "{name}");
    assert_eq!(sha256(&response.body), recorded["sha256"], "{name}");
}

macro_rules! same_answer {
    ($($test:ident => $name:literal),* $(,)?) => {
        $(#[tokio::test] async fn $test() { same_answer($name).await; })*
    };
}

same_answer! {
    not_found => "not-found",
    analyze_get => "analyze-get",
    storage_get => "storage-get",
    regenerate_get => "regenerate-get",
    download_testlog => "download-testlog",
    download_tricky => "download-tricky",
    download_drones => "download-drones",
    download_empty => "download-empty",
    download_unknown => "download-unknown",
    download_without_raw_file => "download-without-raw-file",
}

/// The health page, apart from the time it took to render.
#[tokio::test]
async fn up() {
    let (recorded, response, ..) = replay("up").await;
    assert_eq!(response.status.as_u16(), recorded["status"]);
    let millis = |html: &str| -> String {
        let start = html.find("rendered in ").expect("a time") + "rendered in ".len();
        let end = start + html[start..].find("ms").expect("ms");
        html[..start].to_owned() + "N" + &html[end..]
    };
    assert_eq!(
        millis(&response.text()),
        millis(&read_text("golden/http/up.html"))
    );
}

#[tokio::test]
async fn analyze_head() {
    let (recorded, response, ..) = replay("analyze-head").await;
    assert_eq!(response.status.as_u16(), recorded["status"]);
    assert_eq!(response.header("allow"), header(&recorded, "allow"));
}

/// Laravel's PUT route of the local disk answered 404 without a signed
/// link; nothing is served from `/storage` at all now.
#[tokio::test]
async fn storage_put() {
    let (recorded, response, ..) = replay("storage-put").await;
    assert_eq!(recorded["status"], 404);
    assert_eq!(response.status, StatusCode::NOT_FOUND);
}

/// `/api` did not exist on the Laravel site; it is the api's prefix now.
#[tokio::test]
async fn api_unknown() {
    let (recorded, response, ..) = replay("api-unknown").await;
    assert_eq!(recorded["status"], 404);
    assert_eq!(response.status, StatusCode::OK);
}

#[tokio::test]
async fn download_head() {
    let (recorded, response, ..) = replay("download-head").await;
    assert_eq!(response.status.as_u16(), recorded["status"]);
    for key in ["content-disposition", "cache-control"] {
        assert_eq!(response.header(key), header(&recorded, key), "{key}");
    }
    assert!(response.body.is_empty());
}

/// FIXES.md, "Non-ASCII file names got a different ASCII fallback": the
/// browser uses `filename*`, which is the same; the fallback differs
/// where Laravel's transliteration table had no entry.
#[tokio::test]
async fn download_multiline() {
    let (recorded, response, ..) = replay("download-multiline").await;
    assert_eq!(
        response.header("content-disposition"),
        header(&recorded, "content-disposition")
    );
    assert_eq!(sha256(&response.body), recorded["sha256"]);
}

/// FIXES.md (deliberate changes), "One URL per page".
#[tokio::test]
async fn download_trailing_slash() {
    let (recorded, response, _, path) = replay("download-trailing-slash").await;
    assert_eq!(recorded["status"], 200);
    assert_eq!(response.status, StatusCode::PERMANENT_REDIRECT);
    assert_eq!(response.location(), path.trim_end_matches('/'));
}

/// FIXES.md, "A damaged raw log answered 500".
#[tokio::test]
async fn download_corrupt() {
    let (recorded, response, ..) = replay("download-corrupt").await;
    assert_eq!(recorded["status"], 500);
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.text().contains("422 Unprocessable Content"));
}

/// frontend/static serves the files of Laravel's public/ byte for byte.
#[test]
fn static_files() {
    for name in recorded("http") {
        let Some(file) = name.strip_prefix("static-") else {
            continue;
        };
        let recorded = http(&name);
        assert_eq!(recorded["status"], 200);
        let bytes = std::fs::read(repo_path(&format!("frontend/static/{file}")))
            .unwrap_or_else(|error| panic!("frontend/static/{file}: {error}"));
        assert_eq!(sha256(&bytes), recorded["sha256"], "{file}");
    }
}

#[test]
fn every_recorded_answer_has_a_test() {
    let source = include_str!("golden_http.rs");
    for name in recorded("http") {
        if name.starts_with("static-") {
            continue;
        }
        let test = name.replace('-', "_");
        assert!(
            source.contains(&format!("\"{name}\"")) && source.contains(&test),
            "http/{name}.json has no test"
        );
    }
}
