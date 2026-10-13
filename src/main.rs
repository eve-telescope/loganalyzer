#[tokio::main]
async fn main() {
    // Structured logs; tune with RUST_LOG (e.g. RUST_LOG=loganalyzer=debug).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Local configuration from .env, if present; real environment wins.
    dotenvy::dotenv().ok();

    let config = loganalyzer::config::Config::from_env();
    let addr = loganalyzer::server::bind_addr();

    let pool = loganalyzer::db::connect()
        .await
        .expect("database connection");
    loganalyzer::db::migrate(&pool)
        .await
        .expect("database migrations");

    let state = loganalyzer::server::AppState::new(pool, config);
    let app = loganalyzer::server::router(state);

    tracing::info!("listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("bind address");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .expect("serve");
}
