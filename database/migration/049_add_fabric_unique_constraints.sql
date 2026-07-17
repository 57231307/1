-- ============================================================================
-- Migration 049: V15 P0-F02 面料行业关键业务约束 UNIQUE 补全
-- 依据：V15 P0 审计报告 类一 P0-F02（batch-01 P0-01-01）
-- 业务背景：面料行业四维标识（product_id + color_no + dye_lot_no + batch_no）
--   核心业务表缺少联合唯一约束，导致同维度可存在多条重复记录，破坏数据一致性
-- 术语说明（用户 2026-07-17 澄清）：
--   - batch_no：缸号（=染色批次号，同一概念不同叫法）
--   - dye_lot_no：染色批号（lot 概念，防色差混批）
--   - lot_no：染缸号（purchase_receipt_item 表历史字段名，与 dye_lot_no 同义）
--
-- 已有约束核对（migration 032 已实现，本批次不重复）：
--   ✅ product_colors: UNIQUE(product_id, color_no) — migration 032 第 11 行
--   ✅ inventory_stocks: idx_inv_stock_four_dim_unique(warehouse_id, product_id, color_no, batch_no, COALESCE(dye_lot_no, '')) — migration 032 第 17-18 行
--   ✅ inventory_piece: UNIQUE(dye_lot_id, piece_no) — migration 032 第 39 行
--
-- 本批次新增 3 项联合唯一约束（按实际 schema 字段名调整任务定义）：
--   1. dye_batch: UNIQUE(greige_fabric_id, color_no, dye_lot_no, batch_no)
--      - 任务定义字段名 fabric_id/color_id 实际为 greige_fabric_id/color_no（按 dye_batch.rs Model）
--      - V15 P0-F01（Batch 469）已新增 dye_lot_no 字段，本批次可建立完整四维唯一约束
--   2. sales_delivery_item: UNIQUE(delivery_id, sales_order_item_id, dye_lot_no)
--      - 任务定义字段名 order_id/item_id 实际为 delivery_id/sales_order_item_id
--      - 表中无 batch_no 字段，仅有 dye_lot_no（销售发货按染色批号区分）
--   3. purchase_receipt_item: UNIQUE(receipt_id, order_item_id, batch_no, lot_no)
--      - 任务定义字段名 item_id 实际为 order_item_id
--      - lot_no 为历史字段名，与 dye_lot_no 同义（染缸号）
--      - batch_no + lot_no 联合保证同入库单同明细下不重复
--
-- 关联文件：doto.md P0-F02 / V15 审计报告 类一 P0-F02
-- ============================================================================

-- ==================== 1. dye_batch: 四维联合唯一约束 ====================
-- 业务语义：同一坯布同一色号同一染色批号下，缸号唯一
-- 修复前：仅 batch_no 单字段全局 UNIQUE（migrations/20260518000002/up.sql:4）
--   导致不同坯布/色号/染色批号下不能有相同缸号（业务上不应有此限制）
-- 修复后：四维联合唯一（greige_fabric_id, color_no, dye_lot_no, batch_no）
--   允许不同坯布/色号/染色批号下有相同缸号，同维度下缸号唯一
-- 注意：greige_fabric_id 和 color_no 可为 NULL（历史数据），用 COALESCE 处理
CREATE UNIQUE INDEX IF NOT EXISTS idx_dye_batch_four_dim_unique
ON dye_batch (COALESCE(greige_fabric_id, 0), COALESCE(color_no, ''), dye_lot_no, batch_no);

-- ==================== 2. sales_delivery_item: 发货明细联合唯一约束 ====================
-- 业务语义：同一发货单同一销售订单明细下，同一染色批号唯一（防止重复发货）
-- 修复前：无任何唯一约束，同发货单同明细可重复插入相同染色批号记录
-- 修复后：UNIQUE(delivery_id, sales_order_item_id, dye_lot_no)
-- 注意：sales_order_item_id 可为 NULL（无关联订单的直发单），用 COALESCE 处理
CREATE UNIQUE INDEX IF NOT EXISTS idx_sales_delivery_item_unique
ON sales_delivery_item (delivery_id, COALESCE(sales_order_item_id, 0), dye_lot_no);

-- ==================== 3. purchase_receipt_item: 入库明细联合唯一约束 ====================
-- 业务语义：同一入库单同一订单明细下，同批次号+染缸号唯一（防止重复入库）
-- 修复前：无任何唯一约束，同入库单同明细可重复插入相同批次/染缸号记录
-- 修复后：UNIQUE(receipt_id, order_item_id, batch_no, lot_no)
-- 注意：order_item_id 和 batch_no 可为 NULL，用 COALESCE 处理
--   lot_no 为历史字段名，与 dye_lot_no 同义（染缸号/染色批号）
CREATE UNIQUE INDEX IF NOT EXISTS idx_purchase_receipt_item_unique
ON purchase_receipt_item (receipt_id, COALESCE(order_item_id, 0), COALESCE(batch_no, ''), COALESCE(lot_no, ''));

-- ============================================================================
-- 验证 SQL（手动验证用，迁移脚本不会执行）
-- ============================================================================
-- SELECT indexname, indexdef FROM pg_indexes
-- WHERE indexname IN ('idx_dye_batch_four_dim_unique', 'idx_sales_delivery_item_unique', 'idx_purchase_receipt_item_unique');
-- 预期：3 行记录，均为 UNIQUE INDEX
