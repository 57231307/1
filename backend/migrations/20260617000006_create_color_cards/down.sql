-- 回滚 color_cards 表
DROP INDEX IF EXISTS "idx_color_cards_type_season";
DROP INDEX IF EXISTS "idx_color_cards_status";
DROP INDEX IF EXISTS "idx_color_cards_tenant";
DROP TABLE IF EXISTS "color_cards";
