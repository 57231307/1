-- 回滚 product_color_prices 表
DROP INDEX IF EXISTS "idx_color_prices_product_color";
DROP TABLE IF EXISTS "product_color_prices";
