use crate::state::AppState;
use axum::{extract::{Request, State}, middleware::Next, response::Response};
use hex;
use sha2::{Digest, Sha256};
use sqlx::Row;

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let api_key = request
        .headers()
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if api_key.is_empty() {
        return Response::builder()
            .status(401)
            .body("Unauthorized".into())
            .unwrap();
    }
    // Hash the API key (same way as when it was created)
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let key_hash = hex::encode(hasher.finalize());
    let row: Option<sqlx::postgres::PgRow> = match sqlx::query("SELECT business_id FROM api_keys WHERE key_hash = $1")
        .bind(&key_hash)
        .fetch_optional(&state.pool)
        .await
    {
        Ok(row) => row,
        Err(_) => {
            return Response::builder()
                .status(401)
                .body("Unauthorized".into())
                .unwrap();
        }
    };
    if row.is_none() {
        return Response::builder()
            .status(401)
            .body("Unauthorized".into())
            .unwrap();
    }
    let business_id: sqlx::types::Uuid = row.unwrap().get("business_id");
    request.extensions_mut().insert(business_id);
    next.run(request).await
}
