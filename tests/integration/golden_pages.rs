//! Every page the Laravel reference answered (tests/fixtures/golden/pages)
//! against the props the api serves for it: the recorded log uploaded
//! again with the same file name and bytes, the same query, compared
//! with the normalizations of `common::golden`. The differences FIXES.md
//! lists are pinned as they are now.

use axum::http::StatusCode;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::common::golden::{RECORDED_LOGS, normalize_props, page, recorded, recorded_uuids};
use crate::common::{TestApp, TestResponse, assert_json_eq, fixture};

/// The recorded log a page path names, uploaded again; the path with the
/// replay's uuid in place of the recorded one (in every spelling the
/// recording used).
async fn replay_path(app: &TestApp, path: &str) -> String {
    let uuids = recorded_uuids();
    for (key, file_name, fixture_name) in RECORDED_LOGS {
        let recorded = uuids[key].as_str().expect("a uuid");
        let spellings = [
            recorded.to_owned(),
            recorded.to_uppercase(),
            recorded.replace('-', ""),
        ];
        if spellings
            .iter()
            .any(|spelling| path.contains(spelling.as_str()))
        {
            let uuid = app.upload_log(file_name, &fixture(fixture_name)).await;
            let fresh = uuid.to_string();
            return path
                .replace(recorded, &fresh)
                .replace(&recorded.to_uppercase(), &fresh.to_uppercase())
                .replace(&recorded.replace('-', ""), &fresh.replace('-', ""));
        }
    }
    path.to_owned()
}

fn uuid_in(path: &str) -> Uuid {
    path.trim_start_matches("/logs/")
        .split(['/', '?'])
        .next()
        .and_then(|uuid| uuid.parse().ok())
        .expect("a uuid in the path")
}

/// N2: the recorded uuid in props stands for the replay's.
fn with_uuid(props: &Value, uuid: Uuid) -> Value {
    let mut props = normalize_props(props);
    if props.get("uuid").is_some() {
        props["uuid"] = json!(uuid.to_string());
    }
    props
}

async fn props_of(app: &TestApp, path: &str) -> TestResponse {
    app.get(&format!("/api/pages{path}")).await
}

/// The recorded page and what the api answers for it now.
async fn replay(name: &str) -> (Value, String, TestResponse, TestApp) {
    let recorded = page(name);
    let app = TestApp::new().await;
    let path = replay_path(&app, recorded["path"].as_str().expect("a path")).await;
    if name == "log-without-raw-file" {
        std::fs::remove_file(app.raw_path(uuid_in(&path))).expect("the raw log goes");
    }
    if name == "log-corrupt-raw-file" {
        std::fs::write(app.raw_path(uuid_in(&path)), b"not gzip at all").expect("a broken file");
    }
    let response = if path == "/" {
        props_of(&app, "/home").await
    } else {
        props_of(&app, &path).await
    };
    (recorded, path, response, app)
}

/// The page answered as Laravel answered it.
async fn same_page(name: &str) {
    let (recorded, path, response, _app) = replay(name).await;
    assert_eq!(recorded["status"], 200, "{recorded}");
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let expected = if path == "/" {
        normalize_props(&recorded["page"]["props"])
    } else {
        with_uuid(&recorded["page"]["props"], uuid_in(&path))
    };
    assert_json_eq(&expected, &response.json());
}

async fn not_found(name: &str) {
    let (recorded, _, response, _app) = replay(name).await;
    assert_eq!(recorded["status"], 404);
    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert_json_eq(&json!({ "message": "Not Found" }), &response.json());
}

macro_rules! same_page {
    ($($test:ident => $name:literal),* $(,)?) => {
        $(#[tokio::test] async fn $test() { same_page($name).await; })*
    };
}

same_page! {
    home => "home",
    log_testlog => "log-testlog",
    log_testlog2 => "log-testlog2",
    log_testlog3 => "log-testlog3",
    log_hammerhead => "log-hammerhead",
    log_tricky => "log-tricky",
    log_drones => "log-drones",
    log_empty => "log-empty",
    log_multiline => "log-multiline",
    log_filters => "log-filters",
    log_filters_trimmed => "log-filters-trimmed",
    log_filters_unicode_space => "log-filters-unicode-space",
    log_filters_repeated => "log-filters-repeated",
    log_filters_plus => "log-filters-plus",
    log_filters_only_zero => "log-filters-only-zero",
    log_uppercase_uuid => "log-uppercase-uuid",
    log_without_raw_file => "log-without-raw-file",
    log_corrupt_raw_file => "log-corrupt-raw-file",
}

#[tokio::test]
async fn log_unknown() {
    not_found("log-unknown").await;
}

#[tokio::test]
async fn log_malformed() {
    not_found("log-malformed").await;
}

#[tokio::test]
async fn log_braces() {
    not_found("log-braces").await;
}

#[tokio::test]
async fn log_no_hyphens() {
    not_found("log-no-hyphens").await;
}

/// FIXES.md, "A link with a space after the uuid opened the log":
/// MySQL ignored trailing spaces in the comparison.
#[tokio::test]
async fn log_trailing_space() {
    let (recorded, _, response, _app) = replay("log-trailing-space").await;
    assert_eq!(recorded["status"], 200);
    assert_eq!(response.status, StatusCode::NOT_FOUND);
}

/// FIXES.md (deliberate changes), "One URL per page".
#[tokio::test]
async fn log_trailing_slash() {
    let (recorded, path, response, _app) = replay("log-trailing-slash").await;
    assert_eq!(recorded["status"], 200);
    assert_eq!(response.status, StatusCode::PERMANENT_REDIRECT);
    assert_eq!(
        response.location(),
        format!("/api/pages{}", path.trim_end_matches('/'))
    );
}

/// FIXES.md (deliberate changes), "One URL per page": the old front
/// controller path goes to the home page.
#[tokio::test]
async fn index_php() {
    let recorded = page("index-php");
    assert_eq!(recorded["status"], 200);
    assert_eq!(recorded["page"]["component"], "Upload");
    let app = TestApp::new().await;
    let response = app.get("/index.php").await;
    assert_eq!(response.status, StatusCode::MOVED_PERMANENTLY);
    assert_eq!(response.location(), "/");
}

/// FIXES.md, "Array query parameters broke the analysis page": Laravel
/// passed `pilot[]=x` through as an array.
#[tokio::test]
async fn log_filters_array() {
    let (recorded, path, response, _app) = replay("log-filters-array").await;
    assert_eq!(
        recorded["page"]["props"]["filters"],
        json!({ "from": { "x": "1" }, "to": null, "pilot": ["Aria Vex"], "hide": [] })
    );
    let mut expected = with_uuid(&recorded["page"]["props"], uuid_in(&path));
    expected["filters"] = json!({ "from": null, "to": null, "pilot": null, "hide": [] });
    assert_json_eq(&expected, &response.json());
}

/// FIXES.md, "Array query parameters broke the analysis page":
/// `hide[]=x` answered 500.
#[tokio::test]
async fn log_filters_hide_array() {
    let (recorded, path, response, app) = replay("log-filters-hide-array").await;
    assert_eq!(recorded["status"], 500);
    let plain = props_of(&app, &format!("/logs/{}", uuid_in(&path))).await;
    assert_json_eq(&plain.json(), &response.json());
}

#[test]
fn every_recorded_page_has_a_test() {
    let source = include_str!("golden_pages.rs");
    for name in recorded("pages") {
        assert!(
            source.contains(&format!("\"{name}\"")),
            "pages/{name}.json has no test"
        );
    }
}
