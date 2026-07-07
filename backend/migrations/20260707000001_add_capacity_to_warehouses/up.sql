-- 批次 158 v11 真实接入：warehouse 表添加 capacity 列（规则 0/1/2 真实实现）
-- 原 warehouse_handler.rs CreateWarehouseRequest/UpdateWarehouseRequest 含 capacity 字段
-- 但 warehouse 表无对应列，字段被 #[allow(dead_code)] 标注
-- 现扩展 schema 接入业务，移除 allow 标注
ALTER TABLE warehouses ADD COLUMN IF NOT EXISTS capacity INTEGER;

-- 注释说明字段语义
COMMENT ON COLUMN warehouses.capacity IS '仓库容量（单位由业务约定，如托盘数/立方米）';
