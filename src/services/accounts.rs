use crate::models::{IdempotencyStatus, TransferRequest, TransferResponse};
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
        let status: IdempotencyStatus = row.get("status");

        if status == IdempotencyStatus::Success {
            if let Ok(response_body) = row.try_get::<serde_json::Value, _>("response_body") {
                if let Ok(cached_response) =
                    serde_json::from_value::<TransferResponse>(response_body)
                {
                    return Ok(Some(cached_response));
                }
            }
        }
    }

    Ok(None)
}

pub async fn reserve_idempotency_key(
    state: &AppState,
    business_id: Uuid,
    idempotency_key: &str,
) -> Result<(), Json<Value>> {
    // Try to insert as pending.
    // If it exists:
    //   if status_code is success (200) -> Return conflict/check cache (handler should have checked cache first)
    //   if status_code is pending (e.g. 202) -> Return conflict (in progress)
    //   if status_code is failed (e.g. 500) -> Allow update (retry)

    // We'll use IdempotencyStatus Enum values.

    let result = sqlx::query(
        "INSERT INTO idempotency_keys (business_id, key, status, created_at) 
         VALUES ($1, $2, 'pending'::idempotency_status, NOW())
         ON CONFLICT (business_id, key) DO UPDATE 
         SET created_at = NOW() 
         WHERE idempotency_keys.status != 'success'::idempotency_status AND idempotency_keys.status != 'pending'::idempotency_status",
    )
    .bind(business_id)
    .bind(idempotency_key)
    .execute(&state.pool)
    .await;

    match result {
        Ok(res) => {
            if res.rows_affected() == 0 {
                // If 0 rows affected, it means it existed and was Success or Pending.
                // We need to know which one.
                let existing = sqlx::query(
                    "SELECT status FROM idempotency_keys WHERE business_id = $1 AND key = $2",
                )
                .bind(business_id)
                .bind(idempotency_key)
                .fetch_optional(&state.pool)
                .await;

                match existing {
                    Ok(Some(row)) => {
                        let status: IdempotencyStatus = row.get("status");
                        if status == IdempotencyStatus::Pending {
                            return Err(Json(json!({ "error": "Operation in progress" })));
                        } else if status == IdempotencyStatus::Success {
                            // Should have been caught by cache check, but ok.
                            return Err(Json(
                                json!({ "error": "Operation already completed successfully" }),
                            ));
                        }
                    }
                    _ => {}
                }
                // If we are here, something weird happened or it was retriable but update didn't run?
                // Actually, the DO UPDATE WHERE clause prevents update if it's Success or Pending.
                // So if it was Failed, it would update.
            }
            Ok(())
        }
        Err(_) => Err(Json(
            json!({ "error": "Failed to reserve idempotency key" }),
        )),
    }
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
    idempotency_key: &str,
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

    // Update the pending key to success
    sqlx::query(
        "UPDATE idempotency_keys 
         SET response_body = $1, status = 'success'::idempotency_status 
         WHERE business_id = $2 AND key = $3",
    )
    .bind(response_json)
    .bind(business_id)
    .bind(idempotency_key)
    .execute(&mut **tx)
    .await
    .map_err(|_| Json(json!({ "error": "Failed to update idempotency key" })))?;

    Ok(())
}

pub async fn fail_idempotency_key(
    state: &AppState,
    business_id: Uuid,
    idempotency_key: &str,
) -> Result<(), Json<Value>> {
    sqlx::query(
        "UPDATE idempotency_keys 
         SET status = 'failed'::idempotency_status 
         WHERE business_id = $1 AND key = $2",
    )
    .bind(business_id)
    .bind(idempotency_key)
    .execute(&state.pool)
    .await
    .map_err(|_| Json(json!({ "error": "Failed to set idempotency key failure status" })))?;

    Ok(())
}

pub async fn create_webhook_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    business_id: Uuid,
    payload: &TransferResponse,
) -> Result<(), Json<Value>> {
    let payload_json = serde_json::to_value(payload)
        .map_err(|_| Json(json!({ "error": "Failed to serialize webhook payload" })))?;

    // Find active endpoints
    let endpoints =
        sqlx::query("SELECT id FROM webhook_endpoints WHERE business_id = $1 AND is_active = true")
            .bind(business_id)
            .fetch_all(&mut **tx)
            .await
            .map_err(|_| Json(json!({ "error": "Failed to fetch webhook endpoints" })))?;

    for endpoint in endpoints {
        let endpoint_id: Uuid = endpoint.get("id");
        sqlx::query(
            "INSERT INTO webhook_events (webhook_endpoint_id, event_type, payload) 
             VALUES ($1, 'transfer.created', $2)",
        )
        .bind(endpoint_id)
        .bind(&payload_json)
        .execute(&mut **tx)
        .await
        .map_err(|_| Json(json!({ "error": "Failed to create webhook event" })))?;
    }

    Ok(())
}
