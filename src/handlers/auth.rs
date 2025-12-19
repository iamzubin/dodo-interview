use crate::state::AppState;
use axum::{extract::State, Json};
use bcrypt::{hash, verify, DEFAULT_COST};
use hex;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::Row;

#[derive(Deserialize)]
pub struct SignupRequest {
    email: String,
    password: String,
    name: String,
}

#[derive(Deserialize)]
pub struct GenerateApiKeyRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
pub struct GenerateApiKeyResponse {
    api_key: String,
}

pub async fn signup(
    State(state): State<AppState>,
    Json(payload): Json<SignupRequest>,
) -> Result<Json<Value>, Json<Value>> {
    let password_hash = hash(&payload.password, DEFAULT_COST)
        .map_err(|_| Json(json!({ "error": "Failed to hash password" })))?;

    let result = sqlx::query(
        "INSERT INTO businesses (email, password_hash, name) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(&payload.email)
    .bind(&password_hash)
    .bind(&payload.name)
    .fetch_one(&state.pool)
    .await;

    match result {
        Ok(row) => {
            let id: sqlx::types::Uuid = row.get("id");
            Ok(Json(
                json!({ "id": id.to_string(), "email": payload.email, "name": payload.name }),
            ))
        }
        Err(sqlx::Error::Database(e)) if e.constraint().is_some() => {
            Err(Json(json!({ "error": "Email already exists" })))
        }
        Err(_) => Err(Json(json!({ "error": "Failed to create business" }))),
    }
}

pub async fn generate_api_key(
    State(state): State<AppState>,
    Json(payload): Json<GenerateApiKeyRequest>,
) -> Result<Json<GenerateApiKeyResponse>, Json<Value>> {
    let row = sqlx::query("SELECT id, password_hash FROM businesses WHERE email = $1")
        .bind(&payload.email)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| Json(json!({ "error": "Database error" })))?;

    let (business_id, password_hash) = match row {
        Some(row) => (
            row.get::<sqlx::types::Uuid, _>("id"),
            row.get::<String, _>("password_hash"),
        ),
        None => return Err(Json(json!({ "error": "Invalid credentials" }))),
    };

    if !verify(&payload.password, &password_hash).unwrap_or(false) {
        return Err(Json(json!({ "error": "Invalid credentials" })));
    }

    let api_key = format!(
        "sk_live_{}",
        hex::encode(rand::thread_rng().gen::<[u8; 32]>())
    );
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let key_hash = hex::encode(hasher.finalize());

    sqlx::query("INSERT INTO api_keys (business_id, key_hash, is_active) VALUES ($1, $2, true)")
        .bind(business_id)
        .bind(&key_hash)
        .execute(&state.pool)
        .await
        .map_err(|_| Json(json!({ "error": "Failed to create API key" })))?;

    Ok(Json(GenerateApiKeyResponse { api_key }))
}

