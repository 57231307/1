-- 批次 112 P1-9：回滚 api_keys 表的 created_by 列

DROP INDEX IF EXISTS "idx_api_keys_created_by";
ALTER TABLE "api_keys" DROP COLUMN IF EXISTS "created_by";
