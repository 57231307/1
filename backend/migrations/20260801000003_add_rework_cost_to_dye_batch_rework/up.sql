-- V15 P2 B05-P2-2：dye_batch_rework 表新增 rework_cost 字段
-- 记录每次回修的成本金额，按 rework_type 分类统计（re_dye 重染 / replenish_dye 补染）
-- 字段可为空（历史数据无此字段，新数据由业务层按需写入）
ALTER TABLE dye_batch_rework
    ADD COLUMN IF NOT EXISTS rework_cost NUMERIC(14, 4);

COMMENT ON COLUMN dye_batch_rework.rework_cost IS '回修成本（V15 P2 B05-P2-2）：按 rework_type 分类统计，re_dye 整缸重染成本高 / replenish_dye 局部补染成本低';
