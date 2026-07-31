-- V15 缺陷 10-4 修复：审计日志导出二次审计机制（防篡改）
--
-- 背景：
--   原 export_audit_logs handler 仅把"导出操作"写回 audit_logs 表本身，
--   审计员（admin）可查/改自身导出记录，无法满足"审计员不能篡改自身记录"的合规要求
--   （SOC2 / ISO27001 / 中国《数据安全法》第 32 条）。
--
-- 方案：
--   新建独立表 audit_log_export_log，记录每一次审计日志导出操作，
--   通过触发器禁止 UPDATE / DELETE（仅允许 INSERT），实现防篡改。
--   导出文件 SHA256 指纹留存，支持事后比对验证文件未被替换。
--
-- 关联：
--   - .monkeycode/doto.md §0.0.2 打印功能未完成项 缺陷 10-4
--   - .monkeycode/docs/audits/v15/batch-11/audit-report.md

CREATE TABLE IF NOT EXISTS "audit_log_export_log" (
    "id"                       SERIAL PRIMARY KEY,
    "exporter_user_id"         INTEGER NOT NULL,
    "exporter_username"        VARCHAR(255) NOT NULL,
    "export_query_filter"      TEXT,
    "export_record_count"      INTEGER NOT NULL CHECK ("export_record_count" >= 0),
    "export_file_format"       VARCHAR(16) NOT NULL DEFAULT 'xlsx',
    "export_file_hash_sha256"  CHAR(64),
    "export_file_size_bytes"   BIGINT CHECK ("export_file_size_bytes" IS NULL OR "export_file_size_bytes" >= 0),
    "export_ip_address"        VARCHAR(64),
    "export_user_agent"        TEXT,
    "export_request_id"        VARCHAR(64),
    "exported_at"              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 索引：按导出人 / 导出时间检索
CREATE INDEX IF NOT EXISTS "idx_audit_log_export_log_user_id"
    ON "audit_log_export_log" ("exporter_user_id");
CREATE INDEX IF NOT EXISTS "idx_audit_log_export_log_exported_at"
    ON "audit_log_export_log" ("exported_at" DESC);

-- ============================================================
-- 防篡改触发器：禁止 UPDATE / DELETE，仅允许 INSERT
-- ============================================================
CREATE OR REPLACE FUNCTION "fn_audit_log_export_log_immutable"()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION
        'audit_log_export_log 为防篡改表，禁止 UPDATE / DELETE 操作（导出记录只能追加）'
        USING ERRCODE = 'check_violation';
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS "trg_audit_log_export_log_no_update" ON "audit_log_export_log";
CREATE TRIGGER "trg_audit_log_export_log_no_update"
    BEFORE UPDATE ON "audit_log_export_log"
    FOR EACH ROW
    EXECUTE FUNCTION "fn_audit_log_export_log_immutable"();

DROP TRIGGER IF EXISTS "trg_audit_log_export_log_no_delete" ON "audit_log_export_log";
CREATE TRIGGER "trg_audit_log_export_log_no_delete"
    BEFORE DELETE ON "audit_log_export_log"
    FOR EACH ROW
    EXECUTE FUNCTION "fn_audit_log_export_log_immutable"();

COMMENT ON TABLE "audit_log_export_log" IS
    'V15 缺陷 10-4：审计日志导出二次审计表，防篡改（仅 INSERT，触发器禁止 UPDATE/DELETE）';
COMMENT ON COLUMN "audit_log_export_log.export_file_hash_sha256" IS
    '导出文件 SHA256 指纹，事后比对验证文件未被替换';
