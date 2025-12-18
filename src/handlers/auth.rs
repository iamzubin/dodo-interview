use axum::Json;
use serde_json::{json, Value};

pub async fn login() -> Json<Value> {
    Json(json!({ "status": "Login endpoint placeholder" }))
}

pub async fn signup() -> Json<Value> {
    Json(json!({ "status": "Signup endpoint placeholder" }))
}

pub async fn logout() -> Json<Value> {
    Json(json!({ "status": "Logout endpoint placeholder" }))
}
    