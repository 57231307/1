-- 回滚：删除 roles 表的 data_scope 字段
ALTER TABLE roles DROP COLUMN IF EXISTS data_scope;
