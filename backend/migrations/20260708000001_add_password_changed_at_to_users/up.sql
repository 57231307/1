-- 批次 198 P0-2：users 表添加 password_changed_at 列
-- 原 users 表无 password_changed_at 列，PasswordPolicyService::is_expired 无法持久化密码修改时间锚点。
-- 现新增 password_changed_at 列（可空，兼容历史数据），由 change_password 注入当前时间，
-- 由 login 调用 PasswordPolicyService::is_expired 检查密码是否过期（默认 90 天）。

ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "password_changed_at" TIMESTAMP WITH TIME ZONE;

COMMENT ON COLUMN "users"."password_changed_at" IS '密码最后修改时间（批次 198 P0-2 修复：密码过期策略锚点，None 表示历史用户未设置）';

-- 历史用户初始化为当前时间，避免存量用户登录即被判为过期
UPDATE "users" SET "password_changed_at" = CURRENT_TIMESTAMP WHERE "password_changed_at" IS NULL;
