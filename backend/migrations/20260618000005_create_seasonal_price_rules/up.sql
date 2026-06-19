-- 季节调价规则表 - P0-5 面料多色号定价扩展
-- 按季节自动调价（春夏 / 秋冬 / 节日）
-- 创建时间: 2026-06-18
-- 关联 spec: docs/superpowers/specs/2026-06-16-color-price-extension-design.md §3.6

CREATE TABLE IF NOT EXISTS "seasonal_price_rules" (
    "id" BIGSERIAL PRIMARY KEY,
    "rule_name" VARCHAR(100) NOT NULL,
    "season" VARCHAR(10) NOT NULL,
    "product_category_id" BIGINT REFERENCES "product_categories"("id"),
    "adjustment_type" VARCHAR(20) NOT NULL,
    "adjustment_value" DECIMAL(8,4) NOT NULL,
    "valid_from" DATE NOT NULL,
    "valid_until" DATE,
    "is_active" BOOLEAN NOT NULL DEFAULT true,
    "description" TEXT,
    "tenant_id" BIGINT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "chk_seasonal_type" CHECK ("season" IN ('SS', 'AW', 'HOLIDAY')),
    CONSTRAINT "chk_seasonal_adjustment_type" CHECK ("adjustment_type" IN ('percentage', 'fixed')),
    CONSTRAINT "chk_seasonal_valid_range" CHECK ("valid_until" IS NULL OR "valid_until" >= "valid_from")
);

-- 索引
CREATE INDEX IF NOT EXISTS "idx_seasonal_tenant_active" ON "seasonal_price_rules"("tenant_id", "is_active");
CREATE INDEX IF NOT EXISTS "idx_seasonal_season_valid" ON "seasonal_price_rules"("season", "valid_from", "valid_until");
CREATE INDEX IF NOT EXISTS "idx_seasonal_category" ON "seasonal_price_rules"("product_category_id");

-- 注释
COMMENT ON TABLE "seasonal_price_rules" IS '季节性调价规则表 - 春夏/秋冬/节日自动调价';
COMMENT ON COLUMN "seasonal_price_rules"."season" IS '季节：SS(春夏) / AW(秋冬) / HOLIDAY(节日)';
COMMENT ON COLUMN "seasonal_price_rules"."product_category_id" IS '品类（NULL = 全部产品）';
COMMENT ON COLUMN "seasonal_price_rules"."adjustment_type" IS '调整方式：percentage(百分比) / fixed(固定金额)';
COMMENT ON COLUMN "seasonal_price_rules"."adjustment_value" IS '调整值：+0.10 = 涨 10%，-0.05 = 降 5%，+1.5 = 加 1.5 元';
