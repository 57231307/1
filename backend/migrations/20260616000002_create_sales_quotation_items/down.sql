-- 回滚 sales_quotation_items 表
DROP INDEX IF EXISTS "idx_quotation_items_color";
DROP INDEX IF EXISTS "idx_quotation_items_product";
DROP INDEX IF EXISTS "idx_quotation_items_quotation";
DROP TABLE IF EXISTS "sales_quotation_items";
