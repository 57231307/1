-- 回滚批次6：库存管理扩展
-- 创建时间: 2026-05-27
-- 描述: 删除库存管理扩展表

-- 删除表（按依赖关系逆序）
DROP TABLE IF EXISTS "warehouse_locations" CASCADE;
DROP TABLE IF EXISTS "inventory_piece" CASCADE;
DROP TABLE IF EXISTS "inventory_adjustment_items" CASCADE;
DROP TABLE IF EXISTS "inventory_adjustments" CASCADE;
DROP TABLE IF EXISTS "inventory_reservations" CASCADE;
DROP TABLE IF EXISTS "inventory_transactions" CASCADE;
