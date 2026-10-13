//! What the pages send and link to: the upload, the regeneration of a
//! log and the raw log download. The upload and the regeneration answer
//! JSON (`{"redirect": path}`, or `422 {"errors": {field: message}}`);
//! the pages send them with XMLHttpRequest and fetch.

use axum::Json;
use axum::extract::{Multipart, Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::json;
use uuid::Uuid;

use super::{AppState, errors};
use crate::logs::{self, RegenerateError};
use crate::upload;

fn redirect_to_log(uuid: Uuid) -> Response {
    Json(json!({ "redirect": format!("/logs/{uuid}") })).into_response()
}

fn invalid(field: &str, message: &str) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({ "errors": { field: message } })),
    )
        .into_response()
}

/// `POST /analyze`: a gamelog upload.
pub async fn analyze(
    State(state): State<AppState>,
    headers: HeaderMap,
    multipart: Result<Multipart, axum::extract::multipart::MultipartRejection>,
) -> Response {
    let max_size_mb = state.config.upload_max_size_mb;
    let file = match multipart {
        Ok(multipart) => match upload::read(multipart, max_size_mb).await {
            Ok(file) => file,
            Err(message) => return invalid("log_file", &message),
        },
        // Not a multipart body at all: no file was sent.
        Err(_) => return invalid("log_file", upload::REQUIRED),
    };
    match logs::store(
        &state,
        file.contents,
        Some(file.original_name),
        Uuid::new_v4(),
    )
    .await
    {
        Ok(uuid) => redirect_to_log(uuid),
        Err(error) => errors::server_error(error, &headers),
    }
}

/// `POST /logs/{uuid}/regenerate`.
pub async fn regenerate(
    State(state): State<AppState>,
    Path(uuid): Path<String>,
    headers: HeaderMap,
) -> Response {
    let log = match logs::find(&state.pool, &uuid).await {
        Ok(Some(log)) => log,
        Ok(None) => return errors::error(StatusCode::NOT_FOUND, &headers),
        Err(error) => return errors::server_error(error, &headers),
    };
    match logs::regenerate(&state, &log).await {
        Ok(()) => redirect_to_log(log.uuid),
        Err(RegenerateError::Missing) => errors::error(StatusCode::NOT_FOUND, &headers),
        Err(RegenerateError::Undecodable) => errors::json_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "The stored combat log could not be decoded.",
        ),
        Err(RegenerateError::Failed(error)) => errors::server_error(error, &headers),
    }
}

/// `GET /logs/{uuid}/download`: the raw log as it was uploaded.
pub async fn download(
    State(state): State<AppState>,
    Path(uuid): Path<String>,
    headers: HeaderMap,
) -> Response {
    let log = match logs::find(&state.pool, &uuid).await {
        Ok(Some(log)) => log,
        Ok(None) => return errors::error_page(StatusCode::NOT_FOUND),
        Err(error) => return errors::server_error(error, &headers),
    };
    let contents = match state.raw_logs.get(log.uuid).await {
        Ok(Some(Ok(contents))) => contents,
        Ok(Some(Err(_))) => return errors::error_page(StatusCode::UNPROCESSABLE_ENTITY),
        Ok(None) => return errors::error_page(StatusCode::NOT_FOUND),
        Err(error) => return errors::server_error(error, &headers),
    };
    let name = log
        .original_filename
        .filter(|name| !name.is_empty() && name != "0")
        .unwrap_or_else(|| "combat-log.txt".to_owned());
    let mut response = contents.into_response();
    let response_headers = response.headers_mut();
    response_headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    response_headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, private"),
    );
    if let Ok(disposition) = HeaderValue::from_str(&crate::disposition::attachment(&name)) {
        response_headers.insert(header::CONTENT_DISPOSITION, disposition);
    }
    response
}
