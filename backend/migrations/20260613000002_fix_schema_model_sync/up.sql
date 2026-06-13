-- P0-1 数据库迁移一致性修复
-- 修复模型与数据库 schema 不一致的问题
-- 创建时间: 2026-06-13

-- ============================================
-- 1. 确保 log_login 表存在（防御性创建）
-- ============================================
CREATE TABLE IF NOT EXISTS "log_login" (
    "id" BIGSERIAL PRIMARY KEY,
    "log_no" VARCHAR(50) NOT NULL DEFAULT '',
    "user_id" INTEGER,
    "username" VARCHAR(100) NOT NULL,
    "real_name" VARCHAR(100),
    "status" VARCHAR(20) NOT NULL,
    "fail_reason" TEXT,
    "login_type" VARCHAR(20),
    "ip_address" VARCHAR(50),
    "ip_location" VARCHAR(200),
    "user_agent" TEXT,
    "device_type" VARCHAR(50),
    "browser" VARCHAR(100),
    "os" VARCHAR(100),
    "login_time" TIMESTAMPTZ,
    "logout_time" TIMESTAMPTZ,
    "session_duration_seconds" BIGINT,
    "created_at" TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_log_login_user" ON "log_login" ("user_id");
CREATE INDEX IF NOT EXISTS "idx_log_login_time" ON "log_login" ("login_time");
CREATE INDEX IF NOT EXISTS "idx_log_login_username" ON "log_login" ("username");
CREATE INDEX IF NOT EXISTS "idx_log_login_status" ON "log_login" ("status");

COMMENT ON TABLE "log_login" IS '登录日志表 - 记录用户登录信息';

-- ============================================
-- 2. 确保 omni_audit_logs 表存在（防御性创建）
-- ============================================
CREATE TABLE IF NOT EXISTS "omni_audit_logs" (
    "id" BIGSERIAL PRIMARY KEY,
    "tenant_id" INTEGER,
    "trace_id" VARCHAR(100),
    "span_id" VARCHAR(50),
    "parent_span_id" VARCHAR(50),
    "user_id" INTEGER,
    "username" VARCHAR(100),
    "module" VARCHAR(100),
    "action" VARCHAR(100),
    "resource_type" VARCHAR(100),
    "resource_id" VARCHAR(100),
    "resource_name" VARCHAR(200),
    "description" TEXT,
    "ip_address" VARCHAR(50),
    "user_agent" TEXT,
    "request_method" VARCHAR(10),
    "request_path" VARCHAR(500),
    "request_body" TEXT,
    "response_status" INTEGER,
    "duration_ms" INTEGER,
    "old_value" JSONB,
    "new_value" JSONB,
    "created_at" TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_omni_audit_logs_trace" ON "omni_audit_logs" ("trace_id");
CREATE INDEX IF NOT EXISTS "idx_omni_audit_logs_user" ON "omni_audit_logs" ("user_id");
CREATE INDEX IF NOT EXISTS "idx_omni_audit_logs_created" ON "omni_audit_logs" ("created_at");
CREATE INDEX IF NOT EXISTS "idx_omni_audit_logs_tenant" ON "omni_audit_logs" ("tenant_id");
CREATE INDEX IF NOT EXISTS "idx_omni_audit_logs_action" ON "omni_audit_logs" ("action");
CREATE INDEX IF NOT EXISTS "idx_omni_audit_logs_resource_type" ON "omni_audit_logs" ("resource_type");

COMMENT ON TABLE "omni_audit_logs" IS '全局审计日志表 - 记录系统所有操作';

-- ============================================
-- 3. 确保 users 表包含 TOTP 字段
-- ============================================
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "totp_secret" VARCHAR(255);
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "is_totp_enabled" BOOLEAN NOT NULL DEFAULT FALSE;

COMMENT ON COLUMN "users"."totp_secret" IS 'TOTP 密钥';
COMMENT ON COLUMN "users"."is_totp_enabled" IS '是否启用 TOTP 二次验证';

-- ============================================
-- 4. products 表：重命名 product_no 为 code
-- ============================================
-- 检查并重命名列
DO $$
BEGIN
    -- 如果 product_no 存在且 code 不存在，则重命名
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'products' AND column_name = 'product_no')
       AND NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'products' AND column_name = 'code') THEN
        ALTER TABLE "products" RENAME COLUMN "product_no" TO "code";
    END IF;
    
    -- 如果 code 列不存在，则添加
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'products' AND column_name = 'code') THEN
        ALTER TABLE "products" ADD COLUMN "code" VARCHAR(50) NOT NULL DEFAULT '';
    END IF;
