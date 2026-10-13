//! The props of the two pages, one endpoint each under `/api/pages`,
//! with the props every page shares (`name`, `auth`).

use axum::Json;
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::{Map, Value, json};

use super::{AppState, errors};
use crate::filters::Filters;
use crate::logs;

fn with_shared(state: &AppState, mut props: Map<String, Value>) -> Json<Value> {
    props.insert("name".to_owned(), json!(state.config.app_name));
    props.insert("auth".to_owned(), json!({ "user": null }));
    Json(Value::Object(props))
}

/// The upload page.
pub async fn home(State(state): State<AppState>) -> Json<Value> {
    let mut props = Map::new();
    props.insert(
        "maxUploadSizeMb".to_owned(),
        json!(state.config.upload_max_size_mb),
    );
    with_shared(&state, props)
}

/// The analysis page of a log.
pub async fn log(
    State(state): State<AppState>,
    Path(uuid): Path<String>,
    RawQuery(query): RawQuery,
) -> Response {
    let log = match logs::find(&state.pool, &uuid).await {
        Ok(Some(log)) => log,
        Ok(None) => {
            return errors::json_error(
                StatusCode::NOT_FOUND,
                errors::message(StatusCode::NOT_FOUND),
            );
        }
        Err(error) => return errors::server_error(error, &json_accept()),
    };
    let filters = Filters::from_query(query.as_deref().unwrap_or_default());
    match logs::page_props(&state.pool, &state.raw_logs, &log, filters).await {
        Ok(props) => with_shared(&state, props).into_response(),
        Err(error) => errors::server_error(error, &json_accept()),
    }
}

/// The api answers JSON whatever the client accepts.
fn json_accept() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::ACCEPT,
        axum::http::HeaderValue::from_static("application/json"),
    );
    headers
}
