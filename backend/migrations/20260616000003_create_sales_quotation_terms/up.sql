-- 销售报价单贸易条款
-- 用于存储报价单中各类贸易条款（物流/付款/样品/检验）
-- 创建时间: 2026-06-16

CREATE TABLE IF NOT EXISTS "sales_quotation_terms" (
    "id" BIGSERIAL PRIMARY KEY,
    "quotation_id" BIGINT NOT NULL REFERENCES "sales_quotations"("id") ON DELETE CASCADE,
    "term_type" VARCHAR(50) NOT NULL,
    "term_key" VARCHAR(100) NOT NULL,
    "term_value" TEXT NOT NULL,
    "sequence" INT NOT NULL DEFAULT 0,

    CONSTRAINT "chk_term_type" CHECK ("term_type" IN ('logistics','payment','sample','inspection'))
);

CREATE INDEX IF NOT EXISTS "idx_quotation_terms_quotation" ON "sales_quotation_terms"("quotation_id");
CREATE INDEX IF NOT EXISTS "idx_quotation_terms_type" ON "sales_quotation_terms"("term_type");
