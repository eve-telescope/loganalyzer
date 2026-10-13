//! Seeds the sample logs into the configured database (a local stack's
//! preview), safe to repeat. Not for production.

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let config = loganalyzer::config::Config::from_env();
    let pool = loganalyzer::db::connect()
        .await
        .expect("database connection");
    loganalyzer::db::migrate(&pool)
        .await
        .expect("database migrations");
    let state = loganalyzer::server::AppState::new(pool, config);
    let added = loganalyzer::samples::seed(&state)
        .await
        .unwrap_or_else(|error| panic!("seeding failed: {error}"));
    for sample in &loganalyzer::samples::SAMPLES {
        println!("/logs/{}", sample.uuid);
    }
    println!("{added} sample logs added.");
}
