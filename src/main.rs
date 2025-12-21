use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::time::Duration;

use dodointerview::{create_router, AppState};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&db_url)
        .await
        .expect("Failed to connect to database");

    let state = AppState { pool };

    let app = create_router(state.clone()).with_state(state.clone());

    // Spawn background worker for webhooks
    tokio::spawn(dodointerview::services::webhooks::process_webhooks(state));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Server running at http://{}", addr);
    println!("Webhooks service enabled");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
