-- 批次 112 P1-9：api_keys 表添加 created_by 列
-- 原 api_keys 表无 created_by 列，list/get 历史密钥无法回溯创建者，handler 传 0 占位。
-- 现新增 created_by 列（可空，兼容历史数据），由 create_api_key / regenerate_api_key 注入真实 user_id。

ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;

COMMENT ON COLUMN "api_keys"."created_by" IS 'API 密钥创建者用户 ID（批次 112 P1-9 修复：原表无此列，handler 传 0 占位）';

-- 创建外键索引便于按创建者查询
CREATE INDEX IF NOT EXISTS "idx_api_keys_created_by" ON "api_keys" ("created_by");