END $$;

-- 确保 code 列有唯一约束
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'products_code_key') THEN
        ALTER TABLE "products" ADD CONSTRAINT "products_code_key" UNIQUE ("code");
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS "idx_products_code" ON "products" ("code");
COMMENT ON COLUMN "products"."code" IS '产品编码（唯一）';

-- ============================================
-- 5. warehouses 表：重命名 code 为 warehouse_code
-- ============================================
DO $$
BEGIN
    -- 如果 code 存在且 warehouse_code 不存在，则重命名
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'warehouses' AND column_name = 'code')
       AND NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'warehouses' AND column_name = 'warehouse_code') THEN
        ALTER TABLE "warehouses" RENAME COLUMN "code" TO "warehouse_code";
    END IF;
    
    -- 如果 warehouse_code 列不存在，则添加
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'warehouses' AND column_name = 'warehouse_code') THEN
        ALTER TABLE "warehouses" ADD COLUMN "warehouse_code" VARCHAR(50) NOT NULL DEFAULT '';
    END IF;
END $$;

-- 确保 warehouse_code 列有唯一约束
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'warehouses_warehouse_code_key') THEN
        ALTER TABLE "warehouses" ADD CONSTRAINT "warehouses_warehouse_code_key" UNIQUE ("warehouse_code");
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS "idx_warehouses_warehouse_code" ON "warehouses" ("warehouse_code");
COMMENT ON COLUMN "warehouses"."warehouse_code" IS '仓库编码（唯一）';

-- ============================================
-- 6. sales_orders 表：重命名 delivery_date 为 required_date
-- ============================================
DO $$
BEGIN
    -- 如果 delivery_date 存在且 required_date 不存在，则重命名
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sales_orders' AND column_name = 'delivery_date')
       AND NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sales_orders' AND column_name = 'required_date') THEN
        ALTER TABLE "sales_orders" RENAME COLUMN "delivery_date" TO "required_date";
    END IF;
    
    -- 如果 required_date 列不存在，则添加
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sales_orders' AND column_name = 'required_date') THEN
        ALTER TABLE "sales_orders" ADD COLUMN "required_date" DATE;
    END IF;
END $$;

COMMENT ON COLUMN "sales_orders"."required_date" IS '要求交货日期';

-- ============================================
-- 7. 批量转换 TIMESTAMP 为 TIMESTAMPTZ
-- ============================================
-- 注意：PostgreSQL 中 ALTER COLUMN TYPE 会自动处理时区转换

