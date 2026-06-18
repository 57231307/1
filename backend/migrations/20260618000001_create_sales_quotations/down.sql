-- 回滚 sales_quotations 表
DROP INDEX IF EXISTS "idx_quotations_date";
DROP INDEX IF EXISTS "idx_quotations_sales_user";
DROP INDEX IF EXISTS "idx_quotations_valid_until";
DROP INDEX IF EXISTS "idx_quotations_status";
DROP INDEX IF EXISTS "idx_quotations_customer";
DROP TABLE IF EXISTS "sales_quotations";
