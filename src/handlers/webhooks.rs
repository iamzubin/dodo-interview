use crate::services::webhooks::{
    list_webhooks, register_webhook, RegisterWebhookRequest, WebhookEndpointResponse,
};
use crate::state::AppState;
use axum::{
    extract::{Extension, State},
    Json,
};
use serde_json::Value;
use sqlx::types::Uuid;

pub async fn register_webhook_handler(
    State(state): State<AppState>,
    Extension(business_id): Extension<Uuid>,
    Json(payload): Json<RegisterWebhookRequest>,
) -> Result<Json<WebhookEndpointResponse>, Json<Value>> {
    let response = register_webhook(&state, business_id, payload).await?;
    Ok(Json(response))
}

pub async fn list_webhooks_handler(
    State(state): State<AppState>,
    Extension(business_id): Extension<Uuid>,
) -> Result<Json<Vec<WebhookEndpointResponse>>, Json<Value>> {
    let response = list_webhooks(&state, business_id).await?;
    Ok(Json(response))
}
