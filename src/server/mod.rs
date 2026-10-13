//! The HTTP side: shared state, the router and the cross-cutting layers.
//! Pages are rendered by SvelteKit; the api serves their props as JSON
//! (`/api/pages/*`), the upload, the regeneration and the raw log
//! download at the paths the site always used, and the health page.

pub mod actions;
pub mod errors;
pub mod health;
pub mod pages;

use std::sync::Arc;

use axum::Router;
use axum::extract::{DefaultBodyLimit, Request};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use sqlx::PgPool;

use crate::config::Config;
use crate::esi::Esi;
use crate::storage::RawLogs;

/// The default bind address of the API server; the SvelteKit dev proxy and
/// the reverse proxy point at it.
const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3000";

/// The listen address from `BIND_ADDR`, with the local default.
pub fn bind_addr() -> String {
    std::env::var("BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_owned())
}

/// Room for the multipart framing around the largest accepted log, so a
/// log just under the limit always gets through and one over it gets the
/// size message.
pub const MULTIPART_OVERHEAD: u64 = 1024 * 1024;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
    pub esi: Esi,
    pub raw_logs: RawLogs,
}

impl AppState {
    pub fn new(pool: PgPool, config: Config) -> Self {
        Self {
            pool,
            esi: Esi::new(config.esi.clone()),
            raw_logs: RawLogs::new(config.log_storage_dir.clone()),
            config: Arc::new(config),
        }
    }
}

pub fn router(state: AppState) -> Router {
    let body_limit = (state.config.upload_max_bytes() + MULTIPART_OVERHEAD) as usize;
    Router::new()
        .route("/up", get(health::up))
        .route("/analyze", post(actions::analyze))
        .route("/logs/{uuid}/download", get(actions::download))
        .route("/logs/{uuid}/regenerate", post(actions::regenerate))
        .nest("/api", api_router())
        .fallback(errors::not_found)
        .method_not_allowed_fallback(errors::method_not_allowed)
        .layer(DefaultBodyLimit::max(body_limit))
        .layer(axum::middleware::from_fn(reject_cross_site_mutations))
        .layer(axum::middleware::from_fn(canonical_paths))
        .with_state(state)
}

fn api_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health::health))
        .route("/pages/home", get(pages::home))
        .route("/pages/logs/{uuid}", get(pages::log))
}

/// One URL per page: a trailing slash (`/logs/x/`) redirects to the path
/// without it. 308 keeps the method of a form post.
async fn canonical_paths(request: Request, next: Next) -> Response {
    let path = request.uri().path();
    let query = request
        .uri()
        .query()
        .map(|query| format!("?{query}"))
        .unwrap_or_default();
    let target = (path.len() > 1 && path.ends_with('/')).then(|| {
        let trimmed = path.trim_end_matches('/');
        if trimmed.is_empty() {
            "/".to_owned()
        } else {
            trimmed.to_owned()
        }
    });
    match target {
        Some(path) => {
            let mut response = StatusCode::PERMANENT_REDIRECT.into_response();
            if let Ok(location) = HeaderValue::from_str(&format!("{path}{query}")) {
                response.headers_mut().insert(header::LOCATION, location);
            }
            response
        }
        None => next.run(request).await,
    }
}

/// The same-origin check that stands in for a CSRF token: a mutation
/// the browser marks as coming from another site, or whose `Origin` is not
/// this host, is refused. Requests without either header (scripts, curl)
/// cannot be forged by a third-party page and pass.
async fn reject_cross_site_mutations(request: Request, next: Next) -> Response {
    let read_only = matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    if !read_only && is_cross_site(request.headers()) {
        return errors::json_error(
            StatusCode::FORBIDDEN,
            errors::message(StatusCode::FORBIDDEN),
        );
    }
    next.run(request).await
}

fn is_cross_site(headers: &HeaderMap) -> bool {
    let fetch_site = headers
        .get("sec-fetch-site")
        .and_then(|value| value.to_str().ok());
    if fetch_site.is_some_and(|site| {
        !site.eq_ignore_ascii_case("same-origin") && !site.eq_ignore_ascii_case("none")
    }) {
        return true;
    }
    headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|origin| is_cross_site_url(origin, headers))
}

/// Whether a URL (an `Origin` or `Referer`) names a host other than the
/// one serving the request; an opaque `null` origin counts as foreign.
fn is_cross_site_url(url: &str, headers: &HeaderMap) -> bool {
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let authority = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .and_then(|rest| rest.split('/').next())
        .unwrap_or_default();
    authority.is_empty() || !authority.eq_ignore_ascii_case(host)
}
