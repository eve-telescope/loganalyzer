//! Health checks: the `/up` page and the deploy check `/api/health`.

use std::time::Instant;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde_json::json;

use super::AppState;

/// The health page the uptime checks read.
const UP_PAGE: &str = include_str!("up.html");

pub async fn up(State(state): State<AppState>) -> Html<String> {
    let started = Instant::now();
    Html(
        UP_PAGE
            .replace("{{app_name}}", &escape(&state.config.app_name))
            .replace(
                "{{milliseconds}}",
                &started.elapsed().as_millis().to_string(),
            ),
    )
}

/// Up, and the database answers.
pub async fn health(State(state): State<AppState>) -> Response {
    match sqlx::query("select 1").execute(&state.pool).await {
        Ok(_) => Json(json!({ "status": "ok", "database": "ok" })).into_response(),
        Err(error) => {
            tracing::error!(%error, "health check: database unreachable");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "status": "error", "database": "unreachable" })),
            )
                .into_response()
        }
    }
}

/// HTML text escaping.
pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#039;")
}
