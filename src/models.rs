use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CreateAccountRequest {
    pub currency: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "idempotency_status", rename_all = "lowercase")]
pub enum IdempotencyStatus {
    Pending,
    Success,
    Failed,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "webhook_event_status", rename_all = "lowercase")]
pub enum WebhookEventStatus {
    Pending,
    Delivered,
    Failed,
}

#[derive(Deserialize)]
pub struct GetAccountsQuery {
    pub currency: Option<String>,
    pub business_id: Option<String>,
}

#[derive(Deserialize)]
pub struct TransferRequest {
    pub from_account_id: String,
    pub to_account_id: String,
    pub amount: i64,
    pub idempotency_key: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TransferResponse {
    pub transaction_id: String,
    pub from_account_id: String,
    pub to_account_id: String,
    pub amount: i64,
    pub currency: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct AccountResponse {
    pub id: String,
    pub business_id: String,
    pub business_name: Option<String>,
    pub business_email: String,
    pub balance: i64,
    pub currency: String,
}
