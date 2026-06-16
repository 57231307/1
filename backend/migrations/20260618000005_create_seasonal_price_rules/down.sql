-- 回滚 seasonal_price_rules 表 - P0-5
DROP INDEX IF EXISTS "idx_seasonal_category";
DROP INDEX IF EXISTS "idx_seasonal_season_valid";
DROP INDEX IF EXISTS "idx_seasonal_tenant_active";
DROP TABLE IF EXISTS "seasonal_price_rules";
