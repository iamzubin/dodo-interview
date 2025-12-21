use crate::state::AppState;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use hex;
use sha2::{Digest, Sha256};
use sqlx::Row;
use tower_governor::{errors::GovernorError, key_extractor::KeyExtractor};

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ApiKeyExtractor;

impl KeyExtractor for ApiKeyExtractor {
    type Key = String;

    fn extract<B>(&self, req: &Request<B>) -> Result<Self::Key, GovernorError> {
        req.headers()
            .get("Authorization")
            .and_then(|value| value.to_str().ok())
            .map(|s| s.to_string())
            .ok_or(GovernorError::UnableToExtractKey)
    }
}

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
            .unwrap_or_default();
    }

    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let key_hash = hex::encode(hasher.finalize());

    let row = match sqlx::query(
        "SELECT business_id FROM api_keys WHERE key_hash = $1 AND is_active = true",
    )
    .bind(&key_hash)
    .fetch_optional(&state.pool)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => {
            return Response::builder()
                .status(401)
                .body("Unauthorized".into())
                .unwrap_or_default();
        }
        Err(_) => {
            return Response::builder()
                .status(500)
                .body("Internal Server Error".into())
                .unwrap_or_default();
        }
    };

    let business_id: sqlx::types::Uuid = row.get("business_id");
    request.extensions_mut().insert(business_id);
    next.run(request).await
}
