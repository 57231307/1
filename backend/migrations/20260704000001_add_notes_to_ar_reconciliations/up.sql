-- ar_reconciliations 备注列迁移（批次 109 P1-1）
-- 创建时间: 2026-07-04
-- 关联修复: v7 复审 P1-1 — DTO/Request 中 notes 字段已对外暴露但未持久化
--   - CreateReconciliationRequest.notes（services/ar/mod.rs:45）
--   - UpdateReconciliationRequest.notes（services/ar/mod.rs:57）
--   - GenerateReconciliationRequest.notes（services/ar/mod.rs:152）
--
-- 向 ar_reconciliations 表添加 notes 列（TEXT，可选），存储对账单备注。
-- 使用 ADD COLUMN IF NOT EXISTS 防止迁移重入。

ALTER TABLE "ar_reconciliations" ADD COLUMN IF NOT EXISTS "notes" TEXT;
COMMENT ON COLUMN "ar_reconciliations"."notes" IS '对账单备注（批次 109 P1-1 修复：原 DTO 有字段但未持久化）';
