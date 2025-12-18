use axum::{extract::Path, Json};
use serde_json::{json, Value};

pub async fn create_user() -> Json<Value> {
    Json(json!({ "status": "Create user placeholder" }))
}

pub async fn list_users() -> Json<Value> {
    Json(json!({ "status": "List users placeholder" }))
}

pub async fn get_user(Path(id): Path<u32>) -> Json<Value> {
    Json(json!({ "status": "Get user placeholder", "id": id }))
}

pub async fn update_user(Path(id): Path<u32>) -> Json<Value> {
    Json(json!({ "status": "Update user placeholder", "id": id }))
}

pub async fn delete_user(Path(id): Path<u32>) -> Json<Value> {
    Json(json!({ "status": "Delete user placeholder", "id": id }))
}