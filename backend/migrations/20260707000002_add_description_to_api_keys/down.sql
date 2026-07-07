-- 回滚：移除 api_keys.description 列
ALTER TABLE api_keys DROP COLUMN IF EXISTS description;
