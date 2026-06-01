-- Migration 029: 修复omni_audit_logs表，添加缺失的列

-- 添加trace_id列
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "trace_id" VARCHAR(100);

-- 添加span_id列
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "span_id" VARCHAR(50);

-- 添加parent_span_id列
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "parent_span_id" VARCHAR(50);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_omni_audit_logs_trace_id ON "omni_audit_logs"("trace_id");
