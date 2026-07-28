-- 审计日志导出专属字段迁移（V15 P1-3-3）
-- 创建时间: 2026-07-29
-- 关联审计: /workspace/.monkeycode/docs/audits/v15/batch-11/audit-report.md 缺陷 3-3
--
-- 在 audit_logs 表基础上增量添加导出操作专属字段，支持：
-- - 导出条数（export_record_count）：单次导出数据行数，用于大批量导出识别
-- - 查询条件（export_query_filter）：导出时的筛选条件，用于追溯导出数据范围
-- - 文件格式（export_file_format）：xlsx/csv/pdf，用于格式合规审计
-- - 审批 token（export_approval_token）：二级审批 token，敏感数据导出追溯
-- - 水印用户（export_watermark_user）：导出文件水印中的用户名，二次泄露追溯
--
-- 全部使用 ADD COLUMN IF NOT EXISTS 防止迁移重入。

ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_record_count" INTEGER;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_query_filter" TEXT;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_file_format" VARCHAR(20);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_approval_token" VARCHAR(128);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_watermark_user" VARCHAR(100);

-- 索引优化：按导出条数筛选大批量导出（合规审查常用）
CREATE INDEX IF NOT EXISTS "idx_audit_log_export_count" ON "audit_logs"("export_record_count");
-- 索引优化：按审批 token 查询敏感数据导出追溯
CREATE INDEX IF NOT EXISTS "idx_audit_log_approval_token" ON "audit_logs"("export_approval_token");

COMMENT ON COLUMN "audit_logs"."export_record_count" IS 'V15 P1-3-3：导出数据行数，用于大批量导出识别（>80% 上限触发告警）';
COMMENT ON COLUMN "audit_logs"."export_query_filter" IS 'V15 P1-3-3：导出时的筛选条件 JSON，用于追溯导出数据范围';
COMMENT ON COLUMN "audit_logs"."export_file_format" IS 'V15 P1-3-3：导出文件格式（xlsx/csv/pdf），格式合规审计';
COMMENT ON COLUMN "audit_logs"."export_approval_token" IS 'V15 P1-3-3：二级审批 token（敏感数据导出），10 分钟有效期';
COMMENT ON COLUMN "audit_logs"."export_watermark_user" IS 'V15 P1-3-3：导出文件水印中的用户名，二次泄露追溯';
