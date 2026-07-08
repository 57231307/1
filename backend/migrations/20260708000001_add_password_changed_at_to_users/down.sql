-- 批次 198 P0-2 回滚：移除 users.password_changed_at 列
ALTER TABLE "users" DROP COLUMN IF EXISTS "password_changed_at";
