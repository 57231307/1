-- 多租户 SaaS 支持数据库迁移

-- 租户套餐表
CREATE TABLE IF NOT EXISTS tenant_plans (
    id SERIAL PRIMARY KEY,
    code VARCHAR(50) UNIQUE NOT NULL,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    max_users INTEGER NOT NULL DEFAULT 10,
    max_storage_mb INTEGER NOT NULL DEFAULT 1024,
    max_api_calls_per_day INTEGER NOT NULL DEFAULT 10000,
    price_monthly DECIMAL(10,2) NOT NULL DEFAULT 0,
    price_yearly DECIMAL(10,2) NOT NULL DEFAULT 0,
    features TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- 插入默认套餐
INSERT INTO tenant_plans (code, name, description, max_users, max_storage_mb, max_api_calls_per_day, price_monthly, price_yearly, features, is_active) VALUES
('free', '免费版', '适合个人或小型团队试用', 3, 512, 1000, 0, 0, '["基础功能","社区支持"]', true),
('basic', '基础版', '适合小型企业', 10, 2048, 10000, 99, 999, '["基础功能","邮件支持","数据导出"]', true),
('professional', '专业版', '适合中型企业', 50, 10240, 100000, 299, 2999, '["全部功能","优先支持","API访问","自定义报表"]', true),
('enterprise', '企业版', '适合大型企业', 200, 51200, 1000000, 999, 9999, '["全部功能","专属客服","SLA保障","私有部署选项"]', true)
ON CONFLICT (code) DO NOTHING;

-- 租户表
CREATE TABLE IF NOT EXISTS tenants (
    id SERIAL PRIMARY KEY,
    code VARCHAR(50) UNIQUE NOT NULL,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    plan_id INTEGER REFERENCES tenant_plans(id),
    admin_user_id INTEGER REFERENCES users(id),
    db_schema VARCHAR(100),
    custom_domain VARCHAR(255),
    is_deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    expired_at TIMESTAMPTZ WITH TIME ZONE
);

-- 租户用户关联表
CREATE TABLE IF NOT EXISTS tenant_users (
    id SERIAL PRIMARY KEY,
    tenant_id INTEGER NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_in_tenant VARCHAR(50) NOT NULL DEFAULT 'MEMBER',
    is_primary BOOLEAN NOT NULL DEFAULT FALSE,
    joined_at TIMESTAMPTZ WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMPTZ WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(tenant_id, user_id)
);

-- 租户配置表
CREATE TABLE IF NOT EXISTS tenant_configs (
    id SERIAL PRIMARY KEY,
    tenant_id INTEGER NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    config_key VARCHAR(100) NOT NULL,
    config_value TEXT NOT NULL,
    config_type VARCHAR(20) NOT NULL DEFAULT 'STRING',
    description TEXT,
    created_at TIMESTAMPTZ WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(tenant_id, config_key)
);

-- 添加索引
CREATE INDEX IF NOT EXISTS idx_tenants_status ON tenants(status);
CREATE INDEX IF NOT EXISTS idx_tenants_plan ON tenants(plan_id);
CREATE INDEX IF NOT EXISTS idx_tenant_users_tenant ON tenant_users(tenant_id);
CREATE INDEX IF NOT EXISTS idx_tenant_users_user ON tenant_users(user_id);
CREATE INDEX IF NOT EXISTS idx_tenant_configs_tenant ON tenant_configs(tenant_id);
