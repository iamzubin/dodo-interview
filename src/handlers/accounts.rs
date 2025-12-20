use crate::models::{
    AccountResponse, CreateAccountRequest, GetAccountsQuery, TransferRequest, TransferResponse,
};
use crate::services::accounts::{
    check_idempotency_cache, create_transaction_record, execute_balance_transfer,
    fetch_and_validate_accounts, store_idempotency_key, validate_transfer_input,
};
use crate::state::AppState;
use axum::{
    extract::{Extension, Query, State},
    Json,
};
use serde_json::{json, Value};
use sqlx::{types::Uuid, Row};

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

pub async fn get_accounts(
    State(state): State<AppState>,
    Query(params): Query<GetAccountsQuery>,
) -> Result<Json<Vec<AccountResponse>>, Json<Value>> {
    // Build dynamic query based on optional filters
    let mut query_str =
        String::from("SELECT id, business_id, balance, currency FROM accounts WHERE 1=1");
    let mut conditions = Vec::new();

    if params.currency.is_some() {
        conditions.push("currency");
    }
    if params.business_id.is_some() {
        conditions.push("business_id");
    }

    for (idx, condition) in conditions.iter().enumerate() {
        query_str.push_str(&format!(" AND {} = ${}", condition, idx + 1));
    }

    query_str.push_str(" ORDER BY created_at");

    // Build query with bindings
    let mut query = sqlx::query(&query_str);

    if let Some(ref currency) = params.currency {
        query = query.bind(currency);
    }
    if let Some(ref business_id_str) = params.business_id {
        match Uuid::parse_str(business_id_str) {
            Ok(business_id) => {
                query = query.bind(business_id);
            }
            Err(_) => {
                return Err(Json(json!({ "error": "Invalid business_id format" })));
            }
        }
    }

    let result = query.fetch_all(&state.pool).await;

    match result {
        Ok(rows) => {
            let accounts: Vec<AccountResponse> = rows
                .into_iter()
                .map(|row| AccountResponse {
                    id: row.get::<Uuid, _>("id").to_string(),
                    business_id: row.get::<Uuid, _>("business_id").to_string(),
                    balance: row.get("balance"),
                    currency: row.get("currency"),
                })
                .collect();
            Ok(Json(accounts))
        }
        Err(_) => Err(Json(json!({ "error": "Failed to fetch accounts" }))),
    }
}

pub async fn transfer(
    State(state): State<AppState>,
    Extension(business_id): Extension<Uuid>,
    Json(payload): Json<TransferRequest>,
) -> Result<Json<TransferResponse>, Json<Value>> {
    // Validate input and parse account IDs
    let (from_account_id, to_account_id) = validate_transfer_input(&payload)?;

    // Check for cached idempotent response
    if let Some(ref idempotency_key) = payload.idempotency_key {
        if let Some(cached_response) =
            check_idempotency_cache(&state, business_id, idempotency_key).await?
        {
            return Ok(Json(cached_response));
        }
    }

    // Start a transaction
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|_| Json(json!({ "error": "Failed to start transaction" })))?;

    // Fetch and validate accounts, check balance and currency
    let (currency, _) = match fetch_and_validate_accounts(
        &mut tx,
        from_account_id,
        to_account_id,
        business_id,
        payload.amount,
    )
    .await
    {
        Ok(result) => result,
        Err(err) => {
            let _ = tx.rollback().await;
            return Err(err);
        }
    };

    // Execute the balance transfer
    if let Err(err) =
        execute_balance_transfer(&mut tx, from_account_id, to_account_id, payload.amount).await
    {
        let _ = tx.rollback().await;
        return Err(err);
    }

    // Create transaction record
    let transaction_id = match create_transaction_record(
        &mut tx,
        business_id,
        from_account_id,
        to_account_id,
        payload.amount,
        &payload.idempotency_key,
    )
    .await
    {
        Ok(id) => id,
        Err(err) => {
            let _ = tx.rollback().await;
            return Err(err);
        }
    };

    // Prepare response
    let response = TransferResponse {
        transaction_id: transaction_id.to_string(),
        from_account_id: payload.from_account_id.clone(),
        to_account_id: payload.to_account_id.clone(),
        amount: payload.amount,
        currency,
        status: "success".to_string(),
    };

    // Store idempotency key if provided
    if let Some(ref idempotency_key) = payload.idempotency_key {
        if let Err(err) =
            store_idempotency_key(&mut tx, business_id, idempotency_key, &response).await
        {
            let _ = tx.rollback().await;
            return Err(err);
        }
    }

    // Commit transaction
    tx.commit()
        .await
        .map_err(|_| Json(json!({ "error": "Failed to commit transaction" })))?;

    Ok(Json(response))
}
