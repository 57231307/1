-- 回滚：移除 webhooks 表 user_id 列（批次 320 M-4）

DROP INDEX IF EXISTS "idx_webhooks_user";

ALTER TABLE "webhooks" DROP COLUMN IF EXISTS "user_id";
