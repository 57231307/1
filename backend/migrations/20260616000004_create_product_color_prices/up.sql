-- 色号价格表（预先建，报价单依赖）
-- 用于存储每个产品色号在指定币种/客户等级下的基础价
-- 创建时间: 2026-06-16

CREATE TABLE IF NOT EXISTS "product_color_prices" (
    "id" BIGSERIAL PRIMARY KEY,
    "product_id" BIGINT NOT NULL REFERENCES "products"("id"),
    "color_id" BIGINT NOT NULL REFERENCES "product_colors"("id"),
    "currency" VARCHAR(10) NOT NULL DEFAULT 'CNY',
    "base_price" DECIMAL(18,6) NOT NULL,
    "effective_from" DATE NOT NULL,
    "effective_to" DATE,
    "customer_level" VARCHAR(20),
    "min_quantity" DECIMAL(18,2) DEFAULT 1,
    "notes" TEXT,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT "uq_color_price" UNIQUE ("product_id", "color_id", "currency", "customer_level", "effective_from")
);

CREATE INDEX IF NOT EXISTS "idx_color_prices_product_color" ON "product_color_prices"("product_id", "color_id");
