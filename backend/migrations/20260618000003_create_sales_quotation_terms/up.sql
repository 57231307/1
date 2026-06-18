-- 销售报价单贸易条款
-- 用于存储报价单中各类贸易条款（物流/付款/样品/检验）
-- 创建时间: 2026-06-18
-- 关联计划: 2026-06-17-p12-batch1-quotation-port-plan.md PR-1
-- main 适配说明：
--   - ID / 外键类型与主表保持一致（SERIAL / INTEGER）
--   - term_type 枚举沿用 test 分支约定（logistics/payment/sample/inspection）

CREATE TABLE IF NOT EXISTS "sales_quotation_terms" (
    "id" SERIAL PRIMARY KEY,
    "quotation_id" INTEGER NOT NULL REFERENCES "sales_quotations"("id") ON DELETE CASCADE,
    "term_type" VARCHAR(50) NOT NULL,
    "term_key" VARCHAR(100) NOT NULL,
    "term_value" TEXT NOT NULL,
    "sequence" INT NOT NULL DEFAULT 0,

    CONSTRAINT "chk_term_type" CHECK ("term_type" IN ('logistics','payment','sample','inspection'))
);

CREATE INDEX IF NOT EXISTS "idx_quotation_terms_quotation" ON "sales_quotation_terms"("quotation_id");
CREATE INDEX IF NOT EXISTS "idx_quotation_terms_type" ON "sales_quotation_terms"("term_type");

COMMENT ON TABLE "sales_quotation_terms" IS '销售报价单贸易条款 - 物流/付款/样品/检验四类条款';
COMMENT ON COLUMN "sales_quotation_terms"."term_type" IS '条款类型 - logistics/payment/sample/inspection';
