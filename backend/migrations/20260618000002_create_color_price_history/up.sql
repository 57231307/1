-- 价格历史表 - P0-5 面料多色号定价扩展
-- 记录每次调价的变更前/后价格、操作人、原因、审批信息
-- 创建时间: 2026-06-18
-- 关联 spec: docs/superpowers/specs/2026-06-16-color-price-extension-design.md §3.3

CREATE TABLE IF NOT EXISTS "color_price_history" (
    "id" BIGSERIAL PRIMARY KEY,
    "product_color_price_id" BIGINT NOT NULL REFERENCES "product_color_prices"("id"),
    "old_price" DECIMAL(18,6) NOT NULL,
    "new_price" DECIMAL(18,6) NOT NULL,
    "currency" VARCHAR(10) NOT NULL DEFAULT 'CNY',
    "change_type" VARCHAR(20) NOT NULL,
    "change_reason" TEXT,
    "change_percent" DECIMAL(8,4),
    "quantity" DECIMAL(18,2),
    "operated_by" BIGINT NOT NULL REFERENCES "users"("id"),
    "operated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "approved_by" BIGINT REFERENCES "users"("id"),
    "approved_at" TIMESTAMPTZ,
    "tenant_id" BIGINT NOT NULL,
    CONSTRAINT "chk_history_change_type" CHECK ("change_type" IN ('manual', 'batch', 'seasonal', 'customer_specific', 'tier'))
);

-- 索引
CREATE INDEX IF NOT EXISTS "idx_price_history_price" ON "color_price_history"("product_color_price_id");
CREATE INDEX IF NOT EXISTS "idx_price_history_operated_at" ON "color_price_history"("operated_at");
CREATE INDEX IF NOT EXISTS "idx_price_history_tenant" ON "color_price_history"("tenant_id");
CREATE INDEX IF NOT EXISTS "idx_price_history_change_type" ON "color_price_history"("change_type");
CREATE INDEX IF NOT EXISTS "idx_price_history_operator" ON "color_price_history"("operated_by");

-- 注释
COMMENT ON TABLE "color_price_history" IS '色号价格变更历史表 - 纺织行业价格审计与回溯';
COMMENT ON COLUMN "color_price_history"."change_type" IS '变更类型：manual(手工) / batch(批量) / seasonal(季节) / customer_specific(客户专属) / tier(阶梯)';
COMMENT ON COLUMN "color_price_history"."change_percent" IS '涨跌幅（百分比，正数为涨，负数为跌）';
COMMENT ON COLUMN "color_price_history"."quantity" IS '触发价格的数量（阶梯价场景）';
