-- v11 批次 141：回滚 2FA 恢复码字段
ALTER TABLE "users" DROP COLUMN IF EXISTS "totp_recovery_codes";
