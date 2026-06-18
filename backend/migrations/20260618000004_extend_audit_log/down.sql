-- 审计日志增强迁移回滚（P13 批 1 P3-2）
--
-- 反向删除扩展字段与索引。
-- 注意：使用 IF EXISTS 保护，对应 m0001 创建的原始列不会被误删。

DROP INDEX IF EXISTS "idx_audit_log_tenant_created";
DROP INDEX IF EXISTS "idx_audit_log_request_id";
DROP INDEX IF EXISTS "idx_audit_log_severity";
DROP INDEX IF EXISTS "idx_audit_log_op_type";

ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "after_snapshot";
ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "before_snapshot";
ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "request_id";
ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "severity";
ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "operation_type";
