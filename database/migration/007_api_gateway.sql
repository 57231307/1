-- API 网关相关表迁移

-- Webhook 订阅表
CREATE TABLE IF NOT EXISTS webhooks (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    url VARCHAR(500) NOT NULL,
    events VARCHAR(500) NOT NULL,
    secret VARCHAR(255),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    last_triggered_at TIMESTAMPTZ WITH TIME ZONE,
    last_status VARCHAR(50),
    retry_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- API 密钥表
CREATE TABLE IF NOT EXISTS api_keys (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    key_hash VARCHAR(64) NOT NULL,
    key_prefix VARCHAR(20) NOT NULL,
    permissions TEXT,
    rate_limit_per_minute INTEGER NOT NULL DEFAULT 100,
    last_used_at TIMESTAMPTZ WITH TIME ZONE,
    expires_at TIMESTAMPTZ WITH TIME ZONE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- 添加索引
CREATE INDEX IF NOT EXISTS idx_webhooks_active ON webhooks(is_active);
CREATE INDEX IF NOT EXISTS idx_api_keys_hash ON api_keys(key_hash);
CREATE INDEX IF NOT EXISTS idx_api_keys_active ON api_keys(is_active);
