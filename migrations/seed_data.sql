-- Seed data: Create 2 businesses with accounts and API keys
-- 
-- Business 1 Credentials:
--   Email: test_business_1@example.com
--   Password: password123
--   API Key: sk_live_test_business_1_key_12345678901234567890123456789012
--
-- Business 2 Credentials:
--   Email: test_business_2@example.com
--   Password: password123
--   API Key: sk_live_test_business_2_key_12345678901234567890123456789012
--
-- Both businesses have accounts with balances:
--   Business 1: USD account with 1,000,000 (cents), EUR account with 500,000 (cents)
--   Business 2: USD account with 2,000,000 (cents), GBP account with 750,000 (cents)

-- Business 1: test_business_1@example.com
INSERT INTO businesses (id, name, email, password_hash) VALUES
('11111111-1111-1111-1111-111111111111', 'Test Business 1', 'test_business_1@example.com', '$2b$12$ciACRkyJ1hi59MQXmbrrLeol8sslAg7oW.99xzHKXWkcsIXekx8Qu')
ON CONFLICT (email) DO UPDATE SET password_hash = EXCLUDED.password_hash;

-- Business 2: test_business_2@example.com
INSERT INTO businesses (id, name, email, password_hash) VALUES
('22222222-2222-2222-2222-222222222222', 'Test Business 2', 'test_business_2@example.com', '$2b$12$ciACRkyJ1hi59MQXmbrrLeol8sslAg7oW.99xzHKXWkcsIXekx8Qu')
ON CONFLICT (email) DO UPDATE SET password_hash = EXCLUDED.password_hash;

-- API Key for Business 1: sk_live_test_business_1_key_12345678901234567890123456789012
-- Hash: SHA256 of "sk_live_test_business_1_key_12345678901234567890123456789012"
INSERT INTO api_keys (business_id, key_hash, is_active) VALUES
('11111111-1111-1111-1111-111111111111', 'd2e057d4ab17c4311465a1942365ea78677fa2c79d19d90dfaa6346f76776b71', true)
ON CONFLICT (key_hash) DO NOTHING;

-- API Key for Business 2: sk_live_test_business_2_key_12345678901234567890123456789012
-- Hash: SHA256 of "sk_live_test_business_2_key_12345678901234567890123456789012"
INSERT INTO api_keys (business_id, key_hash, is_active) VALUES
('22222222-2222-2222-2222-222222222222', '68b94ec7ee1d1dd29341264e6c0d09be9223c1108b68ec0470203478546e24b7', true)
ON CONFLICT (key_hash) DO NOTHING;

-- Accounts for Business 1
INSERT INTO accounts (business_id, currency, balance) VALUES
('11111111-1111-1111-1111-111111111111', 'USD', 1000000),
('11111111-1111-1111-1111-111111111111', 'EUR', 500000);

-- Accounts for Business 2
INSERT INTO accounts (business_id, currency, balance) VALUES
('22222222-2222-2222-2222-222222222222', 'USD', 2000000),
('22222222-2222-2222-2222-222222222222', 'GBP', 750000);

