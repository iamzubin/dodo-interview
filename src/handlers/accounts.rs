use crate::models::{
    AccountResponse, CreateAccountRequest, CreditDebitRequest, CreditDebitResponse,
    GetAccountsQuery, TransferRequest, TransferResponse,
};
use crate::services::accounts::{
    check_idempotency_cache, create_cd_record, create_transaction_record, create_webhook_event,
    execute_balance_transfer, fail_idempotency_key, fetch_account, fetch_and_validate_accounts,
    reserve_idempotency_key, store_idempotency_key, update_balance, validate_cd_input,
    validate_transfer_input,
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
    // Determine business details first to ensure we can return them
    let row = sqlx::query("SELECT name, email FROM businesses WHERE id = $1")
        .bind(business_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| Json(json!({ "error": "Failed to fetch business details" })))?;

    let (business_name, business_email) = match row {
        Some(r) => (r.get("name"), r.get("email")),
        None => return Err(Json(json!({ "error": "Business not found" }))),
    };

    let result = sqlx::query(
        "INSERT INTO accounts (business_id, currency, balance) VALUES ($1, $2, 10000) RETURNING id, balance"
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
                business_name,
                business_email,
            }))
        }
        Err(_) => Err(Json(json!({ "error": "Failed to create account" }))),
    }
}

pub async fn get_accounts(
    State(state): State<AppState>,
    Query(params): Query<GetAccountsQuery>,
) -> Result<Json<Vec<AccountResponse>>, Json<Value>> {
    let mut query_str =
        String::from("SELECT a.id, a.business_id, a.balance, a.currency, b.name as business_name, b.email as business_email 
                      FROM accounts a 
                      JOIN businesses b ON a.business_id = b.id 
                      WHERE 1=1");
    let mut conditions = Vec::new();

    if params.currency.is_some() {
        conditions.push("a.currency");
    }
    if params.business_id.is_some() {
        conditions.push("a.business_id");
    }

    for (idx, condition) in conditions.iter().enumerate() {
        query_str.push_str(&format!(" AND {} = ${}", condition, idx + 1));
    }

    query_str.push_str(" ORDER BY a.created_at");

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
                    business_name: row.get("business_name"),
                    business_email: row.get("business_email"),
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
    let (from_account_id, to_account_id) = validate_transfer_input(&payload)?;

    if let Some(mut cached_response) =
        check_idempotency_cache::<TransferResponse>(&state, business_id, &payload.idempotency_key)
            .await?
    {
        cached_response.cached = Some(true);
        return Ok(Json(cached_response));
    }

    // Reserve idempotency key
    reserve_idempotency_key(&state, business_id, &payload.idempotency_key).await?;

    let process_transfer = async {
        let mut tx = state
            .pool
            .begin()
            .await
            .map_err(|_| Json(json!({ "error": "Failed to start transaction" })))?;

        let (currency, _) = fetch_and_validate_accounts(
            &mut tx,
            from_account_id,
            to_account_id,
            business_id,
            payload.amount,
        )
        .await?;

        execute_balance_transfer(&mut tx, from_account_id, to_account_id, payload.amount).await?;

        let transaction_id = create_transaction_record(
            &mut tx,
            business_id,
            from_account_id,
            to_account_id,
            payload.amount,
            &payload.idempotency_key,
        )
        .await?;

        let response = TransferResponse {
            transaction_id: transaction_id.to_string(),
            from_account_id: payload.from_account_id.clone(),
            to_account_id: payload.to_account_id.clone(),
            amount: payload.amount,
            currency,
            status: "success".to_string(),
            cached: None,
        };

        create_webhook_event(&mut tx, business_id, "transfer.created", &response).await?;

        store_idempotency_key(&mut tx, business_id, &payload.idempotency_key, &response).await?;

        tx.commit()
            .await
            .map_err(|_| Json(json!({ "error": "Failed to commit transaction" })))?;

        Ok(Json(response))
    };

    match process_transfer.await {
        Ok(response) => Ok(response),
        Err(err) => {
            let _ = fail_idempotency_key(&state, business_id, &payload.idempotency_key).await;
            Err(err)
        }
    }
}

pub async fn credit_debit(
    State(state): State<AppState>,
    Extension(business_id): Extension<Uuid>,
    Json(payload): Json<CreditDebitRequest>,
) -> Result<Json<CreditDebitResponse>, Json<Value>> {
    let account_id = validate_cd_input(&payload)?;

    // Check idempotency cache
    if let Some(mut cached_response) = check_idempotency_cache::<CreditDebitResponse>(
        &state,
        business_id,
        &payload.idempotency_key,
    )
    .await?
    {
        cached_response.cached = Some(true);
        return Ok(Json(cached_response));
    }

    // Reserve idempotency key
    reserve_idempotency_key(&state, business_id, &payload.idempotency_key).await?;

    let is_credit = payload.transaction_type == "credit";

    let process_cd = async {
        let mut tx = state
            .pool
            .begin()
            .await
            .map_err(|_| Json(json!({ "error": "Failed to start transaction" })))?;

        // Fetch and validate account
        let (currency, current_balance) = fetch_account(&mut tx, account_id, business_id).await?;

        // For debit, check sufficient balance
        if !is_credit && current_balance < payload.amount {
            return Err(Json(json!({
                "error": "Insufficient balance",
                "available": current_balance,
                "required": payload.amount
            })));
        }

        // Update balance
        let new_balance = update_balance(&mut tx, account_id, payload.amount, is_credit).await?;

        // Create transaction record
        let transaction_id = create_cd_record(
            &mut tx,
            business_id,
            account_id,
            payload.amount,
            &payload.transaction_type,
            &payload.idempotency_key,
        )
        .await?;

        let response = CreditDebitResponse {
            transaction_id: transaction_id.to_string(),
            account_id: payload.account_id.clone(),
            amount: payload.amount,
            currency,
            transaction_type: payload.transaction_type.clone(),
            status: "success".to_string(),
            new_balance,
            cached: None,
        };

        // Create webhook event
        let event_type = format!("{}.created", payload.transaction_type);
        create_webhook_event(&mut tx, business_id, &event_type, &response).await?;

        // Store idempotency key with response
        store_idempotency_key(&mut tx, business_id, &payload.idempotency_key, &response).await?;

        tx.commit()
            .await
            .map_err(|_| Json(json!({ "error": "Failed to commit transaction" })))?;

        Ok(Json(response))
    };

    match process_cd.await {
        Ok(response) => Ok(response),
        Err(err) => {
            let _ = fail_idempotency_key(&state, business_id, &payload.idempotency_key).await;
            Err(err)
        }
    }
}
