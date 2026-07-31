-- V15 P2 B05-P2-2 回滚：移除 rework_cost 字段
ALTER TABLE dye_batch_rework
    DROP COLUMN IF EXISTS rework_cost;
