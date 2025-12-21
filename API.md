# API Documentation

## Base URL

```
http://localhost:3000
```

## Authentication

Protected endpoints require an API key in the `Authorization` header:

```
Authorization: sk_live_<64-character-hex-string>
```

---

## Endpoints

### Health Check

Check if the service is running and database is connected.

```http
GET /
```

**Response** `200 OK`
```json
{
  "status": "healthy",
  "database": "connected"
}
```

**Response** (database disconnected)
```json
{
  "status": "unhealthy",
  "database": "disconnected",
  "error": "connection refused"
}
```

---

## Authentication Endpoints

### Sign Up

Create a new business account.

```http
POST /auth/signup
Content-Type: application/json
```

**Request Body**
```json
{
  "email": "user@example.com",
  "password": "securepassword",
  "name": "My Business"
}
```

**Response** `200 OK`
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "email": "user@example.com",
  "name": "My Business"
}
```

**Error Responses**

| Status | Body | Condition |
|--------|------|-----------|
| `200` | `{"error": "Email already exists"}` | Duplicate email |
| `200` | `{"error": "Failed to create business"}` | Database error |

---

### Generate API Key

Authenticate with email/password to generate an API key.

```http
POST /auth/generate-api-key
Content-Type: application/json
```

**Request Body**
```json
{
  "email": "user@example.com",
  "password": "securepassword"
}
```

**Response** `200 OK`
```json
{
  "api_key": "sk_live_a1b2c3d4e5f6..."
}
```

> ⚠️ **Important**: Store this key securely. It cannot be retrieved again.

**Error Responses**

| Status | Body | Condition |
|--------|------|-----------|
| `200` | `{"error": "Invalid credentials"}` | Wrong email/password |
| `200` | `{"error": "Database error"}` | Database error |

---

## Account Endpoints

### List Accounts

List all accounts. Optionally filter by currency or business.

```http
GET /accounts
GET /accounts?currency=USD
GET /accounts?business_id=550e8400-e29b-41d4-a716-446655440000
```

**Response** `200 OK`
```json
[
  {
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "business_id": "550e8400-e29b-41d4-a716-446655440000",
    "business_name": "My Business",
    "business_email": "user@example.com",
    "balance": 1000000,
    "currency": "USD"
  }
]
```

> **Note**: Balance is in smallest currency unit (e.g., cents). `1000000` = $10,000.00

---

### Create Account

Create a new account for the authenticated business.

```http
POST /accounts/create
Authorization: sk_live_...
Content-Type: application/json
```

**Request Body**
```json
{
  "currency": "USD"
}
```

**Response** `200 OK`
```json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "business_id": "550e8400-e29b-41d4-a716-446655440000",
  "business_name": "My Business",
  "business_email": "user@example.com",
  "balance": 10000,
  "currency": "USD"
}
```

> **Note**: New accounts are created with an initial balance of 10000 (100.00 in currency units).

**Error Responses**

| Status | Body | Condition |
|--------|------|-----------|
| `401` | `Unauthorized` | Missing/invalid API key |
| `200` | `{"error": "Business not found"}` | Business deleted |

---

## Transaction Endpoints

### Transfer

Transfer funds between two accounts. Source account must belong to the authenticated business.

```http
POST /accounts/transfer
Authorization: sk_live_...
Content-Type: application/json
```

**Request Body**
```json
{
  "from_account_id": "123e4567-e89b-12d3-a456-426614174000",
  "to_account_id": "987fcdeb-51a2-3bc4-d567-890123456789",
  "amount": 50000,
  "idempotency_key": "txn_001"
}
```

**Response** `200 OK`
```json
{
  "transaction_id": "abcd1234-ef56-7890-abcd-ef1234567890",
  "from_account_id": "123e4567-e89b-12d3-a456-426614174000",
  "to_account_id": "987fcdeb-51a2-3bc4-d567-890123456789",
  "amount": 50000,
  "currency": "USD",
  "status": "success"
}
```

**Idempotent Response** (same idempotency_key)
```json
{
  "transaction_id": "abcd1234-ef56-7890-abcd-ef1234567890",
  "from_account_id": "123e4567-e89b-12d3-a456-426614174000",
  "to_account_id": "987fcdeb-51a2-3bc4-d567-890123456789",
  "amount": 50000,
  "currency": "USD",
  "status": "success",
  "cached": true
}
```

**Error Responses**

| Status | Body | Condition |
|--------|------|-----------|
| `401` | `Unauthorized` | Missing/invalid API key |
| `200` | `{"error": "Amount must be positive"}` | amount ≤ 0 |
| `200` | `{"error": "Invalid from_account_id format"}` | Invalid UUID |
| `200` | `{"error": "Source account not found or does not belong to this business"}` | Wrong ownership |
| `200` | `{"error": "Destination account not found"}` | Invalid destination |
| `200` | `{"error": "Currency mismatch", "from_currency": "USD", "to_currency": "EUR"}` | Different currencies |
| `200` | `{"error": "Insufficient balance", "available": 500, "required": 1000}` | Not enough funds |
| `200` | `{"error": "Operation in progress"}` | Concurrent request with same key |

---

### Credit / Debit

Add funds (credit) or withdraw funds (debit) from an account.

```http
POST /accounts/credit-debit
Authorization: sk_live_...
Content-Type: application/json
```

**Request Body**
```json
{
  "account_id": "123e4567-e89b-12d3-a456-426614174000",
  "amount": 25000,
  "transaction_type": "credit",
  "idempotency_key": "cd_001"
}
```

| Field | Type | Values |
|-------|------|--------|
| `transaction_type` | string | `"credit"` or `"debit"` |

**Response** `200 OK`
```json
{
  "transaction_id": "abcd1234-ef56-7890-abcd-ef1234567890",
  "account_id": "123e4567-e89b-12d3-a456-426614174000",
  "amount": 25000,
  "currency": "USD",
  "transaction_type": "credit",
  "status": "success",
  "new_balance": 125000
}
```

**Error Responses**

| Status | Body | Condition |
|--------|------|-----------|
| `401` | `Unauthorized` | Missing/invalid API key |
| `200` | `{"error": "Amount must be positive"}` | amount ≤ 0 |
| `200` | `{"error": "Invalid transaction_type. Must be 'credit' or 'debit'"}` | Bad type |
| `200` | `{"error": "Account not found or does not belong to this business"}` | Wrong ownership |
| `200` | `{"error": "Insufficient balance", "available": 500, "required": 1000}` | Debit exceeds balance |

---

## Webhook Endpoints

### Register Webhook

Register a URL to receive transaction notifications.

```http
POST /webhooks/register
Authorization: sk_live_...
Content-Type: application/json
```

**Request Body**
```json
{
  "url": "https://example.com/webhook",
  "secret": "my-webhook-secret"
}
```

**Response** `200 OK`
```json
{
  "id": "webhook-uuid",
  "business_id": "550e8400-e29b-41d4-a716-446655440000",
  "url": "https://example.com/webhook",
  "is_active": true
}
```

---

### List Webhooks

List all registered webhooks for the authenticated business.

```http
GET /webhooks/list
Authorization: sk_live_...
```

**Response** `200 OK`
```json
[
  {
    "id": "webhook-uuid",
    "business_id": "550e8400-e29b-41d4-a716-446655440000",
    "url": "https://example.com/webhook",
    "is_active": true
  }
]
```

---

## Webhook Delivery

When a transaction occurs, registered webhooks receive a POST request:

```http
POST <your-webhook-url>
Content-Type: application/json
X-Webhook-Secret: <your-secret>
```

**Payload** (Transfer)
```json
{
  "transaction_id": "abcd1234-ef56-7890-abcd-ef1234567890",
  "from_account_id": "123e4567-e89b-12d3-a456-426614174000",
  "to_account_id": "987fcdeb-51a2-3bc4-d567-890123456789",
  "amount": 50000,
  "currency": "USD",
  "status": "success"
}
```

**Payload** (Credit/Debit)
```json
{
  "transaction_id": "abcd1234-ef56-7890-abcd-ef1234567890",
  "account_id": "123e4567-e89b-12d3-a456-426614174000",
  "amount": 25000,
  "currency": "USD",
  "transaction_type": "credit",
  "status": "success",
  "new_balance": 125000
}
```

**Event Types**
- `transfer.created`
- `credit.created`
- `debit.created`

**Retry Policy**
- Up to 5 attempts
- Exponential backoff: 10s, 20s, 30s, 40s, 50s
- Marked as `failed` after 5 unsuccessful attempts

---

## Test Credentials

For local development, seed data provides test accounts:

| Business | Email | Password | API Key |
|----------|-------|----------|---------|
| Test Business 1 | `test_business_1@example.com` | `password123` | `sk_live_test_business_1_key_12345678901234567890123456789012` |
| Test Business 2 | `test_business_2@example.com` | `password123` | `sk_live_test_business_2_key_12345678901234567890123456789012` |

Pre-seeded account balances:
- Business 1: USD (1,000,000 cents = $10,000), EUR (500,000 cents = €5,000)
- Business 2: USD (2,000,000 cents = $20,000), GBP (750,000 cents = £7,500)
