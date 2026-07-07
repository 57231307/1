-- 回滚：移除 warehouses.capacity 列
ALTER TABLE warehouses DROP COLUMN IF EXISTS capacity;
