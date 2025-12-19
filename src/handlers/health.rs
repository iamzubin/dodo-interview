use axum::{extract::State, Json};
use serde_json::{json, Value};
use crate::state::AppState;

pub async fn health_check(State(state): State<AppState>) -> Json<Value> {
    // Check PostgreSQL connection
    match sqlx::query("SELECT 1")
        .execute(&state.pool)
        .await
    {
        Ok(_) => Json(json!({
            "status": "healthy",
            "database": "connected"
        })),
        Err(e) => Json(json!({
            "status": "unhealthy",
            "database": "disconnected",
            "error": e.to_string()
        })),
    }
}

