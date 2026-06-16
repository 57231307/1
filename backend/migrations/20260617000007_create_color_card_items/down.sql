-- 回滚 color_card_items 表
DROP INDEX IF EXISTS "idx_color_items_price";
DROP INDEX IF EXISTS "idx_color_items_dye_recipe";
DROP INDEX IF EXISTS "idx_color_items_tenant";
DROP INDEX IF EXISTS "idx_color_items_cncs";
DROP INDEX IF EXISTS "idx_color_items_pantone";
DROP INDEX IF EXISTS "idx_color_items_code";
DROP INDEX IF EXISTS "idx_color_items_card";
DROP TABLE IF EXISTS "color_card_items";
