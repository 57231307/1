-- v11 批次 141：2FA 恢复码后端实现
-- 新增 totp_recovery_codes 字段存储恢复码的 bcrypt 哈希（JSON 数组格式）
-- 恢复码明文仅在生成时返回给用户，服务端只存哈希
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "totp_recovery_codes" TEXT;
COMMENT ON COLUMN "users"."totp_recovery_codes" IS 'TOTP 恢复码哈希数组（JSON 格式，bcrypt 哈希）';
