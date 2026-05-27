-- 回滚批次7：销售管理扩展 + 物流
-- 创建时间: 2026-05-27
-- 描述: 删除销售管理扩展表和物流表

-- 删除表（按依赖关系逆序）
DROP TABLE IF EXISTS "logistics_waybills" CASCADE;
DROP TABLE IF EXISTS "sales_statistics" CASCADE;
DROP TABLE IF EXISTS "sales_prices" CASCADE;
DROP TABLE IF EXISTS "sales_return_item" CASCADE;
DROP TABLE IF EXISTS "sales_return" CASCADE;
DROP TABLE IF EXISTS "sales_order_change_history" CASCADE;
DROP TABLE IF EXISTS "sales_delivery_item" CASCADE;
DROP TABLE IF EXISTS "sales_delivery" CASCADE;
DROP TABLE IF EXISTS "sales_contracts" CASCADE;
