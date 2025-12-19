use axum::{extract::{Extension, State}, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::state::AppState;
use sqlx::{types::Uuid, Row};

#[derive(Deserialize)]
pub struct CreateAccountRequest {
    currency: String,
}

#[derive(Serialize)]
pub struct AccountResponse {
    id: String,
    business_id: String,
    balance: i64,
    currency: String,
}

pub async fn create_account(
    State(state): State<AppState>,
    Extension(business_id): Extension<Uuid>,
    Json(payload): Json<CreateAccountRequest>,
) -> Result<Json<AccountResponse>, Json<Value>> {
    let result = sqlx::query(
        "INSERT INTO accounts (business_id, currency, balance) VALUES ($1, $2, 0) RETURNING id, balance"
    )
    .bind(business_id)
    .bind(&payload.currency)
    .fetch_one(&state.pool)
    .await;

    match result {
        Ok(row) => {
            let id: Uuid = row.get("id");
            let balance: i64 = row.get("balance");
            Ok(Json(AccountResponse {
                id: id.to_string(),
                business_id: business_id.to_string(),
                balance,
                currency: payload.currency,
            }))
        }
        Err(_) => Err(Json(json!({ "error": "Failed to create account" }))),
    }
}