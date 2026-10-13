//! Error answers: the minimal error page for the common statuses, a
//! plain page for the rest, and JSON for the api and for fetch requests.

use axum::Json;
use axum::http::{HeaderMap, HeaderValue, StatusCode, Uri, header};
use axum::response::{Html, IntoResponse, Response};
use serde_json::json;

/// The minimal error page: the status and its message, centered.
const MINIMAL_PAGE: &str = include_str!("error.html");
/// The plain page for the statuses the minimal page is not used for.
const PLAIN_PAGE: &str = include_str!("plain-error.html");

/// The English message of a status, as the pages and JSON errors show it.
pub fn message(status: StatusCode) -> &'static str {
    match status.as_u16() {
        401 => "Unauthorized",
        402 => "Payment Required",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Content Too Large",
        419 => "Page Expired",
        422 => "Unprocessable Content",
        429 => "Too Many Requests",
        503 => "Service Unavailable",
        _ => "Server Error",
    }
}

pub fn error_page(status: StatusCode) -> Response {
    let template = match status.as_u16() {
        401 | 402 | 403 | 404 | 419 | 429 | 500 | 503 => MINIMAL_PAGE,
        _ => PLAIN_PAGE,
    };
    let page = template
        .replace("{{status}}", status.as_str())
        .replace("{{message}}", message(status));
    let mut response = (status, Html(page)).into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, private"),
    );
    response
}

pub fn json_error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "message": message }))).into_response()
}

/// Whether the client asked for JSON (the pages' fetch requests do).
pub fn wants_json(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|accept| accept.contains("json"))
}

/// An error in the form the client asked for.
pub fn error(status: StatusCode, headers: &HeaderMap) -> Response {
    if wants_json(headers) {
        json_error(status, message(status))
    } else {
        error_page(status)
    }
}

/// Any failure the client cannot fix: logged, answered generically.
pub fn server_error(error: impl std::fmt::Display, headers: &HeaderMap) -> Response {
    tracing::error!(%error, "request failed");
    self::error(StatusCode::INTERNAL_SERVER_ERROR, headers)
}

/// Unknown paths: JSON for the api, the 404 page otherwise.
pub async fn not_found(uri: Uri) -> Response {
    if uri.path() == "/api" || uri.path().starts_with("/api/") {
        json_error(StatusCode::NOT_FOUND, message(StatusCode::NOT_FOUND))
    } else {
        error_page(StatusCode::NOT_FOUND)
    }
}

/// A known path with another method.
pub async fn method_not_allowed() -> Response {
    error_page(StatusCode::METHOD_NOT_ALLOWED)
}
