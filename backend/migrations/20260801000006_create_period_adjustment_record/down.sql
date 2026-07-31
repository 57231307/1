-- V15 P2 B05-P2-10：期末调整记录表（回滚）
-- 删除顺序：先删索引，再删表（IF EXISTS 保证幂等）
DROP INDEX IF EXISTS "idx_period_adjustment_record_type";
DROP INDEX IF EXISTS "idx_period_adjustment_record_status";
DROP INDEX IF EXISTS "idx_period_adjustment_record_period";
DROP INDEX IF EXISTS "uq_period_adjustment_record_no";
DROP TABLE IF EXISTS "period_adjustment_record";
