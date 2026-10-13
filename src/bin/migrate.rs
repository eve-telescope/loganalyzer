//! Applies the database migrations and exits, so the schema can be in place
//! before the api first starts (a data import, for one).

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let pool = loganalyzer::db::connect()
        .await
        .expect("database connection");
    loganalyzer::db::migrate(&pool)
        .await
        .expect("database migrations");
    println!("Migrations applied.");
}
