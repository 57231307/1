-- ========================================
-- 秉羲 ERP 系统 - 数据库迁移脚本
-- 版本：2026-03-15
-- 说明：包含所有表的完整定义和初始数据
-- ========================================

-- 启用扩展
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ========================================
-- 1. 基础数据表
-- ========================================

-- ==================== 部门表 ====================
CREATE TABLE IF NOT EXISTS departments (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    parent_id INTEGER REFERENCES departments(id),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE departments IS '部门信息表';
COMMENT ON COLUMN departments.id IS '部门 ID';
COMMENT ON COLUMN departments.name IS '部门名称';
COMMENT ON COLUMN departments.parent_id IS '父部门 ID';

CREATE INDEX idx_departments_name ON departments(name);
CREATE INDEX idx_departments_parent_id ON departments(parent_id);

-- ==================== 角色表 ====================
CREATE TABLE IF NOT EXISTS roles (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) NOT NULL UNIQUE,
    description TEXT,
    permissions JSONB,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE roles IS '角色信息表';
COMMENT ON COLUMN roles.permissions IS '权限配置（JSON 格式）';

CREATE INDEX idx_roles_name ON roles(name);

-- ==================== 用户表 ====================
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(50) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    email VARCHAR(100),
    phone VARCHAR(20),
    role_id INTEGER REFERENCES roles(id),
    department_id INTEGER REFERENCES departments(id),
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_login_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE users IS '用户信息表';
COMMENT ON COLUMN users.password_hash IS '密码哈希值';
COMMENT ON COLUMN users.is_active IS '是否激活';

CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_role_id ON users(role_id);
CREATE INDEX idx_users_department_id ON users(department_id);
CREATE INDEX idx_users_is_active ON users(is_active);

-- ==================== 角色权限关联表 ====================
CREATE TABLE IF NOT EXISTS role_permissions (
    id SERIAL PRIMARY KEY,
    role_id INTEGER NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_code VARCHAR(100) NOT NULL,
    permission_name VARCHAR(200) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(role_id, permission_code)
);

COMMENT ON TABLE role_permissions IS '角色权限关联表';

CREATE INDEX idx_role_permissions_role_id ON role_permissions(role_id);

-- ========================================
-- 2. 产品管理
-- ========================================

-- ==================== 产品类别表 ====================
CREATE TABLE IF NOT EXISTS product_categories (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    parent_id INTEGER REFERENCES product_categories(id),
    description TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE product_categories IS '产品类别表';
COMMENT ON COLUMN product_categories.parent_id IS '父类别 ID';

CREATE INDEX idx_product_categories_name ON product_categories(name);
CREATE INDEX idx_product_categories_parent_id ON product_categories(parent_id);

-- ==================== 产品表 ====================
CREATE TABLE IF NOT EXISTS products (
    id SERIAL PRIMARY KEY,
    name VARCHAR(200) NOT NULL,
    code VARCHAR(50) NOT NULL UNIQUE,
    category_id INTEGER REFERENCES product_categories(id),
    specification VARCHAR(200),
    unit VARCHAR(20) NOT NULL DEFAULT '件',
    standard_price DECIMAL(10,2),
    cost_price DECIMAL(10,2),
    description TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE products IS '产品信息表';
COMMENT ON COLUMN products.code IS '产品编码（唯一）';
COMMENT ON COLUMN products.status IS '状态：active-启用，inactive-停用';

CREATE INDEX idx_products_code ON products(code);
CREATE INDEX idx_products_category_id ON products(category_id);
CREATE INDEX idx_products_status ON products(status);

-- ========================================
-- 3. 仓库管理
-- ========================================

-- ==================== 仓库表 ====================
CREATE TABLE IF NOT EXISTS warehouses (
    id SERIAL PRIMARY KEY,
    code VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(200) NOT NULL,
    address VARCHAR(500),
    manager VARCHAR(100),
    phone VARCHAR(20),
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE warehouses IS '仓库信息表';
COMMENT ON COLUMN warehouses.code IS '仓库编码';
COMMENT ON COLUMN warehouses.status IS '状态：active-启用，inactive-停用';

CREATE INDEX idx_warehouses_code ON warehouses(code);
CREATE INDEX idx_warehouses_status ON warehouses(status);

-- ========================================
-- 4. 库存管理
-- ========================================

-- ==================== 库存表 ====================
CREATE TABLE IF NOT EXISTS inventory_stocks (
    id SERIAL PRIMARY KEY,
    product_id INTEGER NOT NULL REFERENCES products(id),
    warehouse_id INTEGER NOT NULL REFERENCES warehouses(id),
    batch_no VARCHAR(50) NOT NULL,
    color_code VARCHAR(50),
    color_name VARCHAR(100),
    quantity DECIMAL(10,2) NOT NULL DEFAULT 0,
    unit VARCHAR(20) NOT NULL,
    unit_price DECIMAL(10,2),
    total_amount DECIMAL(10,2),
    min_stock DECIMAL(10,2) DEFAULT 0,
    max_stock DECIMAL(10,2) DEFAULT 0,
    remark TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE inventory_stocks IS '库存信息表（面料批次管理）';
COMMENT ON COLUMN inventory_stocks.batch_no IS '批次号';
COMMENT ON COLUMN inventory_stocks.color_code IS '色号';
COMMENT ON COLUMN inventory_stocks.min_stock IS '最低库存预警线';
COMMENT ON COLUMN inventory_stocks.max_stock IS '最高库存预警线';

CREATE INDEX idx_inventory_stocks_product_id ON inventory_stocks(product_id);
CREATE INDEX idx_inventory_stocks_warehouse_id ON inventory_stocks(warehouse_id);
CREATE INDEX idx_inventory_stocks_batch_no ON inventory_stocks(batch_no);
CREATE INDEX idx_inventory_stocks_status ON inventory_stocks(status);

-- ==================== 库存调拨表 ====================
CREATE TABLE IF NOT EXISTS inventory_transfers (
    id SERIAL PRIMARY KEY,
    transfer_no VARCHAR(50) NOT NULL UNIQUE,
    from_warehouse_id INTEGER NOT NULL REFERENCES warehouses(id),
    to_warehouse_id INTEGER NOT NULL REFERENCES warehouses(id),
    transfer_date TIMESTAMPTZ NOT NULL,
    status VARCHAR(20) NOT NULL,
    total_quantity DECIMAL(12,2) NOT NULL,
    notes TEXT,
    created_by INTEGER,
    approved_by INTEGER,
    approved_at TIMESTAMPTZ,
    shipped_at TIMESTAMPTZ,
    received_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE inventory_transfers IS '库存调拨表';
COMMENT ON COLUMN inventory_transfers.status IS '状态：pending-待审核，approved-已审核，rejected-已驳回，shipped-已发出，completed-已完成';

CREATE INDEX idx_inventory_transfers_no ON inventory_transfers(transfer_no);
CREATE INDEX idx_inventory_transfers_status ON inventory_transfers(status);

-- ==================== 库存调拨明细表 ====================
CREATE TABLE IF NOT EXISTS inventory_transfer_items (
    id SERIAL PRIMARY KEY,
    transfer_id INTEGER NOT NULL REFERENCES inventory_transfers(id) ON DELETE CASCADE,
    product_id INTEGER NOT NULL REFERENCES products(id),
    quantity DECIMAL(12,2) NOT NULL,
    shipped_quantity DECIMAL(12,2) NOT NULL,
    received_quantity DECIMAL(12,2) NOT NULL,
    unit_cost DECIMAL(12,2),
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE inventory_transfer_items IS '库存调拨明细表';

CREATE INDEX idx_inventory_transfer_items_transfer_id ON inventory_transfer_items(transfer_id);
CREATE INDEX idx_inventory_transfer_items_product_id ON inventory_transfer_items(product_id);

-- ==================== 库存盘点表 ====================
CREATE TABLE IF NOT EXISTS inventory_counts (
    id SERIAL PRIMARY KEY,
    count_no VARCHAR(50) NOT NULL UNIQUE,
    warehouse_id INTEGER NOT NULL REFERENCES warehouses(id),
    count_date TIMESTAMPTZ NOT NULL,
    status VARCHAR(20) NOT NULL,
    total_items INTEGER NOT NULL DEFAULT 0,
    counted_items INTEGER NOT NULL DEFAULT 0,
    variance_items INTEGER NOT NULL DEFAULT 0,
    notes TEXT,
    created_by INTEGER,
    approved_by INTEGER,
    approved_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE inventory_counts IS '库存盘点表';
COMMENT ON COLUMN inventory_counts.status IS '状态：pending-待审核，approved-已审核，rejected-已驳回，completed-已完成';

CREATE INDEX idx_inventory_counts_no ON inventory_counts(count_no);
CREATE INDEX idx_inventory_counts_status ON inventory_counts(status);

-- ==================== 库存盘点明细表 ====================
CREATE TABLE IF NOT EXISTS inventory_count_items (
    id SERIAL PRIMARY KEY,
    count_id INTEGER NOT NULL REFERENCES inventory_counts(id) ON DELETE CASCADE,
    product_id INTEGER NOT NULL REFERENCES products(id),
    bin_location VARCHAR(50),
    quantity_book DECIMAL(12,2) NOT NULL,
    quantity_actual DECIMAL(12,2) NOT NULL,
    quantity_variance DECIMAL(12,2) NOT NULL,
    unit_cost DECIMAL(12,2),
    variance_amount DECIMAL(12,2),
    notes TEXT,
    counted_by INTEGER,
    counted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE inventory_count_items IS '库存盘点明细表';

CREATE INDEX idx_inventory_count_items_count_id ON inventory_count_items(count_id);
CREATE INDEX idx_inventory_count_items_product_id ON inventory_count_items(product_id);

-- ==================== 库存调整表 ====================
CREATE TABLE IF NOT EXISTS inventory_adjustments (
    id SERIAL PRIMARY KEY,
    adjustment_no VARCHAR(50) NOT NULL UNIQUE,
    warehouse_id INTEGER NOT NULL,
    adjustment_date TIMESTAMPTZ NOT NULL,
    adjustment_type VARCHAR(20) NOT NULL,
    reason_type VARCHAR(20) NOT NULL,
    reason_description TEXT,
    total_quantity DECIMAL(12,2) NOT NULL,
    notes TEXT,
    created_by INTEGER,
    approved_by INTEGER,
    approved_at TIMESTAMPTZ,
    status VARCHAR(20) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE inventory_adjustments IS '库存调整表';
COMMENT ON COLUMN inventory_adjustments.adjustment_type IS '调整类型：increase-增加，decrease-减少';
COMMENT ON COLUMN inventory_adjustments.reason_type IS '调整原因：damage-损坏，sample-样品，correction-修正，other-其他';
COMMENT ON COLUMN inventory_adjustments.status IS '状态：pending-待审核，approved-已审核，rejected-已驳回';

CREATE INDEX idx_inventory_adjustments_no ON inventory_adjustments(adjustment_no);
CREATE INDEX idx_inventory_adjustments_status ON inventory_adjustments(status);
CREATE INDEX idx_inventory_adjustments_warehouse_id ON inventory_adjustments(warehouse_id);

-- ==================== 库存调整明细表 ====================
CREATE TABLE IF NOT EXISTS inventory_adjustment_items (
    id SERIAL PRIMARY KEY,
    adjustment_id INTEGER NOT NULL REFERENCES inventory_adjustments(id) ON DELETE CASCADE,
    stock_id INTEGER NOT NULL REFERENCES inventory_stocks(id),
    quantity DECIMAL(12,2) NOT NULL,
    quantity_before DECIMAL(12,2) NOT NULL,
    quantity_after DECIMAL(12,2) NOT NULL,
    unit_cost DECIMAL(12,2),
    amount DECIMAL(12,2),
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE inventory_adjustment_items IS '库存调整明细表';
COMMENT ON COLUMN inventory_adjustment_items.quantity_before IS '调整前数量';
COMMENT ON COLUMN inventory_adjustment_items.quantity_after IS '调整后数量';

CREATE INDEX idx_inventory_adjustment_items_adjustment_id ON inventory_adjustment_items(adjustment_id);
CREATE INDEX idx_inventory_adjustment_items_stock_id ON inventory_adjustment_items(stock_id);

-- ========================================
-- 5. 销售管理
-- ========================================

-- ==================== 销售订单表 ====================
CREATE TABLE IF NOT EXISTS sales_orders (
    id SERIAL PRIMARY KEY,
    order_no VARCHAR(50) NOT NULL UNIQUE,
    customer_name VARCHAR(200) NOT NULL,
    order_date TIMESTAMPTZ NOT NULL,
    status VARCHAR(20) NOT NULL,
    total_amount DECIMAL(12,2) NOT NULL,
    notes TEXT,
    created_by INTEGER,
    approved_by INTEGER,
    approved_at TIMESTAMPTZ,
    shipped_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE sales_orders IS '销售订单表';
COMMENT ON COLUMN sales_orders.status IS '状态：pending-待审核，approved-已审核，shipped-已发货，completed-已完成，cancelled-已取消';

CREATE INDEX idx_sales_orders_no ON sales_orders(order_no);
CREATE INDEX idx_sales_orders_status ON sales_orders(status);

-- ==================== 销售订单明细表 ====================
CREATE TABLE IF NOT EXISTS sales_order_items (
    id SERIAL PRIMARY KEY,
    order_id INTEGER NOT NULL REFERENCES sales_orders(id) ON DELETE CASCADE,
    product_id INTEGER NOT NULL REFERENCES products(id),
    quantity DECIMAL(12,2) NOT NULL,
    unit_price DECIMAL(12,2) NOT NULL,
    total_price DECIMAL(12,2) NOT NULL,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE sales_order_items IS '销售订单明细表';

CREATE INDEX idx_sales_order_items_order_id ON sales_order_items(order_id);
CREATE INDEX idx_sales_order_items_product_id ON sales_order_items(product_id);

-- ========================================
-- 6. 财务管理
-- ========================================

-- ==================== 收款单表 ====================
CREATE TABLE IF NOT EXISTS finance_payments (
    id SERIAL PRIMARY KEY,
    payment_no VARCHAR(50) NOT NULL UNIQUE,
    invoice_id INTEGER,
    payment_date TIMESTAMPTZ NOT NULL,
    amount DECIMAL(12,2) NOT NULL,
    payment_method VARCHAR(20) NOT NULL,
    status VARCHAR(20) NOT NULL,
    notes TEXT,
    created_by INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE finance_payments IS '收款单表';
COMMENT ON COLUMN finance_payments.payment_method IS '支付方式：bank_transfer-银行转账，cash-现金，check-支票，other-其他';
COMMENT ON COLUMN finance_payments.status IS '状态：pending-待处理，completed-已完成，cancelled-已取消';

CREATE INDEX idx_finance_payments_no ON finance_payments(payment_no);
CREATE INDEX idx_finance_payments_status ON finance_payments(status);

-- ==================== 发票表 ====================
CREATE TABLE IF NOT EXISTS finance_invoices (
    id SERIAL PRIMARY KEY,
    invoice_no VARCHAR(50) NOT NULL UNIQUE,
    order_id INTEGER,
    invoice_date TIMESTAMPTZ NOT NULL,
    amount DECIMAL(12,2) NOT NULL,
    tax_amount DECIMAL(12,2),
    total_amount DECIMAL(12,2) NOT NULL,
    status VARCHAR(20) NOT NULL,
    paid_date TIMESTAMPTZ,
    payment_method VARCHAR(20),
    notes TEXT,
    created_by INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE finance_invoices IS '发票表';
COMMENT ON COLUMN finance_invoices.status IS '状态：draft-草稿，approved-已审核，verified-已核销，cancelled-已作废';

CREATE INDEX idx_finance_invoices_no ON finance_invoices(invoice_no);
CREATE INDEX idx_finance_invoices_status ON finance_invoices(status);

-- ========================================
-- 7. 系统管理
-- ========================================

-- ==================== 操作日志表 ====================
CREATE TABLE IF NOT EXISTS operation_logs (
    id SERIAL PRIMARY KEY,
    user_id INTEGER,
    username VARCHAR(50),
    module VARCHAR(50) NOT NULL,
    action VARCHAR(20) NOT NULL,
    description TEXT,
    request_method VARCHAR(10),
    request_uri VARCHAR(200),
    request_ip VARCHAR(50),
    user_agent TEXT,
    status VARCHAR(20) NOT NULL,
    error_message TEXT,
    duration_ms BIGINT,
    extra_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE operation_logs IS '操作日志表';
COMMENT ON COLUMN operation_logs.module IS '操作模块：user-用户，role-角色，product-产品，inventory-库存等';
COMMENT ON COLUMN operation_logs.action IS '操作类型：create-创建，update-更新，delete-删除，approve-审核等';
COMMENT ON COLUMN operation_logs.status IS '状态：success-成功，failure-失败';

CREATE INDEX idx_operation_logs_user_id ON operation_logs(user_id);
CREATE INDEX idx_operation_logs_module ON operation_logs(module);
CREATE INDEX idx_operation_logs_action ON operation_logs(action);
CREATE INDEX idx_operation_logs_created_at ON operation_logs(created_at);
CREATE INDEX idx_operation_logs_status ON operation_logs(status);

-- ========================================
-- 8. 初始数据
-- ========================================

-- 插入默认角色
INSERT INTO roles (name, description, permissions) VALUES
('admin', '系统管理员', '["*"]'::jsonb),
('manager', '部门经理', '["user:view", "product:*", "inventory:*", "sales:*"]'::jsonb),
('operator', '操作员', '["product:view", "inventory:view", "sales:view"]'::jsonb)
ON CONFLICT (name) DO NOTHING;

-- 插入默认部门
INSERT INTO departments (name, description) VALUES
('总经办', '公司管理层'),
('财务部', '财务管理'),
('销售部', '销售业务'),
('仓储部', '库存管理'),
('生产部', '生产管理')
ON CONFLICT (name) DO NOTHING;

-- 插入默认仓库
INSERT INTO warehouses (code, name, status) VALUES
('WH001', '主仓库', 'active'),
('WH002', '成品仓库', 'active'),
('WH003', '原料仓库', 'active')
ON CONFLICT (code) DO NOTHING;

-- 插入默认管理员用户（密码：admin123，使用 bcrypt 加密）
-- 注意：实际密码哈希值需要在应用层生成
INSERT INTO users (username, password_hash, email, role_id, department_id, is_active)
SELECT 'admin', '$2b$10$N9qo8uLOickgx2ZMRZoMyeIjZAgcfl7p92ldGxad68LJZdL17lhWy', 'admin@example.com', r.id, d.id, true
FROM roles r, departments d
WHERE r.name = 'admin' AND d.name = '总经办'
ON CONFLICT (username) DO NOTHING;

-- ========================================
-- 9. 视图和函数（可选）
-- ========================================

-- 创建库存预警视图
CREATE OR REPLACE VIEW v_low_stock_alerts AS
SELECT 
    s.id,
    s.batch_no,
    p.name AS product_name,
    p.code AS product_code,
    w.name AS warehouse_name,
    s.quantity,
    s.min_stock,
    s.max_stock,
    CASE 
        WHEN s.quantity < s.min_stock THEN 'low'
        WHEN s.quantity > s.max_stock THEN 'high'
        ELSE 'normal'
    END AS stock_status
FROM inventory_stocks s
JOIN products p ON s.product_id = p.id
JOIN warehouses w ON s.warehouse_id = w.id
WHERE s.quantity < s.min_stock OR s.quantity > s.max_stock;

-- ========================================
-- 迁移完成
-- ========================================
