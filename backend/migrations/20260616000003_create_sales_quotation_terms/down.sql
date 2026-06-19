-- 回滚 sales_quotation_terms 表
DROP INDEX IF EXISTS "idx_quotation_terms_type";
DROP INDEX IF EXISTS "idx_quotation_terms_quotation";
DROP TABLE IF EXISTS "sales_quotation_terms";
