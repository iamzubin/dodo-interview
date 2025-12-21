# Simple Transaction Service

A Rust-based transaction service with API authentication, account management, atomic transactions, and webhook notifications.

## Quick Start

```bash
# Start all services (PostgreSQL, App, Webhook Consumer)
docker compose up

# Wait for services to be ready, then run migrations
docker exec -i dodo_postgres psql -U dodo -d dodo < migrations/initial_setup.sql
docker exec -i dodo_postgres psql -U dodo -d dodo < migrations/seed_data.sql
```

The API is now available at `http://localhost:3000`

## Documentation

- **[DESIGN.md](./DESIGN.md)** — Architecture, design decisions, schema, trade-offs
- **[API.md](./API.md)** — Full API documentation with request/response examples

## Example Requests

### Health Check

```bash
curl http://localhost:3000/
```
```json
{"status": "healthy", "database": "connected"}
```

### Sign Up

```bash
curl -X POST http://localhost:3000/auth/signup \
  -H "Content-Type: application/json" \
  -d '{"email": "demo@example.com", "password": "secret123", "name": "Demo Business"}'
```

### Generate API Key

```bash
curl -X POST http://localhost:3000/auth/generate-api-key \
  -H "Content-Type: application/json" \
  -d '{"email": "demo@example.com", "password": "secret123"}'
```
```json
{"api_key": "sk_live_abc123..."}
```

### Create Account

```bash
curl -X POST http://localhost:3000/accounts/create \
  -H "Authorization: sk_live_abc123..." \
  -H "Content-Type: application/json" \
  -d '{"currency": "USD"}'
```

### List Accounts

```bash
curl http://localhost:3000/accounts
```

### Transfer Funds

```bash
curl -X POST http://localhost:3000/accounts/transfer \
  -H "Authorization: sk_live_abc123..." \
  -H "Content-Type: application/json" \
  -d '{
    "from_account_id": "<uuid>",
    "to_account_id": "<uuid>",
    "amount": 5000,
    "idempotency_key": "txn_001"
  }'
```

### Credit/Debit

```bash
curl -X POST http://localhost:3000/accounts/credit-debit \
  -H "Authorization: sk_live_abc123..." \
  -H "Content-Type: application/json" \
  -d '{
    "account_id": "<uuid>",
    "amount": 10000,
    "transaction_type": "credit",
    "idempotency_key": "cd_001"
  }'
```

### Register Webhook

```bash
curl -X POST http://localhost:3000/webhooks/register \
  -H "Authorization: sk_live_abc123..." \
  -H "Content-Type: application/json" \
  -d '{"url": "http://webhook_consumer:8000/", "secret": "my-secret"}'
```

## Test Credentials

Seed data provides pre-configured test accounts:

| Email | Password | API Key |
|-------|----------|---------|
| `test_business_1@example.com` | `password123` | `sk_live_test_business_1_key_12345678901234567890123456789012` |
| `test_business_2@example.com` | `password123` | `sk_live_test_business_2_key_12345678901234567890123456789012` |

## Interactive Testing

Open `api-test.html` in your browser while the server is running for a visual API tester with:

- **Signup & API Key Generation** — Create accounts and generate keys with one click
- **Account Management** — Create accounts and view all balances
- **Transfers** — Execute transfers with auto-generated idempotency keys
- **Credit/Debit** — Add or withdraw funds from accounts
- **Webhook Registration** — Register and list webhook endpoints
- **Response Viewer** — See formatted JSON responses for each request

The API key is automatically saved in localStorage and used for authenticated requests.

## Watching Webhooks

```bash
docker compose logs -f webhook_consumer
```

## Tech Stack

- **Framework**: Axum
- **Database**: PostgreSQL 16
- **Language**: Rust
