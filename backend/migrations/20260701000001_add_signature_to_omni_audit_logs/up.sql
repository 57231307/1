-- omni_audit_logs 签名列迁移（P0 8-2 批次 53）
-- 创建时间: 2026-07-01
-- 关联修复: 八维度审计 P0 8-2 — 审计日志签名计算后丢弃，无防篡改
--
-- 向 omni_audit_logs 表添加 signature 列，存储 HMAC-SHA256 防篡改签名。
-- 签名材料：trace_id|event_type|action|payload
-- 使用 ADD COLUMN IF NOT EXISTS 防止迁移重入。

ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "signature" VARCHAR(128);

COMMENT ON COLUMN "omni_audit_logs"."signature" IS 'HMAC-SHA256 防篡改签名（trace_id|event_type|action|payload）';
