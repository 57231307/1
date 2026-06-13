-- P0-1 数据库迁移一致性修复 - 回滚脚本
-- 回滚模型与数据库 schema 一致性修复
-- 创建时间: 2026-06-13

-- ============================================
-- 7. 批量转换 TIMESTAMPTZ 回 TIMESTAMP（逆序）
-- ============================================

-- audit_alert_rules 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'audit_alert_rules' AND column_name = 'created_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "audit_alert_rules" ALTER COLUMN "created_at" TYPE TIMESTAMP USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- log_api_accesses 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'log_api_accesses' AND column_name = 'created_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "log_api_accesses" ALTER COLUMN "created_at" TYPE TIMESTAMP USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- log_system 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'log_system' AND column_name = 'created_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "log_system" ALTER COLUMN "created_at" TYPE TIMESTAMP USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- operation_logs 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'operation_logs' AND column_name = 'created_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "operation_logs" ALTER COLUMN "created_at" TYPE TIMESTAMP USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- inventory_count_items 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_count_items' AND column_name = 'created_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "inventory_count_items" ALTER COLUMN "created_at" TYPE TIMESTAMP USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- inventory_counts 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_counts' AND column_name = 'updated_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "inventory_counts" ALTER COLUMN "updated_at" TYPE TIMESTAMP USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_counts' AND column_name = 'created_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "inventory_counts" ALTER COLUMN "created_at" TYPE TIMESTAMP USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- inventory_transfer_items 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_transfer_items' AND column_name = 'created_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "inventory_transfer_items" ALTER COLUMN "created_at" TYPE TIMESTAMP USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- inventory_transfers 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_transfers' AND column_name = 'updated_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "inventory_transfers" ALTER COLUMN "updated_at" TYPE TIMESTAMP USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_transfers' AND column_name = 'created_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "inventory_transfers" ALTER COLUMN "created_at" TYPE TIMESTAMP USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- finance_payments 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'finance_payments' AND column_name = 'updated_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "finance_payments" ALTER COLUMN "updated_at" TYPE TIMESTAMP USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'finance_payments' AND column_name = 'created_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "finance_payments" ALTER COLUMN "created_at" TYPE TIMESTAMP USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- purchase_order_items 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'purchase_order_items' AND column_name = 'updated_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "purchase_order_items" ALTER COLUMN "updated_at" TYPE TIMESTAMP USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'purchase_order_items' AND column_name = 'created_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "purchase_order_items" ALTER COLUMN "created_at" TYPE TIMESTAMP USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- purchase_orders 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'purchase_orders' AND column_name = 'updated_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "purchase_orders" ALTER COLUMN "updated_at" TYPE TIMESTAMP USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'purchase_orders' AND column_name = 'created_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "purchase_orders" ALTER COLUMN "created_at" TYPE TIMESTAMP USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- sales_order_items 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sales_order_items' AND column_name = 'updated_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "sales_order_items" ALTER COLUMN "updated_at" TYPE TIMESTAMP USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sales_order_items' AND column_name = 'created_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "sales_order_items" ALTER COLUMN "created_at" TYPE TIMESTAMP USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- product_categories 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'product_categories' AND column_name = 'updated_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "product_categories" ALTER COLUMN "updated_at" TYPE TIMESTAMP USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'product_categories' AND column_name = 'created_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "product_categories" ALTER COLUMN "created_at" TYPE TIMESTAMP USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- customers 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'customers' AND column_name = 'updated_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "customers" ALTER COLUMN "updated_at" TYPE TIMESTAMP USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'customers' AND column_name = 'created_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "customers" ALTER COLUMN "created_at" TYPE TIMESTAMP USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- suppliers 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'suppliers' AND column_name = 'updated_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "suppliers" ALTER COLUMN "updated_at" TYPE TIMESTAMP USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'suppliers' AND column_name = 'created_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "suppliers" ALTER COLUMN "created_at" TYPE TIMESTAMP USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- inventory_stocks 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_stocks' AND column_name = 'updated_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "inventory_stocks" ALTER COLUMN "updated_at" TYPE TIMESTAMP USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_stocks' AND column_name = 'created_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "inventory_stocks" ALTER COLUMN "created_at" TYPE TIMESTAMP USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- sales_orders 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sales_orders' AND column_name = 'updated_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "sales_orders" ALTER COLUMN "updated_at" TYPE TIMESTAMP USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sales_orders' AND column_name = 'created_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "sales_orders" ALTER COLUMN "created_at" TYPE TIMESTAMP USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- warehouses 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'warehouses' AND column_name = 'updated_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "warehouses" ALTER COLUMN "updated_at" TYPE TIMESTAMP USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'warehouses' AND column_name = 'created_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "warehouses" ALTER COLUMN "created_at" TYPE TIMESTAMP USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- products 表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'products' AND column_name = 'updated_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "products" ALTER COLUMN "updated_at" TYPE TIMESTAMP USING "updated_at" AT TIME ZONE 'UTC';
    END IF;
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'products' AND column_name = 'created_at' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "products" ALTER COLUMN "created_at" TYPE TIMESTAMP USING "created_at" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- ============================================
-- 6. sales_orders 表：重命名 required_date 回 delivery_date
-- ============================================
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sales_orders' AND column_name = 'required_date')
       AND NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'sales_orders' AND column_name = 'delivery_date') THEN
        ALTER TABLE "sales_orders" RENAME COLUMN "required_date" TO "delivery_date";
    END IF;
