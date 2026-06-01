-- 库存模块外键约束
-- 创建时间: 2026-05-09
-- 说明: 为库存相关表添加数据库级外键约束
-- 注意: 大部分外键已在 001_consolidated_schema.sql 中定义，此处只添加缺失的

-- 库存调整 → 仓库
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'inventory_adjustments') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'warehouses') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_inventory_adjustments_warehouse') THEN
            ALTER TABLE inventory_adjustments ADD CONSTRAINT fk_inventory_adjustments_warehouse FOREIGN KEY (warehouse_id) REFERENCES warehouses(id) ON DELETE RESTRICT ON UPDATE CASCADE;
        END IF;
    END IF;
END $$;
