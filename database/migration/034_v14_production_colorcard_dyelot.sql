-- 迁移脚本: 034_v14_production_colorcard_dyelot.sql
-- 描述: v14 复审批次 419 - 为生产订单/库存匹号/色卡借出记录补全面料行业追溯字段
-- 日期: 2026-07-15
-- 修复: F-P0-1（生产订单缺失缸号字段）+ F-P0-2（库存匹号缺失 color_no/dye_lot_no）+ T-P0-3（色卡借出记录缺失 dye_lot_no）

BEGIN;

-- =====================================================
-- 1. production_orders 表：添加 color_no/dye_lot_no/batch_no（F-P0-1）
-- =====================================================
-- 生产订单需要记录对应的色号/缸号/批号，支持按缸号追踪生产进度
ALTER TABLE production_orders ADD COLUMN IF NOT EXISTS color_no VARCHAR(50);
ALTER TABLE production_orders ADD COLUMN IF NOT EXISTS dye_lot_no VARCHAR(50);
ALTER TABLE production_orders ADD COLUMN IF NOT EXISTS batch_no VARCHAR(50);

-- 添加索引便于按缸号/色号/批号查询生产订单
CREATE INDEX IF NOT EXISTS idx_production_orders_color_no ON production_orders(color_no);
CREATE INDEX IF NOT EXISTS idx_production_orders_dye_lot_no ON production_orders(dye_lot_no);

-- =====================================================
-- 2. inventory_piece 表：添加 color_no/dye_lot_no（F-P0-2）
-- =====================================================
-- 库存匹号已有 dye_lot_id 外键和 batch_no，但缺少 color_no/dye_lot_no 字符串字段
-- 添加字符串字段便于直接查询，无需 JOIN batch_dye_lot 表
ALTER TABLE inventory_piece ADD COLUMN IF NOT EXISTS color_no VARCHAR(50);
ALTER TABLE inventory_piece ADD COLUMN IF NOT EXISTS dye_lot_no VARCHAR(50);

CREATE INDEX IF NOT EXISTS idx_inventory_piece_color_no ON inventory_piece(color_no);
CREATE INDEX IF NOT EXISTS idx_inventory_piece_dye_lot_no ON inventory_piece(dye_lot_no);

-- =====================================================
-- 3. color_card_borrow_records 表：添加 dye_lot_no（T-P0-3）
-- =====================================================
-- 色卡借出需要记录对应的缸号，支持按缸号追溯色卡去向
ALTER TABLE color_card_borrow_records ADD COLUMN IF NOT EXISTS dye_lot_no VARCHAR(50);

CREATE INDEX IF NOT EXISTS idx_color_card_borrow_dye_lot_no ON color_card_borrow_records(dye_lot_no);

COMMIT;

-- 验证迁移
SELECT 'Migration 034_v14_production_colorcard_dyelot completed successfully' AS status;
