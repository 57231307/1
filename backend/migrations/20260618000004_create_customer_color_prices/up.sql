-- 客户专属价表 - P0-5 面料多色号定价扩展
-- 战略客户 / 大客户协议价（最高优先级）
-- 创建时间: 2026-06-18
-- 关联 spec: docs/superpowers/specs/2026-06-16-color-price-extension-design.md §3.5

CREATE TABLE IF NOT EXISTS "customer_color_prices" (
    "id" BIGSERIAL PRIMARY KEY,
    "customer_id" BIGINT NOT NULL REFERENCES "customers"("id"),
    "product_id" BIGINT NOT NULL REFERENCES "products"("id"),
    "color_id" BIGINT NOT NULL REFERENCES "product_colors"("id"),
    "special_price" DECIMAL(18,6) NOT NULL,
    "discount_percent" DECIMAL(5,2),
    "currency" VARCHAR(10) NOT NULL DEFAULT 'CNY',
    "valid_from" DATE NOT NULL,
    "valid_until" DATE,
    "notes" TEXT,
    "approved_by" BIGINT REFERENCES "users"("id"),
    "approved_at" TIMESTAMPTZ,
    "tenant_id" BIGINT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "uq_customer_color_price" UNIQUE ("customer_id", "product_id", "color_id", "valid_from")
);

-- 索引
CREATE INDEX IF NOT EXISTS "idx_cust_color_price_customer" ON "customer_color_prices"("customer_id");
CREATE INDEX IF NOT EXISTS "idx_cust_color_price_product_color" ON "customer_color_prices"("product_id", "color_id");
CREATE INDEX IF NOT EXISTS "idx_cust_color_price_tenant" ON "customer_color_prices"("tenant_id");
CREATE INDEX IF NOT EXISTS "idx_cust_color_price_valid" ON "customer_color_prices"("valid_from", "valid_until");

-- 注释
COMMENT ON TABLE "customer_color_prices" IS '客户专属价格表 - 战略客户大客户协议价（最高优先级）';
COMMENT ON COLUMN "customer_color_prices"."special_price" IS '专属价格（直接覆盖所有其他规则）';
COMMENT ON COLUMN "customer_color_prices"."discount_percent" IS '折扣率（0.95 = 95 折，0.85 = 85 折）';
COMMENT ON COLUMN "customer_color_prices"."valid_from" IS '生效日期';
COMMENT ON COLUMN "customer_color_prices"."valid_until" IS '失效日期（NULL = 长期有效）';
