-- 定制订单全流程跟踪模块 migration
-- 创建 5 张核心表：custom_orders / process_nodes / process_logs / quality_issues / after_sales
-- 创建时间: 2026-06-17
-- 关联 spec: docs/superpowers/specs/2026-06-16-custom-order-design.md

-- 1. 定制订单主表：记录定制订单基础信息和 5 阶段工艺状态
CREATE TABLE IF NOT EXISTS "custom_orders" (
    "id" BIGSERIAL PRIMARY KEY,
    "order_no" VARCHAR(50) UNIQUE NOT NULL,
    "customer_id" BIGINT NOT NULL REFERENCES "customers"("id"),
    "product_id" BIGINT NOT NULL REFERENCES "products"("id"),
    "color_id" BIGINT REFERENCES "product_colors"("id"),
    "spec" VARCHAR(200) NOT NULL,
    "quantity" DECIMAL(18,2) NOT NULL CHECK ("quantity" > 0),
    "unit" VARCHAR(20) NOT NULL DEFAULT 'm',
    "custom_requirements" JSONB NOT NULL DEFAULT '{}'::jsonb,
    "yarn_spec" VARCHAR(200),
    "dye_method" VARCHAR(50),
    "finishing_method" VARCHAR(50),
    "status" VARCHAR(30) NOT NULL DEFAULT 'draft',
    "expected_delivery_date" DATE,
    "actual_delivery_date" DATE,
    "sales_order_id" BIGINT REFERENCES "sales_orders"("id"),
    "total_amount" DECIMAL(18,2),
    "currency" VARCHAR(10) NOT NULL DEFAULT 'CNY',
    "tenant_id" BIGINT NOT NULL,
    "created_by" BIGINT,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "chk_custom_order_status" CHECK ("status" IN (
        'draft', 'yarn_purchasing', 'dyeing', 'finishing',
        'delivery', 'after_sales', 'completed', 'cancelled'
    ))
);

CREATE INDEX IF NOT EXISTS "idx_custom_orders_tenant" ON "custom_orders"("tenant_id");
CREATE INDEX IF NOT EXISTS "idx_custom_orders_customer" ON "custom_orders"("customer_id");
CREATE INDEX IF NOT EXISTS "idx_custom_orders_status" ON "custom_orders"("status");
CREATE INDEX IF NOT EXISTS "idx_custom_orders_sales_order" ON "custom_orders"("sales_order_id");

COMMENT ON TABLE "custom_orders" IS '定制订单主表 - 客户特殊定制订单跟踪';
COMMENT ON COLUMN "custom_orders"."status" IS '订单状态：draft(草稿) / yarn_purchasing(纱线采购) / dyeing(染整) / finishing(后整理) / delivery(交付) / after_sales(售后) / completed(已完成) / cancelled(已取消)';
COMMENT ON COLUMN "custom_orders"."custom_requirements" IS '客户定制要求（特殊工艺、克重、幅宽等）JSONB';
COMMENT ON COLUMN "custom_orders"."yarn_spec" IS '指定纱线规格';
COMMENT ON COLUMN "custom_orders"."dye_method" IS '染色工艺方法';
COMMENT ON COLUMN "custom_orders"."finishing_method" IS '后整理工艺方法';
