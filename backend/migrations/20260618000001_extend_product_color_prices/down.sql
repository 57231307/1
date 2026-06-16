-- 回滚 product_color_prices 表扩展 - P0-5
DROP INDEX IF EXISTS "idx_color_prices_approval";
DROP INDEX IF EXISTS "idx_color_prices_active";
DROP INDEX IF EXISTS "idx_color_prices_season";
DROP INDEX IF EXISTS "idx_color_prices_customer";
DROP INDEX IF EXISTS "idx_color_prices_tenant";

ALTER TABLE "product_color_prices" DROP CONSTRAINT IF EXISTS "chk_color_price_season";
ALTER TABLE "product_color_prices" DROP CONSTRAINT IF EXISTS "chk_color_price_approval_status";

ALTER TABLE "product_color_prices" DROP COLUMN IF EXISTS "approval_status";
ALTER TABLE "product_color_prices" DROP COLUMN IF EXISTS "approved_at";
ALTER TABLE "product_color_prices" DROP COLUMN IF EXISTS "approved_by";
ALTER TABLE "product_color_prices" DROP COLUMN IF EXISTS "created_by";
ALTER TABLE "product_color_prices" DROP COLUMN IF EXISTS "priority";
ALTER TABLE "product_color_prices" DROP COLUMN IF EXISTS "is_active";
ALTER TABLE "product_color_prices" DROP COLUMN IF EXISTS "season";
ALTER TABLE "product_color_prices" DROP COLUMN IF EXISTS "customer_id";
ALTER TABLE "product_color_prices" DROP COLUMN IF EXISTS "max_quantity";
ALTER TABLE "product_color_prices" DROP COLUMN IF EXISTS "tenant_id";
