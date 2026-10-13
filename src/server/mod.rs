//! The HTTP side: shared state, the router and the cross-cutting layers.
//! Every route of the Laravel app is registered and answers 501 until it
//! is ported, so the tests fail on behavior, not on missing routes.

pub mod errors;

use std::sync::Arc;

use axum::Router;
use axum::routing::{any, get, post};
use sqlx::PgPool;

use crate::config::Config;

/// The default bind address of the API server; the SvelteKit dev proxy and
/// the reverse proxy point at it.
const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3000";

/// The listen address from `BIND_ADDR`, with the local default.
pub fn bind_addr() -> String {
    std::env::var("BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_owned())
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
}

impl AppState {
    pub fn new(pool: PgPool, config: Config) -> Self {
        Self {
            pool,
            config: Arc::new(config),
        }
    }
}

pub fn router(state: AppState) -> Router {
    let pending = errors::not_implemented;
    Router::new()
        .route("/up", get(pending))
        .route("/analyze", post(pending))
        .route("/logs/{uuid}/download", get(pending))
        .route("/logs/{uuid}/regenerate", post(pending))
        .route("/storage/{*path}", any(pending))
        .route("/api/health", get(pending))
        .route("/api/pages/home", get(pending))
        .route("/api/pages/logs/{uuid}", get(pending))
        .fallback(errors::not_found)
        .method_not_allowed_fallback(errors::method_not_allowed)
        .with_state(state)
}
