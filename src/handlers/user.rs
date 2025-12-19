use axum::{extract::{Path, State}, Json};
use serde_json::{json, Value};
use crate::state::AppState;

pub async fn create_user(State(state): State<AppState>) -> Json<Value> {
    let _pool = &state.pool;
    Json(json!({ "status": "Create user placeholder" }))
}

pub async fn list_users(State(state): State<AppState>) -> Json<Value> {
    let _pool = &state.pool;
    Json(json!({ "status": "List users placeholder" }))
}

pub async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<u32>
) -> Json<Value> {
    let _pool = &state.pool;
    Json(json!({ "status": "Get user placeholder", "id": id }))
}

pub async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<u32>
) -> Json<Value> {
    let _pool = &state.pool;
    Json(json!({ "status": "Update user placeholder", "id": id }))
}

pub async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<u32>
) -> Json<Value> {
    let _pool = &state.pool;
    Json(json!({ "status": "Delete user placeholder", "id": id }))
}