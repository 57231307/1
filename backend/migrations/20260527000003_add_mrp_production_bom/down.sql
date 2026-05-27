-- 回滚批次3：MRP/生产计划 + BOM
-- 创建时间: 2026-05-27
-- 描述: 删除生产管理相关表

-- 删除表（按依赖关系逆序）
DROP TABLE IF EXISTS "scheduling_result" CASCADE;
DROP TABLE IF EXISTS "mrp_results" CASCADE;
DROP TABLE IF EXISTS "production_orders" CASCADE;
DROP TABLE IF EXISTS "work_centers" CASCADE;
DROP TABLE IF EXISTS "bom_items" CASCADE;
DROP TABLE IF EXISTS "boms" CASCADE;
