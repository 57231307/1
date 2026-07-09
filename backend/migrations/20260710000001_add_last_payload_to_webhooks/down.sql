-- 批次 251 回滚：移除 webhooks.last_payload + last_event 列
ALTER TABLE "webhooks" DROP COLUMN IF EXISTS "last_payload";
ALTER TABLE "webhooks" DROP COLUMN IF EXISTS "last_event";
