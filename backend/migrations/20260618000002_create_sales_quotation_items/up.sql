-- 销售报价单明细
-- 用于存储报价单中每个产品/色号的行项目
-- 创建时间: 2026-06-18
-- 关联计划: 2026-06-17-p12-batch1-quotation-port-plan.md PR-1
-- main 适配说明：
--   - ID / 外键类型与主表保持一致（SERIAL / INTEGER）
--   - product_id / color_id 引用 main 已有的 products / product_colors 表

CREATE TABLE IF NOT EXISTS "sales_quotation_items" (
    "id" SERIAL PRIMARY KEY,
    "quotation_id" INTEGER NOT NULL REFERENCES "sales_quotations"("id") ON DELETE CASCADE,

    "product_id" INTEGER NOT NULL REFERENCES "products"("id"),
    "color_id" INTEGER REFERENCES "product_colors"("id"),
    "color_code" VARCHAR(50),
    "pantone_code" VARCHAR(50),
    "cncs_code" VARCHAR(50),

    "specification" TEXT,
    "unit" VARCHAR(20) NOT NULL,

    "quantity" DECIMAL(18,2) NOT NULL,
    "unit_price" DECIMAL(18,6) NOT NULL,
    "unit_price_with_tax" DECIMAL(18,6) NOT NULL,
    "amount" DECIMAL(18,2) NOT NULL,
    "amount_with_tax" DECIMAL(18,2) NOT NULL,

    "tier_pricing" JSONB,
    "discount_rate" DECIMAL(5,2) DEFAULT 0,
    "discount_amount" DECIMAL(18,2) DEFAULT 0,

    "notes" TEXT,
    "sequence" INT NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS "idx_quotation_items_quotation" ON "sales_quotation_items"("quotation_id");
CREATE INDEX IF NOT EXISTS "idx_quotation_items_product" ON "sales_quotation_items"("product_id");
CREATE INDEX IF NOT EXISTS "idx_quotation_items_color" ON "sales_quotation_items"("color_id");

COMMENT ON TABLE "sales_quotation_items" IS '销售报价单明细 - 行项目（产品/色号/数量/单价/金额）';
COMMENT ON COLUMN "sales_quotation_items"."quotation_id" IS '报价单 ID（外键 sales_quotations.id，级联删除）';
COMMENT ON COLUMN "sales_quotation_items"."tier_pricing" IS '阶梯价 JSON 数据';
