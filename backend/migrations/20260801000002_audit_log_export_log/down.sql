-- 回滚：审计日志导出二次审计表
DROP TRIGGER IF EXISTS "trg_audit_log_export_log_no_delete" ON "audit_log_export_log";
DROP TRIGGER IF EXISTS "trg_audit_log_export_log_no_update" ON "audit_log_export_log";
DROP FUNCTION IF EXISTS "fn_audit_log_export_log_immutable"();
DROP INDEX IF EXISTS "idx_audit_log_export_log_exported_at";
DROP INDEX IF EXISTS "idx_audit_log_export_log_user_id";
DROP TABLE IF EXISTS "audit_log_export_log";
