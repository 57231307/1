-- 批次1：基础数据扩展 + 系统管理
-- 创建时间: 2026-05-27
-- 描述: 创建基础数据扩展表和系统管理扩展表

-- ============================================
-- 1. 基础主数据扩展 - 币种表
-- ============================================
CREATE TABLE IF NOT EXISTS "currencies" (
    "id" SERIAL PRIMARY KEY,
    "code" VARCHAR(10) NOT NULL UNIQUE,
    "name" VARCHAR(50) NOT NULL,
    "symbol" VARCHAR(10),
    "decimal_places" INTEGER DEFAULT 2,
    "is_base" BOOLEAN DEFAULT false,
    "status" VARCHAR(20) DEFAULT 'active',
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_currencies_code" ON "currencies" ("code");
COMMENT ON TABLE "currencies" IS '币种表 - 存储系统支持的货币信息';

-- ============================================
-- 2. 基础主数据扩展 - 汇率表
-- ============================================
CREATE TABLE IF NOT EXISTS "exchange_rates" (
    "id" SERIAL PRIMARY KEY,
    "from_currency" VARCHAR(10) NOT NULL,
    "to_currency" VARCHAR(10) NOT NULL,
    "rate" DECIMAL(20, 8) NOT NULL,
    "effective_date" DATE NOT NULL,
    "status" VARCHAR(20) DEFAULT 'active',
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_exchange_rates_currencies" ON "exchange_rates" ("from_currency", "to_currency");
CREATE INDEX IF NOT EXISTS "idx_exchange_rates_date" ON "exchange_rates" ("effective_date");
COMMENT ON TABLE "exchange_rates" IS '汇率表 - 存储货币汇率信息';

-- ============================================
-- 3. 基础主数据扩展 - 供应商分类表
-- ============================================
CREATE TABLE IF NOT EXISTS "supplier_categories" (
    "id" SERIAL PRIMARY KEY,
    "category_code" VARCHAR(50) NOT NULL UNIQUE,
    "category_name" VARCHAR(100) NOT NULL,
    "parent_id" INTEGER,
    "level" INTEGER NOT NULL DEFAULT 1,
    "sort_order" INTEGER NOT NULL DEFAULT 0,
    "is_enabled" BOOLEAN NOT NULL DEFAULT true,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_supplier_categories_parent" FOREIGN KEY ("parent_id") REFERENCES "supplier_categories" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_supplier_categories_parent" ON "supplier_categories" ("parent_id");
COMMENT ON TABLE "supplier_categories" IS '供应商分类表 - 支持多级分类';

-- ============================================
-- 4. 基础主数据扩展 - 供应商等级表
-- ============================================
CREATE TABLE IF NOT EXISTS "supplier_grades" (
    "id" SERIAL PRIMARY KEY,
    "grade_code" VARCHAR(10) NOT NULL UNIQUE,
    "grade_name" VARCHAR(50) NOT NULL,
    "min_score" DECIMAL(5, 2) NOT NULL,
    "max_score" DECIMAL(5, 2) NOT NULL,
    "color_code" VARCHAR(20),
    "permission_desc" TEXT,
    "is_enabled" BOOLEAN NOT NULL DEFAULT true,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "supplier_grades" IS '供应商等级表 - 存储供应商评级标准';

-- ============================================
-- 5. 基础主数据扩展 - 评估指标表
-- ============================================
CREATE TABLE IF NOT EXISTS "supplier_evaluation_indicators" (
    "id" SERIAL PRIMARY KEY,
    "indicator_name" VARCHAR(100) NOT NULL,
    "indicator_code" VARCHAR(50) NOT NULL UNIQUE,
    "category" VARCHAR(50) NOT NULL,
    "weight" DECIMAL(5, 2) NOT NULL DEFAULT 1.00,
    "max_score" INTEGER NOT NULL DEFAULT 100,
    "evaluation_method" TEXT,
    "status" VARCHAR(20) DEFAULT 'active',
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "supplier_evaluation_indicators" IS '评估指标表 - 存储供应商评估指标';

-- ============================================
-- 6. 基础主数据扩展 - 质量标准表
-- ============================================
CREATE TABLE IF NOT EXISTS "quality_standards" (
    "id" SERIAL PRIMARY KEY,
    "standard_name" VARCHAR(200) NOT NULL,
    "standard_code" VARCHAR(50) NOT NULL UNIQUE,
    "standard_type" VARCHAR(50) NOT NULL,
    "product_id" INTEGER,
    "product_category_id" INTEGER,
    "version" VARCHAR(20) NOT NULL DEFAULT '1.0',
    "previous_version_id" INTEGER,
    "content" TEXT NOT NULL,
    "technical_requirements" TEXT,
    "testing_methods" TEXT,
    "acceptance_criteria" TEXT,
    "effective_date" DATE NOT NULL,
    "expiry_date" DATE,
    "status" VARCHAR(20) DEFAULT 'draft',
    "approved_by" INTEGER,
    "approved_at" TIMESTAMP,
    "created_by" INTEGER,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_quality_standards_type" ON "quality_standards" ("standard_type");
CREATE INDEX IF NOT EXISTS "idx_quality_standards_product" ON "quality_standards" ("product_id");
COMMENT ON TABLE "quality_standards" IS '质量标准表 - 存储产品质量标准';

-- ============================================
-- 7. 基础主数据扩展 - 质量检验标准表
-- ============================================
CREATE TABLE IF NOT EXISTS "quality_inspection_standards" (
    "id" SERIAL PRIMARY KEY,
    "standard_name" VARCHAR(200) NOT NULL,
    "standard_code" VARCHAR(50) NOT NULL UNIQUE,
    "product_id" INTEGER,
    "product_category_id" INTEGER,
    "inspection_type" VARCHAR(50) NOT NULL,
    "inspection_items" JSONB,
    "sampling_method" VARCHAR(100),
    "sampling_rate" DECIMAL(5, 2),
    "acceptance_criteria" TEXT,
    "status" VARCHAR(20) DEFAULT 'active',
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "quality_inspection_standards" IS '质量检验标准表 - 存储产品检验标准';

-- ============================================
-- 8. 基础主数据扩展 - 质量检验记录表
-- ============================================
CREATE TABLE IF NOT EXISTS "quality_inspection_records" (
    "id" SERIAL PRIMARY KEY,
    "inspection_no" VARCHAR(50) NOT NULL UNIQUE,
    "product_id" INTEGER NOT NULL,
    "batch_no" VARCHAR(50),
    "supplier_id" INTEGER,
    "inspection_type" VARCHAR(50) NOT NULL,
    "standard_id" INTEGER,
    "inspection_date" DATE NOT NULL,
    "inspector_id" INTEGER,
    "result" VARCHAR(20) NOT NULL,
    "qualified_qty" DECIMAL(12, 2),
    "unqualified_qty" DECIMAL(12, 2),
    "defect_details" JSONB,
    "remarks" TEXT,
    "status" VARCHAR(20) DEFAULT 'draft',
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_quality_inspection_records_product" ON "quality_inspection_records" ("product_id");
CREATE INDEX IF NOT EXISTS "idx_quality_inspection_records_date" ON "quality_inspection_records" ("inspection_date");
COMMENT ON TABLE "quality_inspection_records" IS '质量检验记录表 - 存储产品检验记录';

-- ============================================
-- 9. 系统管理扩展 - 角色权限表
-- ============================================
CREATE TABLE IF NOT EXISTS "role_permissions" (
    "id" SERIAL PRIMARY KEY,
    "role_id" INTEGER NOT NULL,
    "resource_type" VARCHAR(100) NOT NULL,
    "resource_id" INTEGER,
    "action" VARCHAR(50) NOT NULL,
    "allowed" BOOLEAN NOT NULL DEFAULT true,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_role_permissions_role" FOREIGN KEY ("role_id") REFERENCES "roles" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_role_permissions_role" ON "role_permissions" ("role_id");
CREATE INDEX IF NOT EXISTS "idx_role_permissions_resource" ON "role_permissions" ("resource_type", "resource_id");
COMMENT ON TABLE "role_permissions" IS '角色权限表 - 存储角色的资源权限';

-- ============================================
-- 10. 系统管理扩展 - 数据权限表
-- ============================================
CREATE TABLE IF NOT EXISTS "data_permissions" (
    "id" SERIAL PRIMARY KEY,
    "role_id" INTEGER NOT NULL,
    "resource_type" VARCHAR(100) NOT NULL,
    "scope_type" VARCHAR(50) NOT NULL,
    "custom_condition" JSONB,
    "allowed_fields" JSONB,
    "hidden_fields" JSONB,
    "is_enabled" BOOLEAN NOT NULL DEFAULT true,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_data_permissions_role" FOREIGN KEY ("role_id") REFERENCES "roles" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_data_permissions_role" ON "data_permissions" ("role_id");
COMMENT ON TABLE "data_permissions" IS '数据权限表 - 存储角色的数据访问范围';

-- ============================================
-- 11. 系统管理扩展 - API密钥表
-- ============================================
CREATE TABLE IF NOT EXISTS "api_keys" (
    "id" SERIAL PRIMARY KEY,
    "tenant_id" INTEGER,
    "name" VARCHAR(100) NOT NULL,
    "key_hash" VARCHAR(255) NOT NULL,
    "key_prefix" VARCHAR(20) NOT NULL,
    "permissions" TEXT,
    "rate_limit_per_minute" INTEGER NOT NULL DEFAULT 60,
    "last_used_at" TIMESTAMP,
    "expires_at" TIMESTAMP,
    "is_active" BOOLEAN NOT NULL DEFAULT true,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_api_keys_tenant" ON "api_keys" ("tenant_id");
CREATE INDEX IF NOT EXISTS "idx_api_keys_prefix" ON "api_keys" ("key_prefix");
COMMENT ON TABLE "api_keys" IS 'API密钥表 - 存储API访问密钥';

-- ============================================
-- 12. 系统管理扩展 - Webhook表
-- ============================================
CREATE TABLE IF NOT EXISTS "webhooks" (
    "id" SERIAL PRIMARY KEY,
    "tenant_id" INTEGER,
    "name" VARCHAR(100) NOT NULL,
    "url" VARCHAR(500) NOT NULL,
    "events" TEXT NOT NULL,
    "secret" VARCHAR(255),
    "is_active" BOOLEAN NOT NULL DEFAULT true,
    "last_triggered_at" TIMESTAMP,
    "last_status" VARCHAR(20),
    "retry_count" INTEGER NOT NULL DEFAULT 3,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_webhooks_tenant" ON "webhooks" ("tenant_id");
COMMENT ON TABLE "webhooks" IS 'Webhook表 - 存储Webhook配置';

-- ============================================
-- 13. 系统管理扩展 - 系统版本表
-- ============================================
CREATE TABLE IF NOT EXISTS "system_version" (
    "id" SERIAL PRIMARY KEY,
    "version" VARCHAR(50) NOT NULL,
    "release_date" TIMESTAMP NOT NULL,
    "changelog" TEXT,
    "is_current" BOOLEAN NOT NULL DEFAULT false,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "system_version" IS '系统版本表 - 存储系统版本信息';

-- ============================================
-- 14. 系统管理扩展 - API访问日志表
-- ============================================
CREATE TABLE IF NOT EXISTS "log_api_accesses" (
    "id" SERIAL PRIMARY KEY,
    "user_id" INTEGER,
    "username" VARCHAR(100),
    "method" VARCHAR(10) NOT NULL,
    "path" VARCHAR(500) NOT NULL,
    "query_params" TEXT,
    "request_body" TEXT,
    "status_code" INTEGER,
    "response_size" BIGINT,
    "execution_time" BIGINT NOT NULL,
    "ip_address" VARCHAR(50),
    "user_agent" TEXT,
    "error_message" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_log_api_accesses_user" FOREIGN KEY ("user_id") REFERENCES "users" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_log_api_accesses_user" ON "log_api_accesses" ("user_id");
CREATE INDEX IF NOT EXISTS "idx_log_api_accesses_path" ON "log_api_accesses" ("path");
CREATE INDEX IF NOT EXISTS "idx_log_api_accesses_created" ON "log_api_accesses" ("created_at");
COMMENT ON TABLE "log_api_accesses" IS 'API访问日志表 - 记录API调用情况';

-- ============================================
-- 15. 系统管理扩展 - 登录日志表
-- ============================================
CREATE TABLE IF NOT EXISTS "log_login" (
    "id" SERIAL PRIMARY KEY,
    "user_id" INTEGER,
    "username" VARCHAR(100) NOT NULL,
    "login_type" VARCHAR(20) NOT NULL,
    "ip_address" VARCHAR(50) NOT NULL,
    "user_agent" TEXT,
    "status" VARCHAR(20) NOT NULL,
    "fail_reason" TEXT,
    "login_time" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_log_login_user" ON "log_login" ("user_id");
CREATE INDEX IF NOT EXISTS "idx_log_login_time" ON "log_login" ("login_time");
COMMENT ON TABLE "log_login" IS '登录日志表 - 记录用户登录信息';

-- ============================================
-- 16. 系统管理扩展 - 系统日志表
-- ============================================
CREATE TABLE IF NOT EXISTS "log_system" (
    "id" BIGSERIAL PRIMARY KEY,
    "level" VARCHAR(20) NOT NULL,
    "module" VARCHAR(100),
    "message" TEXT NOT NULL,
    "details" JSONB,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_log_system_level" ON "log_system" ("level");
CREATE INDEX IF NOT EXISTS "idx_log_system_created" ON "log_system" ("created_at");
COMMENT ON TABLE "log_system" IS '系统日志表 - 记录系统运行日志';

-- ============================================
-- 17. 系统管理扩展 - 全局审计日志表
-- ============================================
CREATE TABLE IF NOT EXISTS "omni_audit_logs" (
    "id" BIGSERIAL PRIMARY KEY,
    "trace_id" VARCHAR(100),
    "user_id" INTEGER,
    "module" VARCHAR(100),
    "action" VARCHAR(100),
    "response_status" INTEGER,
    "duration_ms" INTEGER,
    "created_at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_omni_audit_logs_trace" ON "omni_audit_logs" ("trace_id");
CREATE INDEX IF NOT EXISTS "idx_omni_audit_logs_user" ON "omni_audit_logs" ("user_id");
CREATE INDEX IF NOT EXISTS "idx_omni_audit_logs_created" ON "omni_audit_logs" ("created_at");
COMMENT ON TABLE "omni_audit_logs" IS '全局审计日志表 - 记录系统所有操作';

-- ============================================
-- 18. 系统管理扩展 - 操作日志表
-- ============================================
CREATE TABLE IF NOT EXISTS "operation_logs" (
    "id" SERIAL PRIMARY KEY,
    "user_id" INTEGER,
    "username" VARCHAR(100),
    "module" VARCHAR(100) NOT NULL,
    "action" VARCHAR(100) NOT NULL,
    "description" TEXT,
    "request_method" VARCHAR(10),
    "request_uri" VARCHAR(500),
    "request_ip" VARCHAR(50),
    "user_agent" TEXT,
    "status" VARCHAR(20) NOT NULL DEFAULT 'success',
    "error_message" TEXT,
    "duration_ms" BIGINT,
    "extra_data" JSONB,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_operation_logs_user" ON "operation_logs" ("user_id");
CREATE INDEX IF NOT EXISTS "idx_operation_logs_module" ON "operation_logs" ("module");
CREATE INDEX IF NOT EXISTS "idx_operation_logs_created" ON "operation_logs" ("created_at");
COMMENT ON TABLE "operation_logs" IS '操作日志表 - 记录用户操作';

-- ============================================
-- 19. 系统管理扩展 - 审计告警规则表
-- ============================================
CREATE TABLE IF NOT EXISTS "audit_alert_rules" (
    "id" SERIAL PRIMARY KEY,
    "rule_name" VARCHAR(200) NOT NULL,
    "event_type" VARCHAR(100) NOT NULL,
    "condition_expr" JSONB,
    "alert_level" VARCHAR(20) NOT NULL,
    "is_active" BOOLEAN NOT NULL DEFAULT true,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "audit_alert_rules" IS '审计告警规则表 - 存储告警规则配置';
