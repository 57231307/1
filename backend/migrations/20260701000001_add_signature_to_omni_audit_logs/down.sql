-- omni_audit_logs 签名列回滚迁移（P0 8-2 批次 53）
--
-- 反向删除 signature 列。

ALTER TABLE "omni_audit_logs" DROP COLUMN IF EXISTS "signature";
