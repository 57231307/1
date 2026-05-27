-- 批次10：SaaS多租户 + 通知 + 报表 + 邮件 + OA
-- 创建时间: 2026-05-27
-- 描述: 创建SaaS多租户、通知、报表、邮件和OA相关表

-- ============================================
-- 多租户（7张表）
-- ============================================

-- 1. 租户计划表
CREATE TABLE IF NOT EXISTS "tenant_plans" (
    "id" SERIAL PRIMARY KEY,
    "code" VARCHAR(50) NOT NULL UNIQUE,
    "name" VARCHAR(100) NOT NULL,
    "description" TEXT,
    "max_users" INTEGER NOT NULL,
    "max_storage_mb" INTEGER NOT NULL,
    "max_api_calls_per_day" INTEGER NOT NULL,
    "price_monthly" DECIMAL(10, 2) NOT NULL,
    "price_yearly" DECIMAL(10, 2) NOT NULL,
    "features" TEXT,
    "is_active" BOOLEAN NOT NULL DEFAULT true,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "tenant_plans" IS '租户计划表';

-- 2. 租户表
CREATE TABLE IF NOT EXISTS "tenants" (
    "id" SERIAL PRIMARY KEY,
    "code" VARCHAR(50) NOT NULL UNIQUE,
    "name" VARCHAR(100) NOT NULL,
    "description" TEXT,
    "status" VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    "plan_id" INTEGER,
    "admin_user_id" INTEGER,
    "db_schema" VARCHAR(50),
    "custom_domain" VARCHAR(200),
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "expired_at" TIMESTAMP,
    CONSTRAINT "fk_tenants_plan" FOREIGN KEY ("plan_id") REFERENCES "tenant_plans" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_tenants_plan" ON "tenants" ("plan_id");
COMMENT ON TABLE "tenants" IS '租户表';

-- 3. 租户用户表
CREATE TABLE IF NOT EXISTS "tenant_users" (
    "id" SERIAL PRIMARY KEY,
    "tenant_id" INTEGER NOT NULL,
    "user_id" INTEGER NOT NULL,
    "role_in_tenant" VARCHAR(50) NOT NULL,
    "is_primary" BOOLEAN NOT NULL DEFAULT false,
    "joined_at" TIMESTAMP NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_tenant_users_tenant" FOREIGN KEY ("tenant_id") REFERENCES "tenants" ("id"),
    CONSTRAINT "fk_tenant_users_user" FOREIGN KEY ("user_id") REFERENCES "users" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_tenant_users_tenant" ON "tenant_users" ("tenant_id");
CREATE INDEX IF NOT EXISTS "idx_tenant_users_user" ON "tenant_users" ("user_id");
COMMENT ON TABLE "tenant_users" IS '租户用户表';

-- 4. 租户配置表
CREATE TABLE IF NOT EXISTS "tenant_configs" (
    "id" SERIAL PRIMARY KEY,
    "tenant_id" INTEGER NOT NULL,
    "config_key" VARCHAR(100) NOT NULL,
    "config_value" TEXT NOT NULL,
    "config_type" VARCHAR(50) NOT NULL,
    "description" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_tenant_configs_tenant" FOREIGN KEY ("tenant_id") REFERENCES "tenants" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_tenant_configs_tenant" ON "tenant_configs" ("tenant_id");
CREATE UNIQUE INDEX IF NOT EXISTS "idx_tenant_configs_key" ON "tenant_configs" ("tenant_id", "config_key");
COMMENT ON TABLE "tenant_configs" IS '租户配置表';

-- 5. 租户订阅表
CREATE TABLE IF NOT EXISTS "tenant_subscriptions" (
    "id" SERIAL PRIMARY KEY,
    "tenant_id" INTEGER NOT NULL,
    "plan_id" INTEGER NOT NULL,
    "status" VARCHAR(20) NOT NULL,
    "billing_cycle" VARCHAR(20) NOT NULL,
    "start_date" TIMESTAMP NOT NULL,
    "end_date" TIMESTAMP,
    "auto_renew" BOOLEAN NOT NULL DEFAULT true,
    "current_price" DECIMAL(10, 2) NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_tenant_subscriptions_tenant" FOREIGN KEY ("tenant_id") REFERENCES "tenants" ("id"),
    CONSTRAINT "fk_tenant_subscriptions_plan" FOREIGN KEY ("plan_id") REFERENCES "tenant_plans" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_tenant_subscriptions_tenant" ON "tenant_subscriptions" ("tenant_id");
COMMENT ON TABLE "tenant_subscriptions" IS '租户订阅表';

-- 6. 租户用量表
CREATE TABLE IF NOT EXISTS "tenant_usage" (
    "id" SERIAL PRIMARY KEY,
    "tenant_id" INTEGER NOT NULL,
    "stat_date" DATE NOT NULL,
    "api_calls" BIGINT NOT NULL DEFAULT 0,
    "storage_used_mb" BIGINT NOT NULL DEFAULT 0,
    "user_count" INTEGER NOT NULL DEFAULT 0,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_tenant_usage_tenant" FOREIGN KEY ("tenant_id") REFERENCES "tenants" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_tenant_usage_tenant" ON "tenant_usage" ("tenant_id");
CREATE INDEX IF NOT EXISTS "idx_tenant_usage_date" ON "tenant_usage" ("stat_date");
COMMENT ON TABLE "tenant_usage" IS '租户用量表';

-- 7. 租户发票表
CREATE TABLE IF NOT EXISTS "tenant_invoices" (
    "id" SERIAL PRIMARY KEY,
    "tenant_id" INTEGER NOT NULL,
    "subscription_id" INTEGER NOT NULL,
    "invoice_number" VARCHAR(50) NOT NULL,
    "billing_period_start" DATE NOT NULL,
    "billing_period_end" DATE NOT NULL,
    "amount" DECIMAL(10, 2) NOT NULL,
    "status" VARCHAR(20) NOT NULL,
    "paid_at" TIMESTAMP,
    "due_date" DATE NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_tenant_invoices_tenant" FOREIGN KEY ("tenant_id") REFERENCES "tenants" ("id"),
    CONSTRAINT "fk_tenant_invoices_subscription" FOREIGN KEY ("subscription_id") REFERENCES "tenant_subscriptions" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_tenant_invoices_tenant" ON "tenant_invoices" ("tenant_id");
COMMENT ON TABLE "tenant_invoices" IS '租户发票表';

-- ============================================
-- 消息通知（3张表）
-- ============================================

-- 8. 通知表
CREATE TABLE IF NOT EXISTS "notifications" (
    "id" SERIAL PRIMARY KEY,
    "user_id" INTEGER NOT NULL,
    "notification_type" VARCHAR(20) NOT NULL,
    "title" VARCHAR(200) NOT NULL,
    "content" TEXT NOT NULL,
    "priority" VARCHAR(10) NOT NULL DEFAULT 'NORMAL',
    "status" VARCHAR(20) NOT NULL DEFAULT 'UNREAD',
    "business_type" VARCHAR(50),
    "business_id" INTEGER,
    "action_url" VARCHAR(500),
    "sender_id" INTEGER,
    "sender_name" VARCHAR(100),
    "read_at" TIMESTAMP,
    "processed_at" TIMESTAMP,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_notifications_user" FOREIGN KEY ("user_id") REFERENCES "users" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_notifications_user" ON "notifications" ("user_id");
CREATE INDEX IF NOT EXISTS "idx_notifications_status" ON "notifications" ("status");
COMMENT ON TABLE "notifications" IS '通知表';

-- 9. 通知设置表
CREATE TABLE IF NOT EXISTS "notification_settings" (
    "id" SERIAL PRIMARY KEY,
    "user_id" INTEGER NOT NULL,
    "business_type" VARCHAR(50) NOT NULL,
    "enable_internal" BOOLEAN NOT NULL DEFAULT true,
    "enable_email" BOOLEAN NOT NULL DEFAULT false,
    "enable_sms" BOOLEAN NOT NULL DEFAULT false,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_notification_settings_user" FOREIGN KEY ("user_id") REFERENCES "users" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_notification_settings_user" ON "notification_settings" ("user_id");
COMMENT ON TABLE "notification_settings" IS '通知设置表';

-- 10. 用户通知设置表
CREATE TABLE IF NOT EXISTS "user_notification_setting" (
    "id" SERIAL PRIMARY KEY,
    "user_id" INTEGER NOT NULL,
    "email_enabled" BOOLEAN NOT NULL DEFAULT true,
    "internal_enabled" BOOLEAN NOT NULL DEFAULT true,
    "order_notification_type" VARCHAR(20) NOT NULL DEFAULT 'both',
    "approval_notification_type" VARCHAR(20) NOT NULL DEFAULT 'both',
    "inventory_notification_type" VARCHAR(20) NOT NULL DEFAULT 'both',
    "purchase_notification_type" VARCHAR(20) NOT NULL DEFAULT 'both',
    "finance_notification_type" VARCHAR(20) NOT NULL DEFAULT 'both',
    "system_notification_type" VARCHAR(20) NOT NULL DEFAULT 'both',
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "user_notification_setting" IS '用户通知设置表';

-- ============================================
-- 报表（3张表）
-- ============================================

-- 11. 报表定义表
CREATE TABLE IF NOT EXISTS "report_definition" (
    "id" SERIAL PRIMARY KEY,
    "report_code" VARCHAR(50) NOT NULL,
    "name" VARCHAR(100) NOT NULL,
    "report_type" VARCHAR(50) NOT NULL,
    "data_source" VARCHAR(100) NOT NULL,
    "sql_query" TEXT,
    "config" JSONB,
    "description" TEXT,
    "status" VARCHAR(20) NOT NULL DEFAULT 'DRAFT',
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "report_definition" IS '报表定义表';

-- 12. 报表模板表
CREATE TABLE IF NOT EXISTS "report_templates" (
    "id" SERIAL PRIMARY KEY,
    "tenant_id" INTEGER NOT NULL,
    "name" VARCHAR(100) NOT NULL,
    "code" VARCHAR(50) NOT NULL,
    "report_type" VARCHAR(50) NOT NULL,
    "columns" JSONB NOT NULL,
    "filters" JSONB,
    "sort_by" VARCHAR(100),
    "sort_order" VARCHAR(10),
    "data_source_sql" TEXT,
    "description" TEXT,
    "is_public" BOOLEAN NOT NULL DEFAULT false,
    "status" VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "report_templates" IS '报表模板表';

-- 13. 报表订阅表
CREATE TABLE IF NOT EXISTS "report_subscriptions" (
    "id" SERIAL PRIMARY KEY,
    "tenant_id" INTEGER NOT NULL,
    "name" VARCHAR(100) NOT NULL,
    "template_id" INTEGER NOT NULL,
    "frequency" VARCHAR(20) NOT NULL,
    "recipients" JSONB NOT NULL,
    "export_format" VARCHAR(20) NOT NULL,
    "is_enabled" BOOLEAN NOT NULL DEFAULT true,
    "status" VARCHAR(20) NOT NULL,
    "next_run_at" TIMESTAMP,
    "last_run_at" TIMESTAMP,
    "last_run_status" VARCHAR(20),
    "last_run_error" TEXT,
    "run_count" INTEGER NOT NULL DEFAULT 0,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "report_subscriptions" IS '报表订阅表';

-- ============================================
-- 邮件（2张表）
-- ============================================

-- 14. 邮件模板表
CREATE TABLE IF NOT EXISTS "email_templates" (
    "id" SERIAL PRIMARY KEY,
    "tenant_id" INTEGER NOT NULL,
    "name" VARCHAR(100) NOT NULL,
    "code" VARCHAR(50) NOT NULL,
    "subject_template" VARCHAR(200) NOT NULL,
    "body_template" TEXT NOT NULL,
    "template_type" VARCHAR(50) NOT NULL,
    "variables" JSONB,
    "description" TEXT,
    "is_active" BOOLEAN NOT NULL DEFAULT true,
    "status" VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "email_templates" IS '邮件模板表';

-- 15. 邮件日志表
CREATE TABLE IF NOT EXISTS "email_logs" (
    "id" SERIAL PRIMARY KEY,
    "tenant_id" INTEGER NOT NULL,
    "user_id" INTEGER,
    "recipients" TEXT NOT NULL,
    "cc" TEXT,
    "bcc" TEXT,
    "subject" VARCHAR(200) NOT NULL,
    "body" TEXT,
    "template_id" INTEGER,
    "status" VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    "error_message" TEXT,
    "external_message_id" VARCHAR(100),
    "sent_at" TIMESTAMP,
    "retry_count" INTEGER NOT NULL DEFAULT 0,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_email_logs_tenant" ON "email_logs" ("tenant_id");
CREATE INDEX IF NOT EXISTS "idx_email_logs_status" ON "email_logs" ("status");
COMMENT ON TABLE "email_logs" IS '邮件日志表';

-- ============================================
-- OA（1张表）
-- ============================================

-- 16. OA公告表
CREATE TABLE IF NOT EXISTS "oa_announcement" (
    "id" SERIAL PRIMARY KEY,
    "title" VARCHAR(200) NOT NULL,
    "content" TEXT NOT NULL,
    "announcement_type" VARCHAR(20) NOT NULL,
    "publish_date" DATE NOT NULL,
    "effective_date" DATE NOT NULL,
    "expiry_date" DATE,
    "publisher_id" INTEGER NOT NULL,
    "status" VARCHAR(20) NOT NULL DEFAULT 'DRAFT',
    "is_top" BOOLEAN NOT NULL DEFAULT false,
    "attachments" JSONB,
    "remarks" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "oa_announcement" IS 'OA公告表';