END $$;

COMMENT ON COLUMN "sales_orders"."delivery_date" IS '交货日期';

-- ============================================
-- 5. warehouses 表：重命名 warehouse_code 回 code
-- ============================================
DROP INDEX IF EXISTS "idx_warehouses_warehouse_code";

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'warehouses' AND column_name = 'warehouse_code')
       AND NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'warehouses' AND column_name = 'code') THEN
        ALTER TABLE "warehouses" RENAME COLUMN "warehouse_code" TO "code";
    END IF;
END $$;

-- 移除唯一约束
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'warehouses_warehouse_code_key') THEN
        ALTER TABLE "warehouses" DROP CONSTRAINT "warehouses_warehouse_code_key";
    END IF;
END $$;

COMMENT ON COLUMN "warehouses"."code" IS '仓库代码';

-- ============================================
-- 4. products 表：重命名 code 回 product_no
-- ============================================
DROP INDEX IF EXISTS "idx_products_code";

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'products' AND column_name = 'code')
       AND NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'products' AND column_name = 'product_no') THEN
        ALTER TABLE "products" RENAME COLUMN "code" TO "product_no";
    END IF;
END $$;

-- 移除唯一约束
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'products_code_key') THEN
        ALTER TABLE "products" DROP CONSTRAINT "products_code_key";
    END IF;
END $$;

COMMENT ON COLUMN "products"."product_no" IS '产品编号';

-- ============================================
-- 3. 移除 users 表的 TOTP 字段
-- ============================================
ALTER TABLE "users" DROP COLUMN IF EXISTS "is_totp_enabled";
ALTER TABLE "users" DROP COLUMN IF EXISTS "totp_secret";

-- ============================================
-- 2. 移除 omni_audit_logs 表
-- ============================================
DROP INDEX IF EXISTS "idx_omni_audit_logs_resource_type";
DROP INDEX IF EXISTS "idx_omni_audit_logs_action";
DROP INDEX IF EXISTS "idx_omni_audit_logs_tenant";
DROP INDEX IF EXISTS "idx_omni_audit_logs_created";
DROP INDEX IF EXISTS "idx_omni_audit_logs_user";
DROP INDEX IF EXISTS "idx_omni_audit_logs_trace";

DROP TABLE IF EXISTS "omni_audit_logs";

-- ============================================
-- 1. 移除 log_login 表
-- ============================================
DROP INDEX IF EXISTS "idx_log_login_status";
DROP INDEX IF EXISTS "idx_log_login_username";
DROP INDEX IF EXISTS "idx_log_login_time";
DROP INDEX IF EXISTS "idx_log_login_user";

DROP TABLE IF EXISTS "log_login";

-- ============================================
-- 完成
-- ============================================
