use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt; // for collecting body
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt; // for one_shot

#[tokio::test]
async fn health_check() {
    // Create a test pool - health check doesn't use it, so any valid connection string works
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://dodo:dodo_password@localhost:5432/dodo".to_string());
    
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
        .expect("Failed to create test pool");
    
    let state = dodointerview::AppState { pool };
    let app = dodointerview::create_router().with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = std::str::from_utf8(&body).unwrap();
    assert!(body_str.contains("healthy"));
    assert!(body_str.contains("database"));
}
