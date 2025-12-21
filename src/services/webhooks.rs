use crate::models::WebhookEventStatus;
use crate::state::AppState;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{types::Uuid, Row};
use std::time::Duration;

pub async fn process_webhooks(state: AppState) {
    let client = reqwest::Client::new();

    loop {
        // Fetch pending events that are due for processing (first attempt or retries after backoff)
        let events = sqlx::query(
            "SELECT we.id, we.event_type, we.payload, we.attempts, ep.url, ep.secret 
             FROM webhook_events we
             JOIN webhook_endpoints ep ON we.webhook_endpoint_id = ep.id
             WHERE we.status = 'pending'::webhook_event_status 
             AND ep.is_active = true
             AND (we.last_attempt_at IS NULL OR we.last_attempt_at < NOW() - INTERVAL '10 seconds' * (we.attempts + 1))
             LIMIT 10
             FOR UPDATE OF we SKIP LOCKED",
        )
        .fetch_all(&state.pool)
        .await;

        match events {
            Ok(rows) => {
                if rows.is_empty() {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue;
                }

                for row in rows {
                    let event_id: Uuid = row.get("id");
                    let url: String = row.get("url");
                    let payload: Value = row.get("payload");
                    let secret: String = row.get("secret");
                    let attempts: i32 = row.get("attempts");

                    // TODO: For better security, sign the payload using HMAC-SHA256 with the secret
                    // and send the signature in a header (e.g., X-Webhook-Signature).
                    // Sending the raw secret is essentially a shared password.

                    let result = client
                        .post(&url)
                        .header("X-Webhook-Secret", secret)
                        .json(&payload)
                        .send()
                        .await;

                    let (new_status, _error) = match result {
                        Ok(res) => {
                            if res.status().is_success() {
                                (WebhookEventStatus::Delivered, None)
                            } else {
                                if attempts >= 5 {
                                    (
                                        WebhookEventStatus::Failed,
                                        Some(format!("HTTP {}", res.status())),
                                    )
                                } else {
                                    (
                                        WebhookEventStatus::Pending,
                                        Some(format!("HTTP {} (will retry)", res.status())),
                                    )
                                }
                            }
                        }
                        Err(e) => {
                            if attempts >= 5 {
                                (WebhookEventStatus::Failed, Some(e.to_string()))
                            } else {
                                (WebhookEventStatus::Pending, Some(e.to_string()))
                            }
                        }
                    };

                    let _ = sqlx::query(
                        "UPDATE webhook_events 
                         SET status = $1, last_attempt_at = NOW(), attempts = attempts + 1 
                         WHERE id = $2",
                    )
                    .bind(new_status)
                    .bind(event_id)
                    .execute(&state.pool)
                    .await;
                }
            }
            Err(e) => {
                eprintln!("Error fetching webhooks: {}", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct RegisterWebhookRequest {
    pub url: String,
    pub secret: String,
}

#[derive(Deserialize, Serialize)]
pub struct WebhookEndpointResponse {
    pub id: String,
    pub business_id: String,
    pub url: String,
    pub is_active: bool,
}

pub async fn register_webhook(
    state: &AppState,
    business_id: Uuid,
    payload: RegisterWebhookRequest,
) -> Result<WebhookEndpointResponse, Json<Value>> {
    let result = sqlx::query(
        "INSERT INTO webhook_endpoints (business_id, url, secret) VALUES ($1, $2, $3) RETURNING id, is_active"
    )
    .bind(business_id)
    .bind(&payload.url)
    .bind(&payload.secret)
    .fetch_one(&state.pool)
    .await;

    match result {
        Ok(row) => {
            let id: Uuid = row.get("id");
            let is_active: bool = row.get("is_active");
            Ok(WebhookEndpointResponse {
                id: id.to_string(),
                business_id: business_id.to_string(),
                url: payload.url,
                is_active,
            })
        }
        Err(_) => Err(Json(json!({ "error": "Failed to register webhook" }))),
    }
}

pub async fn list_webhooks(
    state: &AppState,
    business_id: Uuid,
) -> Result<Vec<WebhookEndpointResponse>, Json<Value>> {
    let rows = sqlx::query(
        "SELECT id, business_id, url, is_active FROM webhook_endpoints WHERE business_id = $1",
    )
    .bind(business_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| Json(json!({ "error": "Failed to fetch webhooks" })))?;

    let webhooks = rows
        .into_iter()
        .map(|row| {
            let id: Uuid = row.get("id");
            let business_id_val: Uuid = row.get("business_id");
            WebhookEndpointResponse {
                id: id.to_string(),
                business_id: business_id_val.to_string(),
                url: row.get("url"),
                is_active: row.get("is_active"),
            }
        })
        .collect();

    Ok(webhooks)
}
