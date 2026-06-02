-- Migration 030: 创建增强日志表
-- 用于存储详细的业务日志、安全事件、性能监控等

-- 1. 资金操作详细日志表
CREATE TABLE IF NOT EXISTS "financial_audit_logs" (
    "id" BIGSERIAL PRIMARY KEY,
    "tenant_id" INTEGER NOT NULL DEFAULT 1,
    "trace_id" VARCHAR(100),
    "user_id" INTEGER,
    "username" VARCHAR(100),
    "operation" VARCHAR(50) NOT NULL,           -- APPROVE, REJECT, CANCEL, CREATE
    "financial_type" VARCHAR(50) NOT NULL,      -- AP_INVOICE, AR_INVOICE, PAYMENT, VOUCHER, FUND
    "financial_id" INTEGER NOT NULL,
    "financial_no" VARCHAR(100) NOT NULL,
    "amount" DECIMAL(15,2),
    "currency" VARCHAR(10) DEFAULT 'CNY',
    "exchange_rate" DECIMAL(10,4) DEFAULT 1.0,
    "amount_cny" DECIMAL(15,2),
    "operator_user_id" INTEGER,
    "operator_username" VARCHAR(100),
    "operator_ip" VARCHAR(50),
    "operator_department" VARCHAR(100),
    "related_type" VARCHAR(50),                 -- 关联类型（采购订单、销售订单等）
    "related_id" INTEGER,
    "related_no" VARCHAR(100),
    "supplier_id" INTEGER,
    "supplier_name" VARCHAR(200),
    "customer_id" INTEGER,
    "customer_name" VARCHAR(200),
    "payment_method" VARCHAR(50),
    "bank_account" VARCHAR(100),
    "due_date" DATE,
    "invoice_ids" JSONB,
    "before_status" VARCHAR(50),
    "after_status" VARCHAR(50),
    "approval_level" INTEGER,
    "approver_comments" TEXT,
    "context" JSONB,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 2. 权限变更详细日志表
CREATE TABLE IF NOT EXISTS "permission_audit_logs" (
    "id" BIGSERIAL PRIMARY KEY,
    "tenant_id" INTEGER NOT NULL DEFAULT 1,
    "trace_id" VARCHAR(100),
    "operator_user_id" INTEGER NOT NULL,
    "operator_username" VARCHAR(100) NOT NULL,
    "operator_ip" VARCHAR(50),
    "operation" VARCHAR(50) NOT NULL,           -- ASSIGN, REVOKE, UPDATE
    "target_user_id" INTEGER,
    "target_username" VARCHAR(100),
    "target_roles" JSONB,
    "before_roles" JSONB,
    "after_roles" JSONB,
    "before_permissions" JSONB,
    "after_permissions" JSONB,
    "roles_added" JSONB,
    "roles_removed" JSONB,
    "permissions_changed" JSONB,
    "context" JSONB,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 3. 安全事件详细日志表
CREATE TABLE IF NOT EXISTS "security_audit_logs" (
    "id" BIGSERIAL PRIMARY KEY,
    "tenant_id" INTEGER NOT NULL DEFAULT 1,
    "trace_id" VARCHAR(100),
    "event" VARCHAR(50) NOT NULL,               -- LOGIN_SUCCESS, LOGIN_FAILURE, LOGOUT, PASSWORD_CHANGE
    "username" VARCHAR(100),
    "user_id" INTEGER,
    "ip_address" VARCHAR(50),
    "user_agent" TEXT,
    "login_method" VARCHAR(50),                 -- password, sso, api_key
    "login_type" VARCHAR(50),                   -- web, mobile, api
    "failure_reason" TEXT,
    "attempts_today" INTEGER,
    "attempts_total" INTEGER,
    "last_success" TIMESTAMPTZ,
    "last_failure" TIMESTAMPTZ,
    "risk_level" VARCHAR(20),                   -- LOW, MEDIUM, HIGH, CRITICAL
    "risk_factors" JSONB,
    "blocked" BOOLEAN DEFAULT false,
    "block_reason" TEXT,
    "require_captcha" BOOLEAN DEFAULT false,
    "notify_user" BOOLEAN DEFAULT false,
    "geo_country" VARCHAR(50),
    "geo_region" VARCHAR(100),
    "geo_city" VARCHAR(100),
    "geo_isp" VARCHAR(100),
    "device_os" VARCHAR(100),
    "device_browser" VARCHAR(100),
    "device_type" VARCHAR(50),
    "is_mobile" BOOLEAN DEFAULT false,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 4. 性能监控日志表
CREATE TABLE IF NOT EXISTS "performance_logs" (
    "id" BIGSERIAL PRIMARY KEY,
    "tenant_id" INTEGER NOT NULL DEFAULT 1,
    "trace_id" VARCHAR(100),
    "endpoint" VARCHAR(500) NOT NULL,
    "method" VARCHAR(10) NOT NULL,
    "user_id" INTEGER,
    "total_duration_ms" INTEGER NOT NULL,
    "db_duration_ms" INTEGER,
    "cache_duration_ms" INTEGER,
    "external_duration_ms" INTEGER,
    "serialization_duration_ms" INTEGER,
    "middleware_duration_ms" INTEGER,
    "db_queries_count" INTEGER,
    "db_slow_queries" INTEGER,
    "db_connection_pool_active" INTEGER,
    "db_connection_pool_idle" INTEGER,
    "db_connection_pool_waiting" INTEGER,
    "cache_hits" JSONB,
    "cache_misses" JSONB,
    "cache_hit_rate" DECIMAL(5,4),
    "memory_allocated_mb" DECIMAL(10,2),
    "memory_peak_mb" DECIMAL(10,2),
    "gc_count" INTEGER,
    "response_status" INTEGER,
    "response_size_bytes" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 5. 系统健康日志表
CREATE TABLE IF NOT EXISTS "system_health_logs" (
    "id" BIGSERIAL PRIMARY KEY,
    "tenant_id" INTEGER NOT NULL DEFAULT 1,
    "cpu_usage_percent" DECIMAL(5,2),
    "memory_usage_percent" DECIMAL(5,2),
    "disk_usage_percent" DECIMAL(5,2),
    "load_average_1m" DECIMAL(5,2),
    "load_average_5m" DECIMAL(5,2),
    "load_average_15m" DECIMAL(5,2),
    "uptime_seconds" BIGINT,
    "db_status" VARCHAR(20),
    "db_connections_active" INTEGER,
    "db_connections_idle" INTEGER,
    "db_connections_max" INTEGER,
    "db_connections_waiting" INTEGER,
    "db_replication_lag_ms" INTEGER,
    "db_query_time_avg_ms" DECIMAL(10,2),
    "cache_status" VARCHAR(20),
    "cache_memory_used_mb" DECIMAL(10,2),
    "cache_memory_max_mb" DECIMAL(10,2),
    "cache_hit_rate" DECIMAL(5,4),
    "cache_evictions" BIGINT,
    "app_version" VARCHAR(50),
    "app_environment" VARCHAR(20),
    "app_active_users" INTEGER,
    "app_requests_per_minute" INTEGER,
    "app_error_rate_percent" DECIMAL(5,2),
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 6. 业务操作详细日志表
CREATE TABLE IF NOT EXISTS "business_audit_logs" (
    "id" BIGSERIAL PRIMARY KEY,
    "tenant_id" INTEGER NOT NULL DEFAULT 1,
    "trace_id" VARCHAR(100),
    "module" VARCHAR(100) NOT NULL,
    "operation" VARCHAR(100) NOT NULL,
    "resource_type" VARCHAR(100),
    "resource_id" VARCHAR(100),
    "resource_name" VARCHAR(200),
    "operator_user_id" INTEGER,
    "operator_username" VARCHAR(100),
    "operator_ip" VARCHAR(50),
    "action_details" JSONB,
    "before_data" JSONB,
    "after_data" JSONB,
    "diff_data" JSONB,
    "context" JSONB,
    "success" BOOLEAN DEFAULT true,
    "affected_rows" INTEGER,
    "generated_id" INTEGER,
    "error_message" TEXT,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_financial_audit_logs_tenant_id ON "financial_audit_logs"("tenant_id");
CREATE INDEX IF NOT EXISTS idx_financial_audit_logs_user_id ON "financial_audit_logs"("user_id");
CREATE INDEX IF NOT EXISTS idx_financial_audit_logs_operation ON "financial_audit_logs"("operation");
CREATE INDEX IF NOT EXISTS idx_financial_audit_logs_financial_type ON "financial_audit_logs"("financial_type");
CREATE INDEX IF NOT EXISTS idx_financial_audit_logs_financial_no ON "financial_audit_logs"("financial_no");
CREATE INDEX IF NOT EXISTS idx_financial_audit_logs_created_at ON "financial_audit_logs"("created_at");

CREATE INDEX IF NOT EXISTS idx_permission_audit_logs_tenant_id ON "permission_audit_logs"("tenant_id");
CREATE INDEX IF NOT EXISTS idx_permission_audit_logs_operator_user_id ON "permission_audit_logs"("operator_user_id");
CREATE INDEX IF NOT EXISTS idx_permission_audit_logs_target_user_id ON "permission_audit_logs"("target_user_id");
CREATE INDEX IF NOT EXISTS idx_permission_audit_logs_operation ON "permission_audit_logs"("operation");
CREATE INDEX IF NOT EXISTS idx_permission_audit_logs_created_at ON "permission_audit_logs"("created_at");

CREATE INDEX IF NOT EXISTS idx_security_audit_logs_tenant_id ON "security_audit_logs"("tenant_id");
CREATE INDEX IF NOT EXISTS idx_security_audit_logs_user_id ON "security_audit_logs"("user_id");
CREATE INDEX IF NOT EXISTS idx_security_audit_logs_event ON "security_audit_logs"("event");
CREATE INDEX IF NOT EXISTS idx_security_audit_logs_risk_level ON "security_audit_logs"("risk_level");
CREATE INDEX IF NOT EXISTS idx_security_audit_logs_ip_address ON "security_audit_logs"("ip_address");
CREATE INDEX IF NOT EXISTS idx_security_audit_logs_created_at ON "security_audit_logs"("created_at");

CREATE INDEX IF NOT EXISTS idx_performance_logs_tenant_id ON "performance_logs"("tenant_id");
CREATE INDEX IF NOT EXISTS idx_performance_logs_endpoint ON "performance_logs"("endpoint");
CREATE INDEX IF NOT EXISTS idx_performance_logs_total_duration_ms ON "performance_logs"("total_duration_ms");
CREATE INDEX IF NOT EXISTS idx_performance_logs_created_at ON "performance_logs"("created_at");

CREATE INDEX IF NOT EXISTS idx_system_health_logs_tenant_id ON "system_health_logs"("tenant_id");
CREATE INDEX IF NOT EXISTS idx_system_health_logs_db_status ON "system_health_logs"("db_status");
CREATE INDEX IF NOT EXISTS idx_system_health_logs_created_at ON "system_health_logs"("created_at");

CREATE INDEX IF NOT EXISTS idx_business_audit_logs_tenant_id ON "business_audit_logs"("tenant_id");
CREATE INDEX IF NOT EXISTS idx_business_audit_logs_module ON "business_audit_logs"("module");
CREATE INDEX IF NOT EXISTS idx_business_audit_logs_operation ON "business_audit_logs"("operation");
CREATE INDEX IF NOT EXISTS idx_business_audit_logs_resource_type ON "business_audit_logs"("resource_type");
CREATE INDEX IF NOT EXISTS idx_business_audit_logs_success ON "business_audit_logs"("success");
CREATE INDEX IF NOT EXISTS idx_business_audit_logs_created_at ON "business_audit_logs"("created_at");

-- 添加表注释
COMMENT ON TABLE "financial_audit_logs" IS '资金操作详细日志';
COMMENT ON TABLE "permission_audit_logs" IS '权限变更详细日志';
COMMENT ON TABLE "security_audit_logs" IS '安全事件详细日志';
COMMENT ON TABLE "performance_logs" IS '性能监控日志';
COMMENT ON TABLE "system_health_logs" IS '系统健康日志';
COMMENT ON TABLE "business_audit_logs" IS '业务操作详细日志';
