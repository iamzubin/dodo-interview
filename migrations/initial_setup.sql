-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS businesses (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name            TEXT,
    email           TEXT UNIQUE NOT NULL,
    password_hash   TEXT NOT NULL,
    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);


CREATE TABLE IF NOT EXISTS api_keys (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    business_id     UUID NOT NULL REFERENCES businesses(id) ON DELETE CASCADE,
    key_hash        TEXT UNIQUE NOT NULL,
    is_active       BOOLEAN DEFAULT TRUE,
    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_api_keys_business_id ON api_keys(business_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_key_hash ON api_keys(key_hash);


CREATE TABLE IF NOT EXISTS accounts (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    business_id     UUID NOT NULL REFERENCES businesses(id) ON DELETE CASCADE,
    balance         BIGINT DEFAULT 0 NOT NULL,
    currency        TEXT NOT NULL,
    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_accounts_business_id ON accounts(business_id);


CREATE TABLE IF NOT EXISTS transactions (
    id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    business_id         UUID NOT NULL REFERENCES businesses(id) ON DELETE CASCADE,
    from_account_id     UUID REFERENCES accounts(id) ON DELETE SET NULL,
    to_account_id       UUID REFERENCES accounts(id) ON DELETE SET NULL,
    amount              BIGINT NOT NULL,
    type                TEXT NOT NULL, -- credit | debit | transfer
    status              TEXT NOT NULL, -- success | failed
    idempotency_key     TEXT,
    created_at          TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    UNIQUE (business_id, idempotency_key)
);

CREATE INDEX IF NOT EXISTS idx_transactions_business_id ON transactions(business_id);
CREATE INDEX IF NOT EXISTS idx_transactions_from_account_id ON transactions(from_account_id);
CREATE INDEX IF NOT EXISTS idx_transactions_to_account_id ON transactions(to_account_id);
CREATE INDEX IF NOT EXISTS idx_transactions_idempotency_key ON transactions(business_id, idempotency_key);


CREATE TABLE IF NOT EXISTS idempotency_keys (
    business_id     UUID NOT NULL REFERENCES businesses(id) ON DELETE CASCADE,
    key             TEXT NOT NULL,
    response_body   JSONB,
    status_code     INT,
    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    PRIMARY KEY (business_id, key)
);

CREATE INDEX IF NOT EXISTS idx_idempotency_keys_business_id ON idempotency_keys(business_id);

CREATE TABLE IF NOT EXISTS webhook_endpoints (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    business_id     UUID NOT NULL REFERENCES businesses(id) ON DELETE CASCADE,
    url             TEXT NOT NULL,
    secret          TEXT NOT NULL,
    is_active       BOOLEAN DEFAULT TRUE,
    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_webhook_endpoints_business_id ON webhook_endpoints(business_id);

CREATE TABLE IF NOT EXISTS webhook_events (
    id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    webhook_endpoint_id UUID NOT NULL REFERENCES webhook_endpoints(id) ON DELETE CASCADE,
    event_type          TEXT NOT NULL,
    payload             JSONB NOT NULL,
    status              TEXT NOT NULL DEFAULT 'pending', -- pending | delivered | failed
    attempts            INT DEFAULT 0,
    last_attempt_at     TIMESTAMP,
    created_at          TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_webhook_events_webhook_endpoint_id ON webhook_events(webhook_endpoint_id);
CREATE INDEX IF NOT EXISTS idx_webhook_events_status ON webhook_events(status);
