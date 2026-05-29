-- 回滚初始数据库架构
-- 按照创建的逆序删除表

-- BPM相关表（按依赖关系逆序）
DROP TABLE IF EXISTS "bpm_task" CASCADE;
DROP TABLE IF EXISTS "bpm_process_instance" CASCADE;
DROP TABLE IF EXISTS "bpm_process_definition" CASCADE;

-- 审计日志
DROP TABLE IF EXISTS "audit_logs" CASCADE;

-- 库存盘点明细
DROP TABLE IF EXISTS "inventory_count_items" CASCADE;
DROP TABLE IF EXISTS "inventory_counts" CASCADE;

-- 库存调拨明细
DROP TABLE IF EXISTS "inventory_transfer_items" CASCADE;
DROP TABLE IF EXISTS "inventory_transfers" CASCADE;

-- 销售订单明细
DROP TABLE IF EXISTS "sales_order_items" CASCADE;
DROP TABLE IF EXISTS "sales_orders" CASCADE;

-- 采购订单明细
DROP TABLE IF EXISTS "purchase_order_items" CASCADE;
DROP TABLE IF EXISTS "purchase_orders" CASCADE;

-- 财务收付款
DROP TABLE IF EXISTS "finance_payments" CASCADE;

-- 库存
DROP TABLE IF EXISTS "inventory_stocks" CASCADE;

-- 基础数据表
DROP TABLE IF EXISTS "customers" CASCADE;
DROP TABLE IF EXISTS "suppliers" CASCADE;
DROP TABLE IF EXISTS "warehouses" CASCADE;
DROP TABLE IF EXISTS "product_categories" CASCADE;
DROP TABLE IF EXISTS "products" CASCADE;

-- 系统管理表
DROP TABLE IF EXISTS "departments" CASCADE;
DROP TABLE IF EXISTS "roles" CASCADE;
DROP TABLE IF EXISTS "users" CASCADE;
