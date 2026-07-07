-- 批次 158 v11 真实接入：密码策略服务 - 密码历史持久化
-- 配合 password_policy_service.rs 的 PasswordHistory / validate_with_history 方法
-- 每次修改密码后将旧密码哈希写入此表，校验新密码时查询最近 N 条记录
CREATE TABLE IF NOT EXISTS "password_histories" (
    "id" SERIAL PRIMARY KEY,
    "user_id" INTEGER NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
    "password_hash" TEXT NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS "idx_password_histories_user_id" ON "password_histories"("user_id");
CREATE INDEX IF NOT EXISTS "idx_password_histories_user_created" ON "password_histories"("user_id", "created_at" DESC);
COMMENT ON TABLE "password_histories" IS '密码历史表（批次 158 v11 真实接入 PasswordPolicyService）';
