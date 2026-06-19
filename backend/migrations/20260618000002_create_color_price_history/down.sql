-- 回滚 color_price_history 表 - P0-5
DROP INDEX IF EXISTS "idx_price_history_operator";
DROP INDEX IF EXISTS "idx_price_history_change_type";
DROP INDEX IF EXISTS "idx_price_history_tenant";
DROP INDEX IF EXISTS "idx_price_history_operated_at";
DROP INDEX IF EXISTS "idx_price_history_price";
DROP TABLE IF EXISTS "color_price_history";
