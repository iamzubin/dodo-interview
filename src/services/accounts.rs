use crate::models::{TransferRequest, TransferResponse};
use crate::state::AppState;
use axum::Json;
use serde_json::{json, Value};
use sqlx::{types::Uuid, Row};

pub fn validate_transfer_input(payload: &TransferRequest) -> Result<(Uuid, Uuid), Json<Value>> {
    if payload.amount <= 0 {
        return Err(Json(json!({ "error": "Amount must be positive" })));
    }

    let from_account_id = Uuid::parse_str(&payload.from_account_id)
        .map_err(|_| Json(json!({ "error": "Invalid from_account_id format" })))?;

    let to_account_id = Uuid::parse_str(&payload.to_account_id)
        .map_err(|_| Json(json!({ "error": "Invalid to_account_id format" })))?;

    Ok((from_account_id, to_account_id))
}

pub async fn check_idempotency_cache(
    state: &AppState,
    business_id: Uuid,
    idempotency_key: &str,
) -> Result<Option<TransferResponse>, Json<Value>> {
    let cached = sqlx::query(
        "SELECT response_body, status_code FROM idempotency_keys WHERE business_id = $1 AND key = $2"
    )
    .bind(business_id)
    .bind(idempotency_key)
    .fetch_optional(&state.pool)
    .await;

    if let Ok(Some(row)) = cached {
        let response_body: serde_json::Value = row.get("response_body");
        if let Ok(cached_response) = serde_json::from_value::<TransferResponse>(response_body) {
            return Ok(Some(cached_response));
        }
    }

    Ok(None)
}

pub async fn fetch_and_validate_accounts(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    from_account_id: Uuid,
    to_account_id: Uuid,
    business_id: Uuid,
    amount: i64,
) -> Result<(String, i64), Json<Value>> {
    let from_account = sqlx::query(
        "SELECT id, business_id, balance, currency FROM accounts WHERE id = $1 AND business_id = $2 FOR UPDATE"
    )
    .bind(from_account_id)
    .bind(business_id)
    .fetch_optional(&mut **tx)
    .await;

    let from_account = match from_account {
        Ok(Some(row)) => row,
        Ok(None) => {
            return Err(Json(
                json!({ "error": "Source account not found or does not belong to this business" }),
            ));
        }
        Err(_) => {
            return Err(Json(json!({ "error": "Failed to fetch source account" })));
        }
    };

    let to_account = sqlx::query(
        "SELECT id, business_id, balance, currency FROM accounts WHERE id = $1 FOR UPDATE",
    )
    .bind(to_account_id)
    .fetch_optional(&mut **tx)
    .await;

    let to_account = match to_account {
        Ok(Some(row)) => row,
        Ok(None) => {
            return Err(Json(json!({ "error": "Destination account not found" })));
        }
        Err(_) => {
            return Err(Json(
                json!({ "error": "Failed to fetch destination account" }),
            ));
        }
    };
    let from_currency: String = from_account.get("currency");
    let to_currency: String = to_account.get("currency");

    if from_currency != to_currency {
        return Err(Json(json!({
            "error": "Currency mismatch",
            "from_currency": from_currency,
            "to_currency": to_currency
        })));
    }

    let from_balance: i64 = from_account.get("balance");
    if from_balance < amount {
        return Err(Json(json!({
            "error": "Insufficient balance",
            "available": from_balance,
            "required": amount
        })));
    }

    Ok((from_currency, from_balance))
}

pub async fn execute_balance_transfer(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    from_account_id: Uuid,
    to_account_id: Uuid,
    amount: i64,
) -> Result<(), Json<Value>> {
    sqlx::query("UPDATE accounts SET balance = balance - $1 WHERE id = $2")
        .bind(amount)
        .bind(from_account_id)
        .execute(&mut **tx)
        .await
        .map_err(|_| Json(json!({ "error": "Failed to debit source account" })))?;

    sqlx::query("UPDATE accounts SET balance = balance + $1 WHERE id = $2")
        .bind(amount)
        .bind(to_account_id)
        .execute(&mut **tx)
        .await
        .map_err(|_| Json(json!({ "error": "Failed to credit destination account" })))?;

    Ok(())
}

pub async fn create_transaction_record(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    business_id: Uuid,
    from_account_id: Uuid,
    to_account_id: Uuid,
    amount: i64,
    idempotency_key: &Option<String>,
) -> Result<Uuid, Json<Value>> {
    let transaction_result = sqlx::query(
        "INSERT INTO transactions (business_id, from_account_id, to_account_id, amount, type, status, idempotency_key) 
         VALUES ($1, $2, $3, $4, 'transfer', 'success', $5) RETURNING id"
    )
    .bind(business_id)
    .bind(from_account_id)
    .bind(to_account_id)
    .bind(amount)
    .bind(idempotency_key)
    .fetch_one(&mut **tx)
    .await;

    let transaction_id = transaction_result
        .map(|row| row.get::<Uuid, _>("id"))
        .map_err(|_| Json(json!({ "error": "Failed to create transaction record" })))?;

    Ok(transaction_id)
}

pub async fn store_idempotency_key(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    business_id: Uuid,
    idempotency_key: &str,
    response: &TransferResponse,
) -> Result<(), Json<Value>> {
    let response_json = serde_json::to_value(response)
        .map_err(|_| Json(json!({ "error": "Failed to serialize response" })))?;

    sqlx::query(
        "INSERT INTO idempotency_keys (business_id, key, response_body, status_code) 
         VALUES ($1, $2, $3, 200)
         ON CONFLICT (business_id, key) DO NOTHING",
    )
    .bind(business_id)
    .bind(idempotency_key)
    .bind(response_json)
    .execute(&mut **tx)
    .await
    .map_err(|_| Json(json!({ "error": "Failed to store idempotency key" })))?;

    Ok(())
}
