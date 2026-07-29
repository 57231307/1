-- V15 P1-21 缺陷 2.2：委外收回单关联质检记录
--
-- 审计报告 batch-18 缺陷 2.2：委外收回未走质检流程
-- 修复：trigger_quality_inspection 已创建质检记录，但 inspection_id 未持久化到收回单
-- 本迁移新增 inspection_id 字段，建立委外收回→质检的关联链路
--
-- 字段说明：
-- - inspection_id：关联 quality_inspection_records.id（可空，未触发质检时为 NULL）

ALTER TABLE outsourcing_receipt
    ADD COLUMN IF NOT EXISTS inspection_id INTEGER;

COMMENT ON COLUMN outsourcing_receipt.inspection_id IS
    '缺陷 2.2：关联质检记录 ID（确认收回时自动创建质检记录并回写）';

-- 创建索引支持按质检记录反查委外收回单
CREATE INDEX IF NOT EXISTS idx_outsourcing_receipt_inspection_id
    ON outsourcing_receipt (inspection_id)
    WHERE inspection_id IS NOT NULL;
