//! A mock of ESI's `POST /universe/ids/` on an ephemeral port, answering
//! from a name table the way scripts/laravel-reference/mock_esi.py
//! answered the Laravel reference: every requested name the table knows
//! (compared case-insensitively) under each category that has it, with
//! the table's spelling; empty categories left out. It records what it
//! was asked and can be told to fail.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use serde_json::{Map, Value, json};

use super::repo_path;

/// One request the mock received.
#[derive(Debug, Clone)]
pub struct EsiRequest {
    pub names: Vec<String>,
    pub user_agent: Option<String>,
    pub compatibility_date: Option<String>,
    pub content_type: Option<String>,
}

#[derive(Default)]
struct Inner {
    table: Value,
    /// Statuses to answer with, one per request, before answering
    /// normally again.
    failures: VecDeque<u16>,
    /// A status to answer every request with.
    always: Option<u16>,
    /// A body to answer with instead of the table's answer.
    body: Option<String>,
    delay: Option<Duration>,
    requests: Vec<EsiRequest>,
}

#[derive(Clone)]
pub struct MockEsi {
    inner: Arc<Mutex<Inner>>,
    url: String,
}

/// The table of tests/fixtures/esi/names.json.
pub fn fixture_table() -> Value {
    let path = repo_path("tests/fixtures/esi/names.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("names.json")).expect("JSON")
}

async fn ids(State(mock): State<MockEsi>, headers: HeaderMap, body: String) -> Response {
    let names: Vec<String> = serde_json::from_str::<Vec<Value>>(&body)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|name| name.as_str().map(str::to_owned))
        .collect();
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    };
    let (status, delay, body, table) = {
        let mut inner = mock.inner.lock().expect("mock state");
        inner.requests.push(EsiRequest {
            names: names.clone(),
            user_agent: header("user-agent"),
            compatibility_date: header("x-compatibility-date"),
            content_type: header("content-type"),
        });
        let status = inner.failures.pop_front().or(inner.always);
        (status, inner.delay, inner.body.clone(), inner.table.clone())
    };
    if let Some(delay) = delay {
        tokio::time::sleep(delay).await;
    }
    if let Some(status) = status {
        return (
            StatusCode::from_u16(status).expect("a status"),
            Json(json!({ "error": "mock failure" })),
        )
            .into_response();
    }
    if let Some(body) = body {
        return ([("content-type", "application/json")], body).into_response();
    }
    Json(answer(&table, &names)).into_response()
}

/// ESI's answer for `names` from a table `{category: {name: id}}`.
pub fn answer(table: &Value, names: &[String]) -> Value {
    let mut result = Map::new();
    for (category, known) in table.as_object().expect("a table") {
        let known = known.as_object().expect("a category");
        let found: Vec<Value> = names
            .iter()
            .filter_map(|name| {
                known
                    .iter()
                    .find(|(spelling, _)| spelling.to_lowercase() == name.to_lowercase())
                    .map(|(spelling, id)| json!({ "id": id, "name": spelling }))
            })
            .collect();
        if !found.is_empty() {
            result.insert(category.clone(), Value::Array(found));
        }
    }
    Value::Object(result)
}

impl MockEsi {
    pub async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a port");
        let url = format!("http://{}", listener.local_addr().expect("an address"));
        let mock = Self {
            inner: Arc::new(Mutex::new(Inner {
                table: fixture_table(),
                ..Inner::default()
            })),
            url,
        };
        let app = axum::Router::new()
            .route("/universe/ids/", post(ids))
            .with_state(mock.clone());
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("mock ESI serves");
        });
        mock
    }

    pub fn url(&self) -> String {
        self.url.clone()
    }

    fn with<T>(&self, change: impl FnOnce(&mut Inner) -> T) -> T {
        change(&mut self.inner.lock().expect("mock state"))
    }

    /// Answers from another table.
    pub fn set_table(&self, table: Value) {
        self.with(|inner| inner.table = table);
    }

    /// Answers every request with `status`.
    pub fn fail_with(&self, status: u16) {
        self.with(|inner| inner.always = Some(status));
    }

    /// Answers the next requests with these statuses, then normally.
    pub fn fail_next(&self, statuses: &[u16]) {
        self.with(|inner| inner.failures.extend(statuses));
    }

    /// Answers with this body (status 200).
    pub fn answer_with(&self, body: &str) {
        self.with(|inner| inner.body = Some(body.to_owned()));
    }

    pub fn delay(&self, delay: Duration) {
        self.with(|inner| inner.delay = Some(delay));
    }

    pub fn requests(&self) -> Vec<EsiRequest> {
        self.with(|inner| inner.requests.clone())
    }
}
