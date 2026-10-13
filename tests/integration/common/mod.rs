//! Shared harness for the integration suites: the real router on the test
//! database, a mock ESI on an ephemeral port, a temporary directory for
//! the raw logs, request helpers and the factories the PHP tests used.

// Not every suite uses every helper.
#![allow(dead_code)]

pub mod esi;
pub mod golden;

use std::path::{Path, PathBuf};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use loganalyzer::config::{Config, EsiConfig};
use loganalyzer::server::AppState;

pub use esi::MockEsi;

/// The site URL, as the reference recorded it (scripts/laravel-reference/env).
pub const APP_URL: &str = "https://loganalyzer.eve-telescope.com";
pub const HOST: &str = "loganalyzer.eve-telescope.com";
pub const USER_AGENT: &str = "EVE Log Analyzer/1.0 (+https://loganalyzer.eve-telescope.com; +https://github.com/eve-telescope/loganalyzer)";

pub fn repo_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

pub fn fixture(name: &str) -> Vec<u8> {
    let path = repo_path(&format!("tests/fixtures/parser/{name}.txt"));
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The fixture the PHP upload tests used.
pub fn testlog() -> Vec<u8> {
    fixture("testlog")
}

/// One app under test: its router, the database it writes to, the mock
/// ESI it asks and the directory of its raw logs.
pub struct TestApp {
    pub router: Router,
    pub state: AppState,
    pub pool: PgPool,
    pub esi: MockEsi,
    pub config: Config,
    pub storage: tempfile::TempDir,
}

pub async fn test_pool() -> PgPool {
    loganalyzer::db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`")
}

/// Clears every table; ids restart at 1, like Laravel's RefreshDatabase.
pub async fn reset_database(pool: &PgPool) {
    sqlx::query("truncate combat_events, combat_logs, eve_entities restart identity cascade")
        .execute(pool)
        .await
        .expect("tables truncate");
}

pub fn test_config(esi: &MockEsi, storage: &Path) -> Config {
    Config {
        app_name: "EVE Log Analyzer".to_owned(),
        app_url: APP_URL.to_owned(),
        log_storage_dir: storage.to_path_buf(),
        upload_max_size_mb: 25,
        esi: EsiConfig {
            base_url: esi.url(),
            user_agent: USER_AGENT.to_owned(),
            tries: 2,
            retry_delay: Duration::from_millis(10),
            timeout: Duration::from_secs(5),
        },
    }
}

impl TestApp {
    /// A fresh app on an empty database.
    pub async fn new() -> Self {
        let pool = test_pool().await;
        reset_database(&pool).await;
        let esi = MockEsi::start().await;
        let storage = tempfile::tempdir().expect("a storage directory");
        let config = test_config(&esi, storage.path());
        let state = AppState::new(pool.clone(), config.clone());
        Self {
            router: loganalyzer::server::router(state.clone()),
            state,
            pool,
            esi,
            config,
            storage,
        }
    }

    /// Rebuilds the router with a changed configuration, on the same
    /// database, mock and storage.
    pub fn reconfigure(&mut self, change: impl FnOnce(&mut Config)) {
        change(&mut self.config);
        self.state = AppState::new(self.pool.clone(), self.config.clone());
        self.router = loganalyzer::server::router(self.state.clone());
    }

    pub async fn get(&self, path: &str) -> TestResponse {
        self.get_with(path, &[]).await
    }

    pub async fn get_with(&self, path: &str, headers: &[(&str, &str)]) -> TestResponse {
        let mut all = vec![("host", HOST)];
        all.extend_from_slice(headers);
        self.request(Method::GET, path, header_map(&all), Body::empty())
            .await
    }

    /// The props of a page (`/api/pages/...`).
    pub async fn props(&self, page: &str) -> Value {
        let response = self.get(&format!("/api/pages{page}")).await;
        assert_eq!(response.status, StatusCode::OK, "{}", response.text());
        response.json()
    }

    /// A same-origin request as the pages send them.
    pub async fn send(
        &self,
        method: Method,
        path: &str,
        content_type: Option<&str>,
        body: Body,
    ) -> TestResponse {
        let mut headers = vec![
            ("accept", "application/json"),
            ("host", HOST),
            ("origin", APP_URL),
            ("sec-fetch-site", "same-origin"),
        ];
        if let Some(content_type) = content_type {
            headers.push(("content-type", content_type));
        }
        self.request(method, path, header_map(&headers), body).await
    }

    pub async fn post(&self, path: &str) -> TestResponse {
        self.send(Method::POST, path, None, Body::empty()).await
    }

    /// A multipart form with text fields and files (field, file name,
    /// bytes).
    pub async fn post_multipart(
        &self,
        path: &str,
        fields: &[(&str, &str)],
        files: &[(&str, &str, &[u8])],
    ) -> TestResponse {
        let (content_type, body) = multipart(fields, files);
        self.send(Method::POST, path, Some(&content_type), Body::from(body))
            .await
    }

    /// The upload form with one log.
    pub async fn upload(&self, file_name: &str, contents: &[u8]) -> TestResponse {
        self.post_multipart("/analyze", &[], &[("log_file", file_name, contents)])
            .await
    }

    /// Uploads a log and returns its uuid.
    pub async fn upload_log(&self, file_name: &str, contents: &[u8]) -> Uuid {
        let response = self.upload(file_name, contents).await;
        let redirect = response.redirect();
        redirect
            .strip_prefix("/logs/")
            .and_then(|uuid| uuid.parse().ok())
            .unwrap_or_else(|| panic!("no log redirect: {redirect}"))
    }

    pub async fn request(
        &self,
        method: Method,
        path: &str,
        headers: HeaderMap,
        body: Body,
    ) -> TestResponse {
        let mut request = Request::builder().method(method).uri(path);
        for (name, value) in &headers {
            request = request.header(name, value);
        }
        let response = self
            .router
            .clone()
            .oneshot(request.body(body).expect("request builds"))
            .await
            .expect("router answers");
        let status = response.status();
        let headers = response.headers().clone();
        let body = response
            .into_body()
            .collect()
            .await
            .expect("body reads")
            .to_bytes()
            .to_vec();
        TestResponse {
            status,
            headers,
            body,
        }
    }

    pub async fn count(&self, table: &str) -> i64 {
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!("select count(*) from {table}")))
            .fetch_one(&self.pool)
            .await
            .expect("count")
    }

    /// `CombatLog::factory()->create()`: a log row with the given fields.
    pub async fn create_log(
        &self,
        listener: &str,
        session_started: &str,
        original_filename: Option<&str>,
    ) -> (i64, Uuid) {
        let uuid = Uuid::new_v4();
        let id = sqlx::query_scalar(
            "insert into combat_logs (uuid, listener, session_started, original_filename, created_at, updated_at)
             values ($1, $2, $3, $4, now(), now()) returning id",
        )
        .bind(uuid)
        .bind(listener)
        .bind(session_started)
        .bind(original_filename)
        .fetch_one(&self.pool)
        .await
        .expect("log inserts");
        (id, uuid)
    }

    /// `CombatEvent::create([...])` with the fields the PHP tests set.
    pub async fn create_event(
        &self,
        log_id: i64,
        timestamp: &str,
        damage: i64,
        direction: &str,
        player_name: &str,
        weapon: &str,
    ) {
        sqlx::query(
            "insert into combat_events (combat_log_id, timestamp, damage, direction, player_name, weapon, quality, type)
             values ($1, $2, $3, $4, $5, $6, 'Hits', 'damage')",
        )
        .bind(log_id)
        .bind(timestamp)
        .bind(damage)
        .bind(direction)
        .bind(player_name)
        .bind(weapon)
        .execute(&self.pool)
        .await
        .expect("event inserts");
    }

    /// `EveEntity::factory()->create([...])`.
    pub async fn create_entity(&self, kind: &str, name: &str, eve_id: Option<i64>) {
        sqlx::query(
            "insert into eve_entities (kind, name, eve_id, created_at, updated_at)
             values ($1, $2, $3, now(), now())",
        )
        .bind(kind)
        .bind(name)
        .bind(eve_id)
        .execute(&self.pool)
        .await
        .expect("entity inserts");
    }

    /// The stored id of a name, as `EveEntity::where('name', ...)->value('eve_id')`
    /// (the first row of any kind).
    pub async fn eve_id(&self, kind: Option<&str>, name: &str) -> Option<i64> {
        sqlx::query_scalar(
            "select eve_id from eve_entities where name = $1 and ($2::text is null or kind = $2)
             order by id limit 1",
        )
        .bind(name)
        .bind(kind)
        .fetch_optional(&self.pool)
        .await
        .expect("entity reads")
        .flatten()
    }

    /// The only log, as `CombatLog::first()`.
    pub async fn first_log(&self) -> Option<(i64, Uuid, String, String, Option<String>)> {
        sqlx::query_as(
            "select id, uuid, listener, session_started, original_filename from combat_logs
             order by id limit 1",
        )
        .fetch_optional(&self.pool)
        .await
        .expect("log reads")
    }

    pub async fn event_count(&self, log_id: i64) -> i64 {
        sqlx::query_scalar("select count(*) from combat_events where combat_log_id = $1")
            .bind(log_id)
            .fetch_one(&self.pool)
            .await
            .expect("count")
    }

    /// The raw file of a log.
    pub fn raw_path(&self, uuid: Uuid) -> PathBuf {
        self.storage.path().join(format!("{uuid}.txt.gz"))
    }
}

/// A multipart body and its content type.
pub fn multipart(fields: &[(&str, &str)], files: &[(&str, &str, &[u8])]) -> (String, Vec<u8>) {
    let boundary = "----loganalyzer-test-boundary";
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            )
            .as_bytes(),
        );
    }
    for (name, file_name, bytes) in files {
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"; filename=\"{file_name}\"\r\nContent-Type: text/plain\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(bytes);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

pub fn header_map(headers: &[(&str, &str)]) -> HeaderMap {
    let mut map = HeaderMap::new();
    for (name, value) in headers {
        map.append(
            axum::http::HeaderName::from_bytes(name.as_bytes()).expect("header name"),
            value.parse().expect("header value"),
        );
    }
    map
}

pub fn gzip(bytes: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    encoder.write_all(bytes).expect("gzip");
    encoder.finish().expect("gzip")
}

pub fn gunzip(bytes: &[u8]) -> Vec<u8> {
    loganalyzer::storage::gunzip(bytes).expect("a gzip file")
}

pub struct TestResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

impl TestResponse {
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or_else(|error| {
            panic!(
                "{} is not JSON ({error}): {}",
                self.status,
                String::from_utf8_lossy(&self.body)
            )
        })
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|value| value.to_str().ok())
    }

    /// The target of a redirect status.
    #[track_caller]
    pub fn location(&self) -> &str {
        assert!(
            self.status.is_redirection(),
            "{} is no redirect: {}",
            self.status,
            self.text()
        );
        self.header("location").expect("a location")
    }

    /// The `redirect` of a successful form answer.
    #[track_caller]
    pub fn redirect(&self) -> String {
        assert_eq!(self.status, StatusCode::OK, "{}", self.text());
        self.json()["redirect"]
            .as_str()
            .expect("a redirect")
            .to_owned()
    }

    /// The `errors` of a 422 answer.
    #[track_caller]
    pub fn errors(&self) -> Value {
        assert_eq!(
            self.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            self.text()
        );
        self.json()["errors"].clone()
    }
}

/// Asserts two JSON values are equal with the same key set at every
/// level, naming the first path that differs.
#[track_caller]
pub fn assert_json_eq(expected: &Value, actual: &Value) {
    if let Some(difference) = json_difference("$", expected, actual) {
        panic!("{difference}");
    }
}

pub fn json_difference(path: &str, expected: &Value, actual: &Value) -> Option<String> {
    match (expected, actual) {
        (Value::Object(expected), Value::Object(actual)) => {
            let mut expected_keys: Vec<_> = expected.keys().collect();
            let mut actual_keys: Vec<_> = actual.keys().collect();
            expected_keys.sort();
            actual_keys.sort();
            if expected_keys != actual_keys {
                return Some(format!(
                    "{path}: key sets differ\n  expected {expected_keys:?}\n  actual   {actual_keys:?}"
                ));
            }
            expected.iter().find_map(|(key, value)| {
                json_difference(&format!("{path}.{key}"), value, &actual[key])
            })
        }
        (Value::Array(expected), Value::Array(actual)) => {
            if expected.len() != actual.len() {
                return Some(format!(
                    "{path}: expected {} items, got {}",
                    expected.len(),
                    actual.len()
                ));
            }
            expected
                .iter()
                .zip(actual)
                .enumerate()
                .find_map(|(index, (expected, actual))| {
                    json_difference(&format!("{path}[{index}]"), expected, actual)
                })
        }
        _ if expected == actual => None,
        _ => Some(format!("{path}: expected {expected}, got {actual}")),
    }
}

/// The value at a dotted path (`filters.hide`), like Inertia's assertions
/// read props. Keys may contain spaces (`pilotIds.Aria Vex`).
#[track_caller]
pub fn at<'a>(value: &'a Value, path: &str) -> &'a Value {
    let mut current = value;
    for part in path.split('.') {
        current = match current {
            Value::Array(items) => items
                .get(part.parse::<usize>().unwrap_or(usize::MAX))
                .unwrap_or_else(|| panic!("{path}: no item {part}")),
            Value::Object(fields) => fields
                .get(part)
                .unwrap_or_else(|| panic!("{path}: no key {part} in {current}")),
            _ => panic!("{path}: {current} has no {part}"),
        };
    }
    current
}
