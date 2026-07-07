-- 批次 158 v11 真实接入：api_keys 表添加 description 列（规则 0/1/2 真实实现）
-- 原 UpdateApiKeyGwRequest 含 description 字段，但 api_keys 表无对应列，
-- 字段被 #[allow(dead_code)] 标注。现扩展 schema 接入业务，移除 allow 标注。
ALTER TABLE api_keys ADD COLUMN IF NOT EXISTS description TEXT;

COMMENT ON COLUMN api_keys.description IS 'API 密钥描述（可选）';
