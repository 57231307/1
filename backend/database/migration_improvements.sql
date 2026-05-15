-- ============================================
-- 数据库设计改进迁移脚本
-- 审计问题修复 - 2026-05-15
-- ============================================

BEGIN;

-- 1. 添加 tenant_id 到所有业务表
-- 注意：需要根据实际租户 ID 设置默认值
ALTER TABLE customers ADD COLUMN IF NOT EXISTS tenant_id INTEGER NOT NULL REFERENCES tenants(id) DEFAULT 1;
ALTER TABLE suppliers ADD COLUMN IF NOT EXISTS tenant_id INTEGER NOT NULL REFERENCES tenants(id) DEFAULT 1;
ALTER TABLE products ADD COLUMN IF NOT EXISTS tenant_id INTEGER NOT NULL REFERENCES tenants(id) DEFAULT 1;
ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS tenant_id INTEGER NOT NULL REFERENCES tenants(id) DEFAULT 1;
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS tenant_id INTEGER NOT NULL REFERENCES tenants(id) DEFAULT 1;
ALTER TABLE purchase_orders ADD COLUMN IF NOT EXISTS tenant_id INTEGER NOT NULL REFERENCES tenants(id) DEFAULT 1;
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS tenant_id INTEGER NOT NULL REFERENCES tenants(id) DEFAULT 1;
ALTER TABLE warehouse_locations ADD COLUMN IF NOT EXISTS tenant_id INTEGER NOT NULL REFERENCES tenants(id) DEFAULT 1;
ALTER TABLE product_categories ADD COLUMN IF NOT EXISTS tenant_id INTEGER NOT NULL REFERENCES tenants(id) DEFAULT 1;
ALTER TABLE product_colors ADD COLUMN IF NOT EXISTS tenant_id INTEGER NOT NULL REFERENCES tenants(id) DEFAULT 1;
ALTER TABLE customers_credit ADD COLUMN IF NOT EXISTS tenant_id INTEGER NOT NULL REFERENCES tenants(id) DEFAULT 1;
ALTER TABLE finance_invoices ADD COLUMN IF NOT EXISTS tenant_id INTEGER NOT NULL REFERENCES tenants(id) DEFAULT 1;
ALTER TABLE finance_payments ADD COLUMN IF NOT EXISTS tenant_id INTEGER NOT NULL REFERENCES tenants(id) DEFAULT 1;

-- 2. 为 tenant_id 添加索引
CREATE INDEX IF NOT EXISTS idx_customers_tenant ON customers(tenant_id);
CREATE INDEX IF NOT EXISTS idx_suppliers_tenant ON suppliers(tenant_id);
CREATE INDEX IF NOT EXISTS idx_products_tenant ON products(tenant_id);
CREATE INDEX IF NOT EXISTS idx_sales_orders_tenant ON sales_orders(tenant_id);
CREATE INDEX IF NOT EXISTS idx_sales_order_items_tenant ON sales_order_items(tenant_id);
CREATE INDEX IF NOT EXISTS idx_purchase_orders_tenant ON purchase_orders(tenant_id);
CREATE INDEX IF NOT EXISTS idx_inventory_stocks_tenant ON inventory_stocks(tenant_id);
CREATE INDEX IF NOT EXISTS idx_product_categories_tenant ON product_categories(tenant_id);
CREATE INDEX IF NOT EXISTS idx_product_colors_tenant ON product_colors(tenant_id);

-- 3. 补充审计字段
ALTER TABLE product_categories ADD COLUMN IF NOT EXISTS created_by INTEGER REFERENCES users(id);
ALTER TABLE product_colors ADD COLUMN IF NOT EXISTS created_by INTEGER REFERENCES users(id);
ALTER TABLE product_categories ADD COLUMN IF NOT EXISTS updated_by INTEGER REFERENCES users(id);
ALTER TABLE product_colors ADD COLUMN IF NOT EXISTS updated_by INTEGER REFERENCES users(id);
ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS updated_by INTEGER REFERENCES users(id);
ALTER TABLE purchase_orders ADD COLUMN IF NOT EXISTS updated_by INTEGER REFERENCES users(id);
ALTER TABLE customers ADD COLUMN IF NOT EXISTS updated_by INTEGER REFERENCES users(id);
ALTER TABLE suppliers ADD COLUMN IF NOT EXISTS updated_by INTEGER REFERENCES users(id);

