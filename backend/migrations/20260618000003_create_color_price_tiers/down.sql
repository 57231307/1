-- 回滚 color_price_tiers 表 - P0-5
DROP INDEX IF EXISTS "idx_price_tiers_tenant";
DROP INDEX IF EXISTS "idx_price_tiers_sequence";
DROP INDEX IF EXISTS "idx_price_tiers_price";
DROP TABLE IF EXISTS "color_price_tiers";
