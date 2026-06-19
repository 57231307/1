-- 扩展 product_color_prices 表 - P0-5 面料多色号定价扩展
-- 添加字段：阶梯价区间、客户等级、季节、客户专属、优先级、审批等
-- 创建时间: 2026-06-18
-- 关联 spec: docs/superpowers/specs/2026-06-16-color-price-extension-design.md §3.2

-- 添加 max_quantity 阶梯价区间上限
ALTER TABLE "product_color_prices"
    ADD COLUMN IF NOT EXISTS "max_quantity" DECIMAL(18,2);

-- 添加 customer_id 客户专属（NULL = 通用）
ALTER TABLE "product_color_prices"
    ADD COLUMN IF NOT EXISTS "customer_id" BIGINT REFERENCES "customers"("id");

-- 添加 season 季节标签
ALTER TABLE "product_color_prices"
    ADD COLUMN IF NOT EXISTS "season" VARCHAR(10);

-- 添加 is_active 是否启用
ALTER TABLE "product_color_prices"
    ADD COLUMN IF NOT EXISTS "is_active" BOOLEAN NOT NULL DEFAULT true;

-- 添加 priority 优先级（数值大 = 优先级高）
ALTER TABLE "product_color_prices"
    ADD COLUMN IF NOT EXISTS "priority" INT NOT NULL DEFAULT 0;

-- 添加创建人 / 审批人 / 审批时间
ALTER TABLE "product_color_prices"
    ADD COLUMN IF NOT EXISTS "created_by" BIGINT REFERENCES "users"("id");

ALTER TABLE "product_color_prices"
    ADD COLUMN IF NOT EXISTS "approved_by" BIGINT REFERENCES "users"("id");

ALTER TABLE "product_color_prices"
    ADD COLUMN IF NOT EXISTS "approved_at" TIMESTAMPTZ;

-- 添加审批状态
ALTER TABLE "product_color_prices"
    ADD COLUMN IF NOT EXISTS "approval_status" VARCHAR(20) NOT NULL DEFAULT 'APPROVED';

-- 添加 tenant_id
ALTER TABLE "product_color_prices"
    ADD COLUMN IF NOT EXISTS "tenant_id" BIGINT NOT NULL DEFAULT 1;

-- 添加约束（CHECK）
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE constraint_name = 'chk_color_price_approval_status'
          AND table_name = 'product_color_prices'
    ) THEN
        ALTER TABLE "product_color_prices"
            ADD CONSTRAINT "chk_color_price_approval_status"
            CHECK ("approval_status" IN ('PENDING', 'APPROVED', 'REJECTED'));
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE constraint_name = 'chk_color_price_season'
          AND table_name = 'product_color_prices'
    ) THEN
        ALTER TABLE "product_color_prices"
            ADD CONSTRAINT "chk_color_price_season"
            CHECK ("season" IS NULL OR "season" IN ('SS', 'AW', 'HOLIDAY'));
    END IF;
END $$;

-- 添加索引
CREATE INDEX IF NOT EXISTS "idx_color_prices_tenant" ON "product_color_prices"("tenant_id");
CREATE INDEX IF NOT EXISTS "idx_color_prices_customer" ON "product_color_prices"("customer_id");
CREATE INDEX IF NOT EXISTS "idx_color_prices_season" ON "product_color_prices"("season");
CREATE INDEX IF NOT EXISTS "idx_color_prices_active" ON "product_color_prices"("is_active");
CREATE INDEX IF NOT EXISTS "idx_color_prices_approval" ON "product_color_prices"("approval_status");

-- 注释
COMMENT ON COLUMN "product_color_prices"."max_quantity" IS '阶梯价区间上限（NULL = 无限）';
COMMENT ON COLUMN "product_color_prices"."customer_id" IS '客户专属（NULL = 通用）';
COMMENT ON COLUMN "product_color_prices"."season" IS '季节标签：SS(春夏) / AW(秋冬) / HOLIDAY(节日) / NULL(通用)';
COMMENT ON COLUMN "product_color_prices"."is_active" IS '是否启用';
COMMENT ON COLUMN "product_color_prices"."priority" IS '优先级（数值大 = 优先级高）';
COMMENT ON COLUMN "product_color_prices"."approval_status" IS '审批状态：PENDING / APPROVED / REJECTED';
