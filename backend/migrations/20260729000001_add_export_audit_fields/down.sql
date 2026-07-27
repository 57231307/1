-- 审计日志导出专属字段回滚（V15 P1-3-3）

DROP INDEX IF EXISTS "idx_audit_log_approval_token";
DROP INDEX IF EXISTS "idx_audit_log_export_count";

ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "export_watermark_user";
ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "export_approval_token";
ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "export_file_format";
ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "export_query_filter";
ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "export_record_count";
