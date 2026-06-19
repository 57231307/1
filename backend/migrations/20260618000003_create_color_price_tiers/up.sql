-- 阶梯定价表 - P0-5 面料多色号定价扩展
-- 数量区间 × 客户等级 → 阶梯价
-- 创建时间: 2026-06-18
-- 关联 spec: docs/superpowers/specs/2026-06-16-color-price-extension-design.md §3.4

CREATE TABLE IF NOT EXISTS "color_price_tiers" (
    "id" BIGSERIAL PRIMARY KEY,
    "product_color_price_id" BIGINT NOT NULL REFERENCES "product_color_prices"("id"),
    "min_quantity" DECIMAL(18,2) NOT NULL DEFAULT 1,
    "max_quantity" DECIMAL(18,2),
    "tier_price" DECIMAL(18,6) NOT NULL,
    "customer_level" VARCHAR(20),
    "sequence" INT NOT NULL DEFAULT 0,
    "tenant_id" BIGINT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "uq_tier_price" UNIQUE ("product_color_price_id", "min_quantity", "customer_level"),
    CONSTRAINT "chk_tier_customer_level" CHECK ("customer_level" IS NULL OR "customer_level" IN ('VIP', 'NORMAL', 'GOLD', 'SILVER'))
);

-- 索引
CREATE INDEX IF NOT EXISTS "idx_price_tiers_price" ON "color_price_tiers"("product_color_price_id");
CREATE INDEX IF NOT EXISTS "idx_price_tiers_sequence" ON "color_price_tiers"("product_color_price_id", "sequence");
CREATE INDEX IF NOT EXISTS "idx_price_tiers_tenant" ON "color_price_tiers"("tenant_id");

-- 注释
COMMENT ON TABLE "color_price_tiers" IS '色号价格阶梯表 - 数量越多价越低，支持按客户等级叠加';
COMMENT ON COLUMN "color_price_tiers"."min_quantity" IS '起始数量（含）';
COMMENT ON COLUMN "color_price_tiers"."max_quantity" IS '结束数量（不含），NULL = 无限';
COMMENT ON COLUMN "color_price_tiers"."tier_price" IS '阶梯价';
COMMENT ON COLUMN "color_price_tiers"."customer_level" IS '客户等级（NULL = 通用，VIP/NORMAL/GOLD/SILVER 各自阶梯）';
COMMENT ON COLUMN "color_price_tiers"."sequence" IS '阶梯顺序（数值小 = 低阶梯）';