-- 4. 补充软删除字段
ALTER TABLE product_categories ADD COLUMN IF NOT EXISTS is_deleted BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE product_colors ADD COLUMN IF NOT EXISTS is_deleted BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE finance_invoices ADD COLUMN IF NOT EXISTS is_deleted BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE warehouse_locations ADD COLUMN IF NOT EXISTS is_deleted BOOLEAN NOT NULL DEFAULT false;

-- 5. 补充关键外键约束
ALTER TABLE sales_order_items 
    ADD CONSTRAINT IF NOT EXISTS fk_sales_order_items_product 
    FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE RESTRICT;

ALTER TABLE purchase_order_items 
    ADD CONSTRAINT IF NOT EXISTS fk_purchase_order_items_product 
    FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE RESTRICT;

ALTER TABLE inventory_transactions 
    ADD CONSTRAINT IF NOT EXISTS fk_inventory_transactions_product 
    FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE RESTRICT;

-- 6. 创建产品 - 供应商关联表
CREATE TABLE IF NOT EXISTS product_suppliers (
    id SERIAL PRIMARY KEY,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id) ON DELETE CASCADE,
    is_primary BOOLEAN DEFAULT false,
    min_order_quantity DECIMAL(12,2),
    lead_time_days INTEGER,
    unit_price DECIMAL(12,2),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(product_id, supplier_id)
);

CREATE INDEX IF NOT EXISTS idx_product_suppliers_product ON product_suppliers(product_id);
CREATE INDEX IF NOT EXISTS idx_product_suppliers_supplier ON product_suppliers(supplier_id);

-- 7. 补充性能索引
CREATE INDEX IF NOT EXISTS idx_purchase_orders_supplier_status_date 
    ON purchase_orders(supplier_id, status, order_date DESC);

CREATE INDEX IF NOT EXISTS idx_inventory_stocks_product_warehouse_batch 
    ON inventory_stocks(product_id, warehouse_id, batch_no);

CREATE INDEX IF NOT EXISTS idx_sales_order_items_order_product 
    ON sales_order_items(order_id, product_id);

CREATE INDEX IF NOT EXISTS idx_customers_tenant_status ON customers(tenant_id, status);
CREATE INDEX IF NOT EXISTS idx_sales_orders_tenant_status ON sales_orders(tenant_id, status);
CREATE INDEX IF NOT EXISTS idx_sales_orders_tenant_order_date ON sales_orders(tenant_id, order_date DESC);

-- 8. 统一时区类型
ALTER TABLE product_categories 
    ALTER COLUMN created_at TYPE TIMESTAMPTZ,
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ;

ALTER TABLE product_colors 
    ALTER COLUMN created_at TYPE TIMESTAMPTZ,
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ;

ALTER TABLE warehouse_locations 
    ALTER COLUMN created_at TYPE TIMESTAMPTZ,
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ;

ALTER TABLE role_permissions 
    ALTER COLUMN created_at TYPE TIMESTAMPTZ;

-- 9. 调整精度不足的字段
ALTER TABLE suppliers 
    ALTER COLUMN registered_capital TYPE DECIMAL(18,2),
    ALTER COLUMN annual_revenue TYPE DECIMAL(18,2);

ALTER TABLE products 
    ALTER COLUMN standard_price TYPE DECIMAL(12,2),
    ALTER COLUMN cost_price TYPE DECIMAL(12,2);

ALTER TABLE sales_orders 
    ALTER COLUMN total_amount TYPE DECIMAL(14,2);

-- 10. 添加租户用户离开时间字段
ALTER TABLE tenant_users ADD COLUMN IF NOT EXISTS left_at TIMESTAMPTZ;

-- 11. 改进租户配置 features 字段为 JSONB
-- 注意：需要先备份数据
ALTER TABLE tenant_configs 
    ALTER COLUMN config_value TYPE JSONB USING config_value::jsonb;

-- 12. 为操作审计日志添加分区支持（示例）
-- 实际分区需要根据数据量决定
-- CREATE TABLE operation_logs_2026_01 (CHECK (created_at >= '2026-01-01' AND created_at < '2026-02-01')) INHERITS (operation_logs);

COMMIT;

-- ============================================
-- 回滚脚本（仅在需要时使用）
-- ============================================
-- BEGIN;
-- ALTER TABLE customers DROP COLUMN IF EXISTS tenant_id;
-- ALTER TABLE suppliers DROP COLUMN IF EXISTS tenant_id;
-- DROP INDEX IF EXISTS idx_customers_tenant;
-- COMMIT;
