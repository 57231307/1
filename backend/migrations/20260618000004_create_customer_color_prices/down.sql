-- 回滚 customer_color_prices 表 - P0-5
DROP INDEX IF EXISTS "idx_cust_color_price_valid";
DROP INDEX IF EXISTS "idx_cust_color_price_tenant";
DROP INDEX IF EXISTS "idx_cust_color_price_product_color";
DROP INDEX IF EXISTS "idx_cust_color_price_customer";
DROP TABLE IF EXISTS "customer_color_prices";