-- products 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'products' AND column_name = 'created_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "products" ALTER COLUMN "created_at" TYPE TIMESTAMPTZ USING "created_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'products' AND column_name = 'updated_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "products" ALTER COLUMN "updated_at" TYPE TIMESTAMPTZ USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- warehouses 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'warehouses' AND column_name = 'created_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "warehouses" ALTER COLUMN "created_at" TYPE TIMESTAMPTZ USING "created_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'warehouses' AND column_name = 'updated_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "warehouses" ALTER COLUMN "updated_at" TYPE TIMESTAMPTZ USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- sales_orders 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sales_orders' AND column_name = 'created_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "sales_orders" ALTER COLUMN "created_at" TYPE TIMESTAMPTZ USING "created_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sales_orders' AND column_name = 'updated_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "sales_orders" ALTER COLUMN "updated_at" TYPE TIMESTAMPTZ USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- inventory_stocks 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_stocks' AND column_name = 'created_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "inventory_stocks" ALTER COLUMN "created_at" TYPE TIMESTAMPTZ USING "created_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_stocks' AND column_name = 'updated_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "inventory_stocks" ALTER COLUMN "updated_at" TYPE TIMESTAMPTZ USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- suppliers 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'suppliers' AND column_name = 'created_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "suppliers" ALTER COLUMN "created_at" TYPE TIMESTAMPTZ USING "created_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'suppliers' AND column_name = 'updated_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "suppliers" ALTER COLUMN "updated_at" TYPE TIMESTAMPTZ USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- customers 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'customers' AND column_name = 'created_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "customers" ALTER COLUMN "created_at" TYPE TIMESTAMPTZ USING "created_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'customers' AND column_name = 'updated_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "customers" ALTER COLUMN "updated_at" TYPE TIMESTAMPTZ USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- product_categories 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'product_categories' AND column_name = 'created_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "product_categories" ALTER COLUMN "created_at" TYPE TIMESTAMPTZ USING "created_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'product_categories' AND column_name = 'updated_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "product_categories" ALTER COLUMN "updated_at" TYPE TIMESTAMPTZ USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- sales_order_items 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sales_order_items' AND column_name = 'created_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "sales_order_items" ALTER COLUMN "created_at" TYPE TIMESTAMPTZ USING "created_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sales_order_items' AND column_name = 'updated_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "sales_order_items" ALTER COLUMN "updated_at" TYPE TIMESTAMPTZ USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- purchase_orders 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'purchase_orders' AND column_name = 'created_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "purchase_orders" ALTER COLUMN "created_at" TYPE TIMESTAMPTZ USING "created_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'purchase_orders' AND column_name = 'updated_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "purchase_orders" ALTER COLUMN "updated_at" TYPE TIMESTAMPTZ USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- purchase_order_items 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'purchase_order_items' AND column_name = 'created_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "purchase_order_items" ALTER COLUMN "created_at" TYPE TIMESTAMPTZ USING "created_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'purchase_order_items' AND column_name = 'updated_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "purchase_order_items" ALTER COLUMN "updated_at" TYPE TIMESTAMPTZ USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- finance_payments 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'finance_payments' AND column_name = 'created_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "finance_payments" ALTER COLUMN "created_at" TYPE TIMESTAMPTZ USING "created_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'finance_payments' AND column_name = 'updated_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "finance_payments" ALTER COLUMN "updated_at" TYPE TIMESTAMPTZ USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- inventory_transfers 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_transfers' AND column_name = 'created_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "inventory_transfers" ALTER COLUMN "created_at" TYPE TIMESTAMPTZ USING "created_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_transfers' AND column_name = 'updated_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "inventory_transfers" ALTER COLUMN "updated_at" TYPE TIMESTAMPTZ USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- inventory_transfer_items 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_transfer_items' AND column_name = 'created_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "inventory_transfer_items" ALTER COLUMN "created_at" TYPE TIMESTAMPTZ USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- inventory_counts 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_counts' AND column_name = 'created_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "inventory_counts" ALTER COLUMN "created_at" TYPE TIMESTAMPTZ USING "created_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_counts' AND column_name = 'updated_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "inventory_counts" ALTER COLUMN "updated_at" TYPE TIMESTAMPTZ USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- inventory_count_items 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_count_items' AND column_name = 'created_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "inventory_count_items" ALTER COLUMN "created_at" TYPE TIMESTAMPTZ USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- operation_logs 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'operation_logs' AND column_name = 'created_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "operation_logs" ALTER COLUMN "created_at" TYPE TIMESTAMPTZ USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- log_system 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'log_system' AND column_name = 'created_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "log_system" ALTER COLUMN "created_at" TYPE TIMESTAMPTZ USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- log_api_accesses 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'log_api_accesses' AND column_name = 'created_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "log_api_accesses" ALTER COLUMN "created_at" TYPE TIMESTAMPTZ USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- audit_alert_rules 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'audit_alert_rules' AND column_name = 'created_at' AND data_type = 'timestamp without time zone') THEN
        ALTER TABLE "audit_alert_rules" ALTER COLUMN "created_at" TYPE TIMESTAMPTZ USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- ============================================
-- 完成
-- ============================================
