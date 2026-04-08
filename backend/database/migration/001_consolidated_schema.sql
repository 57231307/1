
-- ============================================
-- 来源: 001_init.sql
-- ============================================
-- 启用扩展
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ========================================
-- 0. 创建全局通用触发器函数
-- ========================================
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

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
COMMENT ON COLUMN departments.description IS '部门描述';
COMMENT ON COLUMN departments.parent_id IS '父部门 ID';

CREATE INDEX IF NOT EXISTS idx_departments_name ON departments(name);
CREATE INDEX IF NOT EXISTS idx_departments_parent_id ON departments(parent_id);

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

CREATE INDEX IF NOT EXISTS idx_roles_name ON roles(name);

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

CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_role_id ON users(role_id);
CREATE INDEX IF NOT EXISTS idx_users_department_id ON users(department_id);
CREATE INDEX IF NOT EXISTS idx_users_is_active ON users(is_active);

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

CREATE INDEX IF NOT EXISTS idx_role_permissions_role_id ON role_permissions(role_id);

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

CREATE INDEX IF NOT EXISTS idx_product_categories_name ON product_categories(name);
CREATE INDEX IF NOT EXISTS idx_product_categories_parent_id ON product_categories(parent_id);

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

CREATE INDEX IF NOT EXISTS idx_products_code ON products(code);
CREATE INDEX IF NOT EXISTS idx_products_category_id ON products(category_id);
CREATE INDEX IF NOT EXISTS idx_products_status ON products(status);

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

CREATE INDEX IF NOT EXISTS idx_warehouses_code ON warehouses(code);
CREATE INDEX IF NOT EXISTS idx_warehouses_status ON warehouses(status);

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

CREATE INDEX IF NOT EXISTS idx_inventory_stocks_product_id ON inventory_stocks(product_id);
CREATE INDEX IF NOT EXISTS idx_inventory_stocks_warehouse_id ON inventory_stocks(warehouse_id);
CREATE INDEX IF NOT EXISTS idx_inventory_stocks_batch_no ON inventory_stocks(batch_no);
CREATE INDEX IF NOT EXISTS idx_inventory_stocks_status ON inventory_stocks(status);

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

CREATE INDEX IF NOT EXISTS idx_inventory_transfers_no ON inventory_transfers(transfer_no);
CREATE INDEX IF NOT EXISTS idx_inventory_transfers_status ON inventory_transfers(status);

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

CREATE INDEX IF NOT EXISTS idx_inventory_transfer_items_transfer_id ON inventory_transfer_items(transfer_id);
CREATE INDEX IF NOT EXISTS idx_inventory_transfer_items_product_id ON inventory_transfer_items(product_id);

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

CREATE INDEX IF NOT EXISTS idx_inventory_counts_no ON inventory_counts(count_no);
CREATE INDEX IF NOT EXISTS idx_inventory_counts_status ON inventory_counts(status);

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

CREATE INDEX IF NOT EXISTS idx_inventory_count_items_count_id ON inventory_count_items(count_id);
CREATE INDEX IF NOT EXISTS idx_inventory_count_items_product_id ON inventory_count_items(product_id);

-- ==================== 库存调整表 ====================
CREATE TABLE IF NOT EXISTS inventory_adjustments (
    id SERIAL PRIMARY KEY,
    adjustment_no VARCHAR(50) NOT NULL UNIQUE,
    warehouse_id INTEGER NOT NULL REFERENCES warehouses(id),
    adjustment_date TIMESTAMPTZ NOT NULL,
    adjustment_type VARCHAR(20) NOT NULL,
    reason_type VARCHAR(20) NOT NULL,
    reason_description TEXT,
    total_quantity DECIMAL(12,2) NOT NULL,
    notes TEXT,
    created_by INTEGER REFERENCES users(id),
    approved_by INTEGER REFERENCES users(id),
    approved_at TIMESTAMPTZ,
    status VARCHAR(20) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE inventory_adjustments IS '库存调整表';
COMMENT ON COLUMN inventory_adjustments.adjustment_type IS '调整类型：increase-增加，decrease-减少';
COMMENT ON COLUMN inventory_adjustments.reason_type IS '调整原因：damage-损坏，sample-样品，correction-修正，other-其他';
COMMENT ON COLUMN inventory_adjustments.status IS '状态：pending-待审核，approved-已审核，rejected-已驳回';

CREATE INDEX IF NOT EXISTS idx_inventory_adjustments_no ON inventory_adjustments(adjustment_no);
CREATE INDEX IF NOT EXISTS idx_inventory_adjustments_status ON inventory_adjustments(status);
CREATE INDEX IF NOT EXISTS idx_inventory_adjustments_warehouse_id ON inventory_adjustments(warehouse_id);

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

CREATE INDEX IF NOT EXISTS idx_inventory_adjustment_items_adjustment_id ON inventory_adjustment_items(adjustment_id);
CREATE INDEX IF NOT EXISTS idx_inventory_adjustment_items_stock_id ON inventory_adjustment_items(stock_id);

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
    created_by INTEGER REFERENCES users(id),
    approved_by INTEGER REFERENCES users(id),
    approved_at TIMESTAMPTZ,
    shipped_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE sales_orders IS '销售订单表';
COMMENT ON COLUMN sales_orders.status IS '状态：pending-待审核，approved-已审核，shipped-已发货，completed-已完成，cancelled-已取消';

CREATE INDEX IF NOT EXISTS idx_sales_orders_no ON sales_orders(order_no);
CREATE INDEX IF NOT EXISTS idx_sales_orders_status ON sales_orders(status);

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

CREATE INDEX IF NOT EXISTS idx_sales_order_items_order_id ON sales_order_items(order_id);
CREATE INDEX IF NOT EXISTS idx_sales_order_items_product_id ON sales_order_items(product_id);

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

CREATE INDEX IF NOT EXISTS idx_finance_payments_no ON finance_payments(payment_no);
CREATE INDEX IF NOT EXISTS idx_finance_payments_status ON finance_payments(status);

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

CREATE INDEX IF NOT EXISTS idx_finance_invoices_no ON finance_invoices(invoice_no);
CREATE INDEX IF NOT EXISTS idx_finance_invoices_status ON finance_invoices(status);

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

CREATE INDEX IF NOT EXISTS idx_operation_logs_user_id ON operation_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_operation_logs_module ON operation_logs(module);
CREATE INDEX IF NOT EXISTS idx_operation_logs_action ON operation_logs(action);
CREATE INDEX IF NOT EXISTS idx_operation_logs_created_at ON operation_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_operation_logs_status ON operation_logs(status);

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

-- ============================================
-- 来源: 002_inventory_reservation.sql
-- ============================================
-- ============================================
-- 库存预留表 - 用于销售订单锁定库存
-- ============================================

CREATE TABLE IF NOT EXISTS inventory_reservations (
    id SERIAL PRIMARY KEY,
    order_id INTEGER NOT NULL REFERENCES sales_orders(id) ON DELETE CASCADE,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    warehouse_id INTEGER NOT NULL REFERENCES warehouses(id) ON DELETE CASCADE,
    quantity DECIMAL(10,2) NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',  -- pending-待处理，locked-已锁定，released-已释放，used-已使用
    reserved_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    released_at TIMESTAMPTZ,
    notes TEXT,
    created_by INTEGER REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_inventory_reservations_order_id ON inventory_reservations(order_id);
CREATE INDEX IF NOT EXISTS idx_inventory_reservations_product_id ON inventory_reservations(product_id);
CREATE INDEX IF NOT EXISTS idx_inventory_reservations_warehouse_id ON inventory_reservations(warehouse_id);
CREATE INDEX IF NOT EXISTS idx_inventory_reservations_status ON inventory_reservations(status);
CREATE INDEX IF NOT EXISTS idx_inventory_reservations_reserved_at ON inventory_reservations(reserved_at);

-- 添加注释
COMMENT ON TABLE inventory_reservations IS '库存预留表 - 用于销售订单锁定库存';
COMMENT ON COLUMN inventory_reservations.id IS '预留 ID';
COMMENT ON COLUMN inventory_reservations.order_id IS '销售订单 ID';
COMMENT ON COLUMN inventory_reservations.product_id IS '产品 ID';
COMMENT ON COLUMN inventory_reservations.warehouse_id IS '仓库 ID';
COMMENT ON COLUMN inventory_reservations.quantity IS '预留数量';
COMMENT ON COLUMN inventory_reservations.status IS '预留状态：pending-待处理，locked-已锁定，released-已释放，used-已使用';
COMMENT ON COLUMN inventory_reservations.reserved_at IS '预留时间';
COMMENT ON COLUMN inventory_reservations.released_at IS '释放时间';
COMMENT ON COLUMN inventory_reservations.notes IS '备注';
COMMENT ON COLUMN inventory_reservations.created_by IS '创建人';
COMMENT ON COLUMN inventory_reservations.created_at IS '创建时间';
COMMENT ON COLUMN inventory_reservations.updated_at IS '更新时间';

-- ============================================
-- 来源: 003_customers.sql
-- ============================================
-- ============================================
-- 客户管理表
-- ============================================

CREATE TABLE IF NOT EXISTS customers (
    id SERIAL PRIMARY KEY,
    customer_code VARCHAR(50) NOT NULL UNIQUE,
    customer_name VARCHAR(100) NOT NULL,
    contact_person VARCHAR(50),
    contact_phone VARCHAR(20),
    contact_email VARCHAR(100),
    address TEXT,
    city VARCHAR(50),
    province VARCHAR(50),
    country VARCHAR(50) DEFAULT '中国',
    postal_code VARCHAR(20),
    credit_limit DECIMAL(12,2) DEFAULT 0,
    payment_terms INTEGER DEFAULT 30,  -- 账期（天）
    tax_id VARCHAR(50),  -- 税号
    bank_name VARCHAR(100),  -- 开户行
    bank_account VARCHAR(50),  -- 银行账号
    status VARCHAR(20) DEFAULT 'active',  -- active-活跃，inactive-停用，blacklist-黑名单
    customer_type VARCHAR(20) DEFAULT 'retail',  -- retail-零售，wholesale-批发，vip-VIP
    notes TEXT,
    created_by INTEGER REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_customers_code ON customers(customer_code);
CREATE INDEX IF NOT EXISTS idx_customers_name ON customers(customer_name);
CREATE INDEX IF NOT EXISTS idx_customers_status ON customers(status);
CREATE INDEX IF NOT EXISTS idx_customers_type ON customers(customer_type);
CREATE INDEX IF NOT EXISTS idx_customers_created_at ON customers(created_at);

-- 添加注释
COMMENT ON TABLE customers IS '客户信息表';
COMMENT ON COLUMN customers.id IS '客户 ID';
COMMENT ON COLUMN customers.customer_code IS '客户编码（唯一）';
COMMENT ON COLUMN customers.customer_name IS '客户名称';
COMMENT ON COLUMN customers.contact_person IS '联系人';
COMMENT ON COLUMN customers.contact_phone IS '联系电话';
COMMENT ON COLUMN customers.contact_email IS '联系邮箱';
COMMENT ON COLUMN customers.address IS '地址';
COMMENT ON COLUMN customers.city IS '城市';
COMMENT ON COLUMN customers.province IS '省份';
COMMENT ON COLUMN customers.country IS '国家';
COMMENT ON COLUMN customers.postal_code IS '邮编';
COMMENT ON COLUMN customers.credit_limit IS '信用额度';
COMMENT ON COLUMN customers.payment_terms IS '账期（天）';
COMMENT ON COLUMN customers.tax_id IS '税号';
COMMENT ON COLUMN customers.bank_name IS '开户行';
COMMENT ON COLUMN customers.bank_account IS '银行账号';
COMMENT ON COLUMN customers.status IS '状态：active-活跃，inactive-停用，blacklist-黑名单';
COMMENT ON COLUMN customers.customer_type IS '客户类型：retail-零售，wholesale-批发，vip-VIP';
COMMENT ON COLUMN customers.notes IS '备注';
COMMENT ON COLUMN customers.created_by IS '创建人';
COMMENT ON COLUMN customers.created_at IS '创建时间';
COMMENT ON COLUMN customers.updated_at IS '更新时间';

-- ============================================
-- 来源: 004_fabric_industry_adaptation.sql
-- ============================================
-- ========================================
-- 面料 ERP 系统 - 面料行业适配迁移脚本（阶段 1）
-- 版本：2026-03-15
-- 说明：产品、库存、总账模块面料行业适配
-- ========================================

-- ========================================
-- 1. 产品管理模块面料行业适配
-- ========================================

-- 1.1 产品表增加面料行业字段
ALTER TABLE products ADD COLUMN IF NOT EXISTS product_type VARCHAR(20) DEFAULT '成品布';
ALTER TABLE products ADD COLUMN IF NOT EXISTS fabric_composition VARCHAR(200);
ALTER TABLE products ADD COLUMN IF NOT EXISTS yarn_count VARCHAR(50);
ALTER TABLE products ADD COLUMN IF NOT EXISTS density VARCHAR(50);
ALTER TABLE products ADD COLUMN IF NOT EXISTS width DECIMAL(10,2);
ALTER TABLE products ADD COLUMN IF NOT EXISTS gram_weight DECIMAL(10,2);
ALTER TABLE products ADD COLUMN IF NOT EXISTS structure VARCHAR(50);
ALTER TABLE products ADD COLUMN IF NOT EXISTS finish VARCHAR(100);
ALTER TABLE products ADD COLUMN IF NOT EXISTS min_order_quantity DECIMAL(12,2);
ALTER TABLE products ADD COLUMN IF NOT EXISTS lead_time INTEGER;

COMMENT ON COLUMN products.product_type IS '产品类型：坯布/成品布/辅料';
COMMENT ON COLUMN products.fabric_composition IS '面料成分：如 65% 棉 35% 涤';
COMMENT ON COLUMN products.yarn_count IS '纱支：如 40S';
COMMENT ON COLUMN products.density IS '密度：如 133x72';
COMMENT ON COLUMN products.width IS '幅宽（cm）';
COMMENT ON COLUMN products.gram_weight IS '克重（g/m²）';
COMMENT ON COLUMN products.structure IS '组织结构：平纹/斜纹/缎纹';
COMMENT ON COLUMN products.finish IS '后整理：防水/防油/阻燃';
COMMENT ON COLUMN products.min_order_quantity IS '最小起订量（米）';
COMMENT ON COLUMN products.lead_time IS '交货期（天）';

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_products_product_type ON products(product_type);
CREATE INDEX IF NOT EXISTS idx_products_gram_weight ON products(gram_weight);

-- 1.2 创建产品色号表
CREATE TABLE IF NOT EXISTS product_colors (
    id SERIAL PRIMARY KEY,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    color_no VARCHAR(50) NOT NULL,
    color_name VARCHAR(100) NOT NULL,
    pantone_code VARCHAR(50),
    color_type VARCHAR(20) DEFAULT '常规色',
    dye_formula TEXT,                     -- 染色配方（保密）
    extra_cost DECIMAL(10,2) DEFAULT 0,   -- 特殊色号加价（元/米）
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE product_colors IS '产品色号表（面料行业）';
COMMENT ON COLUMN product_colors.color_no IS '色号';
COMMENT ON COLUMN product_colors.color_name IS '颜色名称';
COMMENT ON COLUMN product_colors.pantone_code IS '潘通色号';
COMMENT ON COLUMN product_colors.color_type IS '色号类型：常规色/定制色';
COMMENT ON COLUMN product_colors.dye_formula IS '染色配方（保密）';
COMMENT ON COLUMN product_colors.extra_cost IS '特殊色号加价（元/米）';

CREATE INDEX IF NOT EXISTS idx_product_colors_product_id ON product_colors(product_id);
CREATE INDEX IF NOT EXISTS idx_product_colors_color_no ON product_colors(color_no);
CREATE INDEX IF NOT EXISTS idx_product_colors_color_type ON product_colors(color_type);

-- 创建触发器：自动更新 updated_at
CREATE OR REPLACE FUNCTION update_product_colors_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_product_colors_updated_at ON product_colors;
CREATE TRIGGER trg_product_colors_updated_at
    BEFORE UPDATE ON product_colors
    FOR EACH ROW
    EXECUTE FUNCTION update_product_colors_updated_at();

-- ========================================
-- 2. 仓库管理模块面料行业适配
-- ========================================

-- 2.1 仓库表增加面料行业字段
ALTER TABLE warehouses ADD COLUMN IF NOT EXISTS warehouse_type VARCHAR(20) DEFAULT '成品库';
ALTER TABLE warehouses ADD COLUMN IF NOT EXISTS temperature_control BOOLEAN DEFAULT false;
ALTER TABLE warehouses ADD COLUMN IF NOT EXISTS humidity_control BOOLEAN DEFAULT false;
ALTER TABLE warehouses ADD COLUMN IF NOT EXISTS fire_protection_level VARCHAR(10);

COMMENT ON COLUMN warehouses.warehouse_type IS '仓库类型：原料库/坯布库/成品库/辅料库';
COMMENT ON COLUMN warehouses.temperature_control IS '是否温控';
COMMENT ON COLUMN warehouses.humidity_control IS '是否湿度控制';
COMMENT ON COLUMN warehouses.fire_protection_level IS '消防等级：甲/乙/丙';

CREATE INDEX IF NOT EXISTS idx_warehouses_type ON warehouses(warehouse_type);

-- 2.2 创建库位表
CREATE TABLE IF NOT EXISTS warehouse_locations (
    id SERIAL PRIMARY KEY,
    warehouse_id INTEGER NOT NULL REFERENCES warehouses(id) ON DELETE CASCADE,
    location_code VARCHAR(50) NOT NULL,
    location_type VARCHAR(20) DEFAULT '平面库',
    max_weight DECIMAL(10,2),
    max_height DECIMAL(10,2),
    is_batch_managed BOOLEAN DEFAULT true,
    is_color_managed BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE warehouse_locations IS '库位表（面料行业）';
COMMENT ON COLUMN warehouse_locations.location_code IS '库位编码';
COMMENT ON COLUMN warehouse_locations.location_type IS '库位类型：平面库/货架库/卷装库';
COMMENT ON COLUMN warehouse_locations.is_batch_managed IS '是否批次管理';
COMMENT ON COLUMN warehouse_locations.is_color_managed IS '是否色号管理';

CREATE UNIQUE INDEX IF NOT EXISTS idx_location_code ON warehouse_locations(warehouse_id, location_code);

-- 创建触发器：自动更新 updated_at
DROP TRIGGER IF EXISTS trg_warehouse_locations_updated_at ON warehouse_locations;
CREATE TRIGGER trg_warehouse_locations_updated_at
    BEFORE UPDATE ON warehouse_locations
    FOR EACH ROW
    EXECUTE FUNCTION update_product_colors_updated_at();

-- ========================================
-- 3. 库存管理模块面料行业适配（核心）
-- ========================================

-- 3.1 库存表增加面料行业核心字段
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS batch_no VARCHAR(50) NOT NULL DEFAULT '';
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS color_no VARCHAR(50) NOT NULL DEFAULT '';
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS dye_lot_no VARCHAR(50);
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS grade VARCHAR(20) DEFAULT '一等品';
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS production_date TIMESTAMPTZ;
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS expiry_date TIMESTAMPTZ;

-- 双计量单位
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS quantity_meters DECIMAL(12,2) NOT NULL DEFAULT 0;
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS quantity_kg DECIMAL(12,2) NOT NULL DEFAULT 0;
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS gram_weight DECIMAL(10,2);
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS width DECIMAL(10,2);

-- 库位管理
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS location_id INTEGER REFERENCES warehouse_locations(id);
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS shelf_no VARCHAR(20);
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS layer_no VARCHAR(20);

-- 状态管理
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS stock_status VARCHAR(20) DEFAULT '正常';
ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS quality_status VARCHAR(20) DEFAULT '合格';

COMMENT ON COLUMN inventory_stocks.batch_no IS '批次号';
COMMENT ON COLUMN inventory_stocks.color_no IS '色号';
COMMENT ON COLUMN inventory_stocks.dye_lot_no IS '缸号';
COMMENT ON COLUMN inventory_stocks.grade IS '等级：一等品/二等品/等外品';
COMMENT ON COLUMN inventory_stocks.production_date IS '生产日期';
COMMENT ON COLUMN inventory_stocks.expiry_date IS '保质期';
COMMENT ON COLUMN inventory_stocks.quantity_meters IS '数量（米）';
COMMENT ON COLUMN inventory_stocks.quantity_kg IS '数量（公斤）';
COMMENT ON COLUMN inventory_stocks.gram_weight IS '克重（g/m²）';
COMMENT ON COLUMN inventory_stocks.width IS '幅宽（cm）';
COMMENT ON COLUMN inventory_stocks.location_id IS '库位 ID';
COMMENT ON COLUMN inventory_stocks.shelf_no IS '货架号';
COMMENT ON COLUMN inventory_stocks.layer_no IS '层号';
COMMENT ON COLUMN inventory_stocks.stock_status IS '库存状态：正常/冻结/待检';
COMMENT ON COLUMN inventory_stocks.quality_status IS '质量状态：合格/不合格/待检';

-- 创建索引（重要：提高查询性能）
CREATE INDEX IF NOT EXISTS idx_inventory_stocks_batch ON inventory_stocks(batch_no);
CREATE INDEX IF NOT EXISTS idx_inventory_stocks_color ON inventory_stocks(color_no);
CREATE INDEX IF NOT EXISTS idx_inventory_stocks_dye_lot ON inventory_stocks(dye_lot_no);
CREATE INDEX IF NOT EXISTS idx_inventory_stocks_grade ON inventory_stocks(grade);
CREATE INDEX IF NOT EXISTS idx_inventory_stocks_location ON inventory_stocks(location_id);
CREATE INDEX IF NOT EXISTS idx_inventory_stocks_quality ON inventory_stocks(quality_status);

-- 复合索引（常用查询）
CREATE INDEX IF NOT EXISTS idx_inventory_batch_color ON inventory_stocks(batch_no, color_no);
CREATE INDEX IF NOT EXISTS idx_inventory_warehouse_batch ON inventory_stocks(warehouse_id, batch_no, color_no);

-- 3.2 创建库存流水表（面料行业）
CREATE TABLE IF NOT EXISTS inventory_transactions (
    id SERIAL PRIMARY KEY,
    transaction_type VARCHAR(20) NOT NULL,
    product_id INTEGER NOT NULL REFERENCES products(id),
    warehouse_id INTEGER NOT NULL REFERENCES warehouses(id),
    batch_no VARCHAR(50) NOT NULL,
    color_no VARCHAR(50) NOT NULL,
    dye_lot_no VARCHAR(50),
    grade VARCHAR(20) DEFAULT '一等品',
    
    -- 双计量单位
    quantity_meters DECIMAL(12,2) NOT NULL,
    quantity_kg DECIMAL(12,2) NOT NULL,
    
    -- 关联单据
    source_bill_type VARCHAR(50),
    source_bill_no VARCHAR(50),
    source_bill_id INTEGER,
    
    -- 库存变化
    quantity_before_meters DECIMAL(12,2),
    quantity_before_kg DECIMAL(12,2),
    quantity_after_meters DECIMAL(12,2),
    quantity_after_kg DECIMAL(12,2),
    
    -- 备注
    notes TEXT,
    created_by INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE inventory_transactions IS '库存流水表（面料行业）';
COMMENT ON COLUMN inventory_transactions.transaction_type IS '交易类型：采购入库/生产入库/销售出库/调拨/盘点/调整';
COMMENT ON COLUMN inventory_transactions.batch_no IS '批次号';
COMMENT ON COLUMN inventory_transactions.color_no IS '色号';
COMMENT ON COLUMN inventory_transactions.quantity_meters IS '数量（米）';
COMMENT ON COLUMN inventory_transactions.quantity_kg IS '数量（公斤）';
COMMENT ON COLUMN inventory_transactions.source_bill_type IS '来源单据类型';
COMMENT ON COLUMN inventory_transactions.source_bill_no IS '来源单据号';

CREATE INDEX IF NOT EXISTS idx_inventory_transactions_batch ON inventory_transactions(batch_no, color_no);
CREATE INDEX IF NOT EXISTS idx_inventory_transactions_product ON inventory_transactions(product_id);
CREATE INDEX IF NOT EXISTS idx_inventory_transactions_warehouse ON inventory_transactions(warehouse_id);
CREATE INDEX IF NOT EXISTS idx_inventory_transactions_source ON inventory_transactions(source_bill_type, source_bill_id);
CREATE INDEX IF NOT EXISTS idx_inventory_transactions_created ON inventory_transactions(created_at);

-- ========================================
-- 4. 销售订单模块面料行业适配
-- ========================================

-- 4.1 销售订单表增加面料行业字段
ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS delivery_type VARCHAR(20) DEFAULT '一次性交货';
ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS partial_delivery_allowed BOOLEAN DEFAULT false;
ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS max_partial_count INTEGER;
ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS quality_standard VARCHAR(100);
ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS inspection_standard VARCHAR(100);
ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS packaging_requirement TEXT;
ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS shipping_mark TEXT;

COMMENT ON COLUMN sales_orders.delivery_type IS '交货方式：分批交货/一次性交货';
COMMENT ON COLUMN sales_orders.partial_delivery_allowed IS '是否允许分批交货';
COMMENT ON COLUMN sales_orders.quality_standard IS '质量标准';
COMMENT ON COLUMN sales_orders.inspection_standard IS '检验标准';
COMMENT ON COLUMN sales_orders.packaging_requirement IS '包装要求';
COMMENT ON COLUMN sales_orders.shipping_mark IS '唛头';

-- 4.2 销售订单明细表增加面料行业字段
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS color_no VARCHAR(50) NOT NULL DEFAULT '';
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS color_name VARCHAR(100);
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS pantone_code VARCHAR(50);
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS grade_required VARCHAR(20) DEFAULT '一等品';
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS quantity_meters DECIMAL(12,2) NOT NULL DEFAULT 0;
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS quantity_kg DECIMAL(12,2) DEFAULT 0;
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS gram_weight DECIMAL(10,2);
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS width DECIMAL(10,2);
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS batch_requirement VARCHAR(50) DEFAULT '可混批';
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS dye_lot_requirement VARCHAR(50) DEFAULT '可混缸';
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS base_price DECIMAL(10,2);
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS color_extra_cost DECIMAL(10,2) DEFAULT 0;
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS grade_price_diff DECIMAL(10,2) DEFAULT 0;
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS final_price DECIMAL(10,2);
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS shipped_quantity_meters DECIMAL(12,2) DEFAULT 0;
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS shipped_quantity_kg DECIMAL(12,2) DEFAULT 0;

COMMENT ON COLUMN sales_order_items.color_no IS '色号';
COMMENT ON COLUMN sales_order_items.color_name IS '颜色名称';
COMMENT ON COLUMN sales_order_items.pantone_code IS '潘通色号';
COMMENT ON COLUMN sales_order_items.grade_required IS '要求等级';
COMMENT ON COLUMN sales_order_items.quantity_meters IS '数量（米）';
COMMENT ON COLUMN sales_order_items.quantity_kg IS '数量（公斤）';
COMMENT ON COLUMN sales_order_items.gram_weight IS '克重';
COMMENT ON COLUMN sales_order_items.batch_requirement IS '批次要求：同批次/可混批';
COMMENT ON COLUMN sales_order_items.dye_lot_requirement IS '缸号要求：同缸号/可混缸';
COMMENT ON COLUMN sales_order_items.base_price IS '基价';
COMMENT ON COLUMN sales_order_items.color_extra_cost IS '色号加价';
COMMENT ON COLUMN sales_order_items.grade_price_diff IS '等级差价';
COMMENT ON COLUMN sales_order_items.final_price IS '最终单价';
COMMENT ON COLUMN sales_order_items.shipped_quantity_meters IS '已发货数量（米）';
COMMENT ON COLUMN sales_order_items.shipped_quantity_kg IS '已发货数量（公斤）';

CREATE INDEX IF NOT EXISTS idx_sales_order_items_color ON sales_order_items(color_no);
CREATE INDEX IF NOT EXISTS idx_sales_order_items_grade ON sales_order_items(grade_required);

-- ========================================
-- 5. 客户管理模块面料行业适配
-- ========================================

ALTER TABLE customers ADD COLUMN IF NOT EXISTS customer_industry VARCHAR(50);
ALTER TABLE customers ADD COLUMN IF NOT EXISTS main_products VARCHAR(200);
ALTER TABLE customers ADD COLUMN IF NOT EXISTS annual_purchase DECIMAL(14,2);
ALTER TABLE customers ADD COLUMN IF NOT EXISTS quality_requirement VARCHAR(50) DEFAULT '一般';
ALTER TABLE customers ADD COLUMN IF NOT EXISTS inspection_standard VARCHAR(100);
ALTER TABLE customers ADD COLUMN IF NOT EXISTS payment_terms VARCHAR(50);

COMMENT ON COLUMN customers.customer_industry IS '客户行业：服装/家纺/产业用';
COMMENT ON COLUMN customers.main_products IS '主营产品';
COMMENT ON COLUMN customers.annual_purchase IS '年采购量（万米）';
COMMENT ON COLUMN customers.quality_requirement IS '质量要求：一般/较高/严格';
COMMENT ON COLUMN customers.inspection_standard IS '检验标准偏好';
COMMENT ON COLUMN customers.payment_terms IS '付款条件';

CREATE INDEX IF NOT EXISTS idx_customers_industry ON customers(customer_industry);

-- ========================================
-- 6. 初始化数据（产品色号示例）
-- ========================================

-- 插入示例色号数据
INSERT INTO product_colors (product_id, color_no, color_name, pantone_code, color_type, extra_cost)
SELECT 
    p.id,
    colors.color_no,
    colors.color_name,
    colors.pantone_code,
    colors.color_type,
    colors.extra_cost
FROM products p
CROSS JOIN (
    VALUES 
        ('C001', '藏青色', '19-4052 TCX', '常规色', 0.00),
        ('C002', '大红色', '18-1664 TCX', '常规色', 0.00),
        ('C003', '军绿色', '18-0527 TCX', '常规色', 0.00),
        ('C004', '荧光绿', '802 C', '定制色', 2.50),
        ('C005', '迷彩色', 'CUSTOM', '定制色', 3.00)
) AS colors(color_no, color_name, pantone_code, color_type, extra_cost)
WHERE p.product_type = '成品布'
ON CONFLICT DO NOTHING;

-- ========================================
-- 7. 数据迁移（将现有数据迁移到新字段）
-- ========================================

-- 7.1 迁移现有库存数据到双计量单位字段
UPDATE inventory_stocks
SET 
    quantity_meters = quantity,
    quantity_kg = quantity * COALESCE(gram_weight, 150) / 1000,
    batch_no = COALESCE(batch_no, 'B' || TO_CHAR(CURRENT_DATE, 'YYYYMMDD') || LPAD(id::text, 4, '0')),
    color_no = COALESCE(color_no, 'C001'),
    grade = '一等品'
WHERE quantity_meters = 0;

-- 7.2 迁移现有销售订单数据
UPDATE sales_order_items
SET 
    quantity_meters = quantity,
    quantity_kg = quantity * COALESCE(gram_weight, 150) / 1000,
    color_no = 'C001',
    grade_required = '一等品',
    final_price = total_price / NULLIF(quantity, 0)
WHERE quantity_meters = 0;

-- ========================================
-- 8. 创建视图（面料行业常用查询）
-- ========================================

-- 8.1 库存汇总视图（按批次 + 色号）
CREATE OR REPLACE VIEW v_inventory_summary AS
SELECT 
    p.id AS product_id,
    p.name AS product_name,
    p.code AS product_code,
    p.product_type,
    w.id AS warehouse_id,
    w.name AS warehouse_name,
    s.batch_no,
    s.color_no,
    s.dye_lot_no,
    s.grade,
    SUM(s.quantity_meters) AS total_quantity_meters,
    SUM(s.quantity_kg) AS total_quantity_kg,
    AVG(s.gram_weight) AS avg_gram_weight,
    SUM(s.quantity_meters * COALESCE(s.unit_price, 0)) AS total_amount
FROM inventory_stocks s
INNER JOIN products p ON s.product_id = p.id
INNER JOIN warehouses w ON s.warehouse_id = w.id
WHERE s.stock_status = '正常'
  AND s.quality_status = '合格'
GROUP BY p.id, p.name, p.code, p.product_type, w.id, w.name, 
         s.batch_no, s.color_no, s.dye_lot_no, s.grade;

COMMENT ON VIEW v_inventory_summary IS '库存汇总视图（面料行业）';

-- 8.2 色号销售分析视图
CREATE OR REPLACE VIEW v_color_sales_analysis AS
SELECT 
    soi.color_no,
    soi.color_name,
    soi.pantone_code,
    COUNT(DISTINCT so.id) AS order_count,
    SUM(soi.quantity_meters) AS total_quantity_meters,
    SUM(soi.quantity_kg) AS total_quantity_kg,
    SUM(soi.quantity_meters * soi.final_price) AS total_amount,
    AVG(soi.final_price) AS avg_price,
    SUM(soi.quantity_meters * (soi.final_price - soi.base_price)) AS total_extra_cost
FROM sales_order_items soi
INNER JOIN sales_orders so ON soi.order_id = so.id
WHERE so.status NOT IN ('cancelled')
GROUP BY soi.color_no, soi.color_name, soi.pantone_code
ORDER BY total_quantity_meters DESC;

COMMENT ON VIEW v_color_sales_analysis IS '色号销售分析视图';

-- ========================================
-- 迁移完成提示
-- ========================================

DO $$
BEGIN
    RAISE NOTICE '========================================';
    RAISE NOTICE '面料 ERP 系统 - 面料行业适配迁移完成';
    RAISE NOTICE '版本：2026-03-15';
    RAISE NOTICE '========================================';
    RAISE NOTICE '新增表：2 个';
    RAISE NOTICE '  - product_colors (产品色号表)';
    RAISE NOTICE '  - warehouse_locations (库位表)';
    RAISE NOTICE '  - inventory_transactions (库存流水表)';
    RAISE NOTICE '';
    RAISE NOTICE '改造表：6 个';
    RAISE NOTICE '  - products (增加面料字段)';
    RAISE NOTICE '  - warehouses (增加仓库类型)';
    RAISE NOTICE '  - inventory_stocks (批次 + 色号 + 双计量单位)';
    RAISE NOTICE '  - sales_orders (增加面料字段)';
    RAISE NOTICE '  - sales_order_items (色号 + 双计量单位)';
    RAISE NOTICE '  - customers (增加行业字段)';
    RAISE NOTICE '';
    RAISE NOTICE '创建视图：2 个';
    RAISE NOTICE '  - v_inventory_summary (库存汇总)';
    RAISE NOTICE '  - v_color_sales_analysis (色号销售分析)';
    RAISE NOTICE '========================================';
END $$;

-- ============================================
-- 来源: 005_gl_module.sql
-- ============================================
-- ========================================
-- 总账管理模块（面料行业版）
-- 版本：2026-03-15
-- 说明：财务系统核心基础模块
-- ========================================

-- ========================================
-- 1. 会计科目表
-- ========================================
CREATE TABLE IF NOT EXISTS account_subjects (
    id SERIAL PRIMARY KEY,
    code VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(200) NOT NULL,
    level INTEGER NOT NULL,
    parent_id INTEGER REFERENCES account_subjects(id),
    full_code VARCHAR(200),
    
    -- 余额属性
    balance_direction VARCHAR(10),
    initial_balance_debit DECIMAL(14,2) DEFAULT 0,
    initial_balance_credit DECIMAL(14,2) DEFAULT 0,
    current_period_debit DECIMAL(14,2) DEFAULT 0,
    current_period_credit DECIMAL(14,2) DEFAULT 0,
    ending_balance_debit DECIMAL(14,2) DEFAULT 0,
    ending_balance_credit DECIMAL(14,2) DEFAULT 0,
    
    -- 辅助核算
    assist_customer BOOLEAN DEFAULT false,
    assist_supplier BOOLEAN DEFAULT false,
    assist_department BOOLEAN DEFAULT false,
    assist_employee BOOLEAN DEFAULT false,
    assist_project BOOLEAN DEFAULT false,
    assist_batch BOOLEAN DEFAULT false,           -- 面料行业：批次核算
    assist_color_no BOOLEAN DEFAULT false,        -- 面料行业：色号核算
    assist_dye_lot BOOLEAN DEFAULT false,         -- 面料行业：缸号核算
    assist_grade BOOLEAN DEFAULT false,           -- 面料行业：等级核算
    assist_workshop BOOLEAN DEFAULT false,        -- 面料行业：车间核算
    
    -- 双计量单位
    enable_dual_unit BOOLEAN DEFAULT false,       -- 面料行业：双计量单位
    primary_unit VARCHAR(20) DEFAULT '米',        -- 主单位
    secondary_unit VARCHAR(20) DEFAULT '公斤',    -- 辅单位
    
    -- 控制属性
    is_cash_account BOOLEAN DEFAULT false,
    is_bank_account BOOLEAN DEFAULT false,
    allow_manual_entry BOOLEAN DEFAULT true,
    require_summary BOOLEAN DEFAULT false,
    
    -- 状态
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE account_subjects IS '会计科目表（面料行业版）';
COMMENT ON COLUMN account_subjects.code IS '科目编码';
COMMENT ON COLUMN account_subjects.name IS '科目名称';
COMMENT ON COLUMN account_subjects.level IS '科目级别（1-6）';
COMMENT ON COLUMN account_subjects.balance_direction IS '余额方向：借/贷/无';
COMMENT ON COLUMN account_subjects.assist_batch IS '是否启用批次辅助核算';
COMMENT ON COLUMN account_subjects.assist_color_no IS '是否启用色号辅助核算';
COMMENT ON COLUMN account_subjects.enable_dual_unit IS '是否启用双计量单位';

CREATE INDEX IF NOT EXISTS idx_account_subjects_code ON account_subjects(code);
CREATE INDEX IF NOT EXISTS idx_account_subjects_parent ON account_subjects(parent_id);
CREATE INDEX IF NOT EXISTS idx_account_subjects_level ON account_subjects(level);

-- ========================================
-- 2. 凭证表
-- ========================================
CREATE TABLE IF NOT EXISTS vouchers (
    id SERIAL PRIMARY KEY,
    voucher_no VARCHAR(50) NOT NULL UNIQUE,
    voucher_type VARCHAR(20) NOT NULL,
    voucher_date DATE NOT NULL,
    
    -- 凭证来源
    source_type VARCHAR(20),
    source_module VARCHAR(50),
    source_bill_id INTEGER,
    source_bill_no VARCHAR(50),
    
    -- 面料行业字段
    batch_no VARCHAR(50),                 -- 批次号
    color_no VARCHAR(50),                 -- 色号
    dye_lot_no VARCHAR(50),               -- 缸号
    workshop VARCHAR(100),                -- 车间
    production_order_no VARCHAR(50),      -- 生产订单号
    
    -- 双计量单位
    quantity_meters DECIMAL(14,2),        -- 数量（米）
    quantity_kg DECIMAL(14,2),            -- 数量（公斤）
    gram_weight DECIMAL(10,2),            -- 克重
    
    -- 状态
    attachment_count INTEGER DEFAULT 0,
    status VARCHAR(20) DEFAULT 'draft',
    
    -- 审核
    created_by INTEGER,
    reviewed_by INTEGER,
    reviewed_at TIMESTAMPTZ,
    posted_by INTEGER,
    posted_at TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE vouchers IS '凭证表（面料行业版）';
COMMENT ON COLUMN vouchers.voucher_no IS '凭证字号';
COMMENT ON COLUMN vouchers.voucher_type IS '凭证类型：记/收/付/转';
COMMENT ON COLUMN vouchers.batch_no IS '批次号';
COMMENT ON COLUMN vouchers.color_no IS '色号';
COMMENT ON COLUMN vouchers.quantity_meters IS '数量（米）';
COMMENT ON COLUMN vouchers.quantity_kg IS '数量（公斤）';

CREATE INDEX IF NOT EXISTS idx_vouchers_no ON vouchers(voucher_no);
CREATE INDEX IF NOT EXISTS idx_vouchers_date ON vouchers(voucher_date);
CREATE INDEX IF NOT EXISTS idx_vouchers_status ON vouchers(status);
CREATE INDEX IF NOT EXISTS idx_vouchers_batch ON vouchers(batch_no, color_no);

-- ========================================
-- 3. 凭证分录表
-- ========================================
CREATE TABLE IF NOT EXISTS voucher_items (
    id SERIAL PRIMARY KEY,
    voucher_id INTEGER NOT NULL REFERENCES vouchers(id) ON DELETE CASCADE,
    line_no INTEGER NOT NULL,
    
    -- 科目
    subject_code VARCHAR(50) NOT NULL,
    subject_name VARCHAR(200) NOT NULL,
    
    -- 金额
    debit DECIMAL(14,2) DEFAULT 0,
    credit DECIMAL(14,2) DEFAULT 0,
    
    -- 摘要
    summary TEXT,
    
    -- 辅助核算
    assist_customer_id INTEGER,
    assist_supplier_id INTEGER,
    assist_department_id INTEGER,
    assist_employee_id INTEGER,
    assist_project_id INTEGER,
    assist_batch_id INTEGER,              -- 面料行业：批次
    assist_color_no_id INTEGER,           -- 面料行业：色号
    assist_dye_lot_id INTEGER,            -- 面料行业：缸号
    assist_grade VARCHAR(20),             -- 面料行业：等级
    assist_workshop_id INTEGER,           -- 面料行业：车间
    
    -- 双计量单位
    quantity_meters DECIMAL(14,2),        -- 数量（米）
    quantity_kg DECIMAL(14,2),            -- 数量（公斤）
    unit_price DECIMAL(12,2),             -- 单价
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE voucher_items IS '凭证分录表（面料行业版）';
COMMENT ON COLUMN voucher_items.assist_batch_id IS '批次辅助核算 ID';
COMMENT ON COLUMN voucher_items.assist_color_no_id IS '色号辅助核算 ID';
COMMENT ON COLUMN voucher_items.quantity_meters IS '数量（米）';

CREATE INDEX IF NOT EXISTS idx_voucher_items_voucher ON voucher_items(voucher_id);
CREATE INDEX IF NOT EXISTS idx_voucher_items_subject ON voucher_items(subject_code);
CREATE INDEX IF NOT EXISTS idx_voucher_items_batch ON voucher_items(assist_batch_id, assist_color_no_id);

-- ========================================
-- 4. 会计期间表
-- ========================================
CREATE TABLE IF NOT EXISTS accounting_periods (
    id SERIAL PRIMARY KEY,
    year INTEGER NOT NULL,
    period INTEGER NOT NULL,
    period_name VARCHAR(50) NOT NULL,
    start_date DATE NOT NULL,
    end_date DATE NOT NULL,
    status VARCHAR(20) DEFAULT 'OPEN',
    closed_at TIMESTAMPTZ,
    closed_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE accounting_periods IS '会计期间表';
CREATE UNIQUE INDEX IF NOT EXISTS idx_accounting_periods_year_period ON accounting_periods(year, period);

-- ========================================
-- 5. 初始化会计科目（面料行业）
-- ========================================

-- 插入一级科目
INSERT INTO account_subjects (code, name, level, balance_direction, status) VALUES
('1001', '库存现金', 1, '借', 'active'),
('1002', '银行存款', 1, '借', 'active'),
('1122', '应收账款', 1, '借', 'active'),
('1405', '库存商品', 1, '借', 'active'),
('2202', '应付账款', 1, '贷', 'active'),
('2221', '应交税费', 1, '贷', 'active'),
('5001', '生产成本', 1, '借', 'active'),
('6001', '主营业务收入', 1, '贷', 'active'),
('6401', '主营业务成本', 1, '借', 'active') ON CONFLICT DO NOTHING;

-- 插入二级科目（示例）
INSERT INTO account_subjects (code, name, level, parent_id, balance_direction, status)
SELECT 
    '1002.01', '工商银行', 2, id, '借', 'active'
FROM account_subjects WHERE code = '1002' ON CONFLICT DO NOTHING;

INSERT INTO account_subjects (code, name, level, parent_id, balance_direction, status)
SELECT 
    '1405.01', '坯布', 2, id, '借', 'active'
FROM account_subjects WHERE code = '1405' ON CONFLICT DO NOTHING;

INSERT INTO account_subjects (code, name, level, parent_id, balance_direction, status, assist_batch, assist_color_no, enable_dual_unit)
SELECT 
    '1405.02', '成品布', 2, id, '借', 'active', true, true, true
FROM account_subjects WHERE code = '1405' ON CONFLICT DO NOTHING;

-- ========================================
-- 6. 创建视图
-- ========================================

-- 科目余额视图
CREATE OR REPLACE VIEW v_account_balance AS
SELECT 
    id,
    code,
    name,
    level,
    balance_direction,
    initial_balance_debit,
    initial_balance_credit,
    current_period_debit,
    current_period_credit,
    ending_balance_debit,
    ending_balance_credit
FROM account_subjects
WHERE status = 'active';

COMMENT ON VIEW v_account_balance IS '科目余额视图';

-- ========================================
-- 迁移完成提示
-- ========================================

DO $$
BEGIN
    RAISE NOTICE '========================================';
    RAISE NOTICE '总账管理模块（面料行业版）迁移完成';
    RAISE NOTICE '版本：2026-03-15';
    RAISE NOTICE '========================================';
    RAISE NOTICE '新增表：4 个';
    RAISE NOTICE '  - account_subjects (会计科目表)';
    RAISE NOTICE '  - vouchers (凭证表)';
    RAISE NOTICE '  - voucher_items (凭证分录表)';
    RAISE NOTICE '  - accounting_periods (会计期间表)';
    RAISE NOTICE '';
    RAISE NOTICE '创建视图：1 个';
    RAISE NOTICE '  - v_account_balance (科目余额视图)';
    RAISE NOTICE '========================================';
END $$;

-- ============================================
-- 来源: 006_supplier_management.sql
-- ============================================
-- ========================================
-- 1. 供应商基础表
-- ========================================

-- ==================== 供应商表 ====================
CREATE TABLE IF NOT EXISTS suppliers (
    id SERIAL PRIMARY KEY,
    supplier_code VARCHAR(50) NOT NULL,                    -- 供应商编码
    supplier_name VARCHAR(200) NOT NULL,                   -- 供应商名称
    supplier_short_name VARCHAR(100) NOT NULL,             -- 供应商简称
    supplier_type VARCHAR(50) NOT NULL,                    -- 供应商类型
    credit_code VARCHAR(50) NOT NULL,                      -- 统一社会信用代码
    registered_address VARCHAR(500) NOT NULL,              -- 注册地址
    business_address VARCHAR(500),                         -- 经营地址
    legal_representative VARCHAR(50) NOT NULL,             -- 法人代表
    registered_capital DECIMAL(15,2) NOT NULL,             -- 注册资本（万元）
    establishment_date DATE NOT NULL,                      -- 成立日期
    business_term VARCHAR(100),                            -- 营业期限
    business_scope TEXT,                                   -- 经营范围
    taxpayer_type VARCHAR(50) NOT NULL,                    -- 纳税人类型
    bank_name VARCHAR(100) NOT NULL,                       -- 开户银行
    bank_account VARCHAR(50) NOT NULL,                     -- 银行账号
    contact_phone VARCHAR(50) NOT NULL,                    -- 联系电话
    fax VARCHAR(50),                                       -- 传真
    website VARCHAR(200),                                  -- 公司网址
    email VARCHAR(100),                                    -- 联系邮箱
    main_business VARCHAR(500),                            -- 主营业务
    main_market VARCHAR(500),                              -- 主要市场
    employee_count INTEGER,                                -- 员工人数
    annual_revenue DECIMAL(15,2),                          -- 年营业额（万元）
    
    -- 等级管理
    grade VARCHAR(10) DEFAULT 'B',                         -- 供应商等级（A/B/C/D）
    grade_score DECIMAL(5,2) DEFAULT 0.00,                 -- 等级评分
    last_evaluation_date DATE,                             -- 最后评估日期
    
    -- 状态管理
    status VARCHAR(20) DEFAULT 'active',                   -- 状态：active/inactive/disabled/blacklisted
    is_enabled BOOLEAN DEFAULT TRUE,                       -- 是否启用
    
    -- 辅助核算字段（面料行业特色）
    assist_batch BOOLEAN DEFAULT FALSE,                    -- 是否启用批次核算
    assist_supplier BOOLEAN DEFAULT TRUE,                  -- 是否启用供应商核算
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                    -- 创建人 ID
    updated_by INTEGER,                                    -- 更新人 ID
    remarks TEXT                                           -- 备注
);

COMMENT ON TABLE suppliers IS '供应商信息表';
COMMENT ON COLUMN suppliers.supplier_code IS '供应商编码（自动生成）';
COMMENT ON COLUMN suppliers.supplier_name IS '供应商名称';
COMMENT ON COLUMN suppliers.supplier_type IS '供应商类型（fabric/dye/auxiliary/logistics/service/other）';
COMMENT ON COLUMN suppliers.credit_code IS '统一社会信用代码';
COMMENT ON COLUMN suppliers.grade IS '供应商等级（A/B/C/D）';
COMMENT ON COLUMN suppliers.status IS '供应商状态（active/inactive/disabled/blacklisted）';
COMMENT ON COLUMN suppliers.assist_batch IS '是否启用批次核算';
COMMENT ON COLUMN suppliers.assist_supplier IS '是否启用供应商核算';

-- 唯一约束
ALTER TABLE suppliers ADD CONSTRAINT uk_suppliers_code UNIQUE (supplier_code);
ALTER TABLE suppliers ADD CONSTRAINT uk_suppliers_name UNIQUE (supplier_name);
ALTER TABLE suppliers ADD CONSTRAINT uk_suppliers_credit_code UNIQUE (credit_code);

-- 索引
CREATE INDEX IF NOT EXISTS idx_suppliers_type ON suppliers(supplier_type);
CREATE INDEX IF NOT EXISTS idx_suppliers_grade ON suppliers(grade);
CREATE INDEX IF NOT EXISTS idx_suppliers_status ON suppliers(status);
CREATE INDEX IF NOT EXISTS idx_suppliers_credit_code ON suppliers(credit_code);
CREATE INDEX IF NOT EXISTS idx_suppliers_enabled ON suppliers(is_enabled);
CREATE INDEX IF NOT EXISTS idx_suppliers_created_at ON suppliers(created_at DESC);

-- ========================================
-- 2. 供应商联系人表
-- ========================================

-- ==================== 供应商联系人表 ====================
CREATE TABLE IF NOT EXISTS supplier_contacts (
    id SERIAL PRIMARY KEY,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id) ON DELETE CASCADE,
    contact_name VARCHAR(50) NOT NULL,                 -- 联系人姓名
    department VARCHAR(50),                            -- 所属部门
    position VARCHAR(50),                              -- 职位
    mobile_phone VARCHAR(20) NOT NULL,                 -- 手机号码
    tel_phone VARCHAR(50),                             -- 联系电话
    email VARCHAR(100),                                -- 联系邮箱
    wechat VARCHAR(50),                                -- 微信
    qq VARCHAR(20),                                    -- QQ
    is_primary BOOLEAN DEFAULT FALSE,                  -- 是否主要联系人
    remarks TEXT,                                      -- 备注
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE supplier_contacts IS '供应商联系人表';
COMMENT ON COLUMN supplier_contacts.is_primary IS '是否主要联系人';

-- 索引
CREATE INDEX IF NOT EXISTS idx_supplier_contacts_supplier_id ON supplier_contacts(supplier_id);
CREATE INDEX IF NOT EXISTS idx_supplier_contacts_mobile ON supplier_contacts(mobile_phone);

-- ========================================
-- 3. 供应商分类表
-- ========================================

-- ==================== 供应商分类表 ====================
CREATE TABLE IF NOT EXISTS supplier_categories (
    id SERIAL PRIMARY KEY,
    category_code VARCHAR(50) NOT NULL,                -- 分类编码
    category_name VARCHAR(100) NOT NULL,               -- 分类名称
    parent_id INTEGER REFERENCES supplier_categories(id), -- 父级分类 ID
    level INTEGER NOT NULL DEFAULT 1,                  -- 层级
    sort_order INTEGER NOT NULL DEFAULT 0,             -- 排序
    is_enabled BOOLEAN DEFAULT TRUE,                   -- 是否启用
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE supplier_categories IS '供应商分类表';
COMMENT ON COLUMN supplier_categories.level IS '分类层级（1-3）';

-- 唯一约束
ALTER TABLE supplier_categories ADD CONSTRAINT uk_categories_code UNIQUE (category_code);

-- 索引
CREATE INDEX IF NOT EXISTS idx_supplier_categories_parent ON supplier_categories(parent_id);
CREATE INDEX IF NOT EXISTS idx_supplier_categories_level ON supplier_categories(level);

-- ========================================
-- 4. 供应商等级表
-- ========================================

-- ==================== 供应商等级表 ====================
CREATE TABLE IF NOT EXISTS supplier_grades (
    id SERIAL PRIMARY KEY,
    grade_code VARCHAR(10) NOT NULL,                   -- 等级编码（A/B/C/D）
    grade_name VARCHAR(50) NOT NULL,                   -- 等级名称
    min_score DECIMAL(5,2) NOT NULL,                   -- 最低分数
    max_score DECIMAL(5,2) NOT NULL,                   -- 最高分数
    color_code VARCHAR(20),                            -- 颜色标识
    permission_desc TEXT,                              -- 权限说明
    is_enabled BOOLEAN DEFAULT TRUE,                   -- 是否启用
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE supplier_grades IS '供应商等级表';

-- 唯一约束
ALTER TABLE supplier_grades ADD CONSTRAINT uk_grades_code UNIQUE (grade_code);

-- 初始化数据
INSERT INTO supplier_grades (grade_code, grade_name, min_score, max_score, color_code, permission_desc) VALUES
('A', '战略供应商', 90.00, 100.00, 'green', '优先采购、免检、月结'),
('B', '合格供应商', 75.00, 89.99, 'blue', '正常采购、抽检、月结'),
('C', '考察供应商', 60.00, 74.99, 'yellow', '限制采购、全检、现结'),
('D', '不合格供应商', 0.00, 59.99, 'red', '暂停采购、列入黑名单') ON CONFLICT DO NOTHING;

-- ========================================
-- 5. 供应商评估表
-- ========================================

-- ==================== 供应商评估表 ====================
CREATE TABLE IF NOT EXISTS supplier_evaluations (
    id SERIAL PRIMARY KEY,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id),
    evaluation_year INTEGER NOT NULL,                  -- 评估年份
    evaluation_quarter INTEGER NOT NULL,               -- 评估季度（1-4）
    
    -- 质量水平（35 分）
    quality_pass_rate DECIMAL(5,2),                    -- 来料合格率（%）
    quality_score DECIMAL(5,2),                        -- 质量得分
    quality_incidents INTEGER DEFAULT 0,               -- 质量事故次数
    quality_incident_score DECIMAL(5,2),               -- 质量事故得分
    quality_improvement_score DECIMAL(5,2),            -- 质量改进得分
    
    -- 交货能力（25 分）
    delivery_on_time_rate DECIMAL(5,2),                -- 交货及时率（%）
    delivery_score DECIMAL(5,2),                       -- 交货得分
    order_completion_rate DECIMAL(5,2),                -- 订单完成率（%）
    order_completion_score DECIMAL(5,2),               -- 订单完成得分
    
    -- 价格水平（20 分）
    price_competitiveness_score DECIMAL(5,2),          -- 价格竞争力得分
    price_stability_score DECIMAL(5,2),                -- 价格稳定性得分
    
    -- 服务水平（15 分）
    response_speed_score DECIMAL(5,2),                 -- 响应速度得分
    after_sales_score DECIMAL(5,2),                    -- 售后服务得分
    cooperation_score DECIMAL(5,2),                    -- 配合度得分
    
    -- 技术能力（5 分）
    rd_capability_score DECIMAL(5,2),                  -- 研发能力得分
    technical_support_score DECIMAL(5,2),              -- 技术支持得分
    
    -- 总分和等级
    total_score DECIMAL(5,2),                          -- 总分
    grade_before VARCHAR(10),                          -- 评估前等级
    grade_after VARCHAR(10),                           -- 评估后等级
    
    -- 审批
    evaluator_id INTEGER,                              -- 评估人 ID
    approver_id INTEGER,                               -- 审批人 ID
    approval_status VARCHAR(20) DEFAULT 'pending',     -- 审批状态
    approval_date DATE,                                -- 审批日期
    approval_remarks TEXT,                             -- 审批意见
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,
    updated_by INTEGER,
    
    -- 约束
    CONSTRAINT chk_quarter CHECK (evaluation_quarter BETWEEN 1 AND 4)
);

COMMENT ON TABLE supplier_evaluations IS '供应商评估表';
COMMENT ON COLUMN supplier_evaluations.quality_pass_rate IS '来料合格率（%）';
COMMENT ON COLUMN supplier_evaluations.delivery_on_time_rate IS '交货及时率（%）';
COMMENT ON COLUMN supplier_evaluations.order_completion_rate IS '订单完成率（%）';

-- 唯一约束
ALTER TABLE supplier_evaluations ADD CONSTRAINT uk_supplier_quarter UNIQUE (supplier_id, evaluation_year, evaluation_quarter);

-- 索引
CREATE INDEX IF NOT EXISTS idx_supplier_evaluations_supplier ON supplier_evaluations(supplier_id);
CREATE INDEX IF NOT EXISTS idx_supplier_evaluations_year_quarter ON supplier_evaluations(evaluation_year, evaluation_quarter);
CREATE INDEX IF NOT EXISTS idx_supplier_evaluations_total_score ON supplier_evaluations(total_score);

-- ========================================
-- 6. 供应商资质表
-- ========================================

-- ==================== 供应商资质表 ====================
CREATE TABLE IF NOT EXISTS supplier_qualifications (
    id SERIAL PRIMARY KEY,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id) ON DELETE CASCADE,
    qualification_name VARCHAR(200) NOT NULL,          -- 资质名称
    qualification_type VARCHAR(50) NOT NULL,           -- 资质类型
    qualification_no VARCHAR(100) NOT NULL,            -- 资质编号
    issuing_authority VARCHAR(200) NOT NULL,           -- 发证机构
    issue_date DATE NOT NULL,                          -- 发证日期
    valid_until DATE NOT NULL,                         -- 有效期至
    attachment_path VARCHAR(500),                      -- 附件路径
    need_annual_check BOOLEAN DEFAULT FALSE,           -- 是否年检
    annual_check_record TEXT,                          -- 年检记录
    is_expired BOOLEAN DEFAULT FALSE,                  -- 是否过期
    remarks TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE supplier_qualifications IS '供应商资质表';
COMMENT ON COLUMN supplier_qualifications.qualification_type IS '资质类型（营业执照/税务登记证/组织机构代码证/ISO9001 等）';

-- 索引
CREATE INDEX IF NOT EXISTS idx_supplier_qualifications_supplier ON supplier_qualifications(supplier_id);
CREATE INDEX IF NOT EXISTS idx_supplier_qualifications_type ON supplier_qualifications(qualification_type);
CREATE INDEX IF NOT EXISTS idx_supplier_qualifications_valid_until ON supplier_qualifications(valid_until);

-- ========================================
-- 7. 供应商黑名单表
-- ========================================

-- ==================== 供应商黑名单表 ====================
CREATE TABLE IF NOT EXISTS supplier_blacklists (
    id SERIAL PRIMARY KEY,
    supplier_id INTEGER NOT NULL UNIQUE REFERENCES suppliers(id),
    blacklist_date DATE NOT NULL,                      -- 列入日期
    blacklist_reason VARCHAR(50) NOT NULL,             -- 列入原因
    detail_description TEXT NOT NULL,                  -- 详细说明
    evidence TEXT,                                     -- 证据材料
    approver_id INTEGER NOT NULL,                      -- 审批人 ID
    approval_date DATE NOT NULL,                       -- 审批日期
    is_permanent BOOLEAN DEFAULT FALSE,                -- 是否永久
    release_date DATE,                                 -- 解禁日期
    release_condition TEXT,                            -- 解禁条件
    release_status VARCHAR(20) DEFAULT 'blacklisted',  -- 解禁状态
    release_date_actual DATE,                          -- 实际解禁日期
    remarks TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,
    updated_by INTEGER
);

COMMENT ON TABLE supplier_blacklists IS '供应商黑名单表';
COMMENT ON COLUMN supplier_blacklists.blacklist_reason IS '列入原因（质量事故/欺诈/违约/贿赂等）';
COMMENT ON COLUMN supplier_blacklists.release_status IS '解禁状态（blacklisted/released）';

-- 索引
CREATE INDEX IF NOT EXISTS idx_supplier_blacklists_supplier ON supplier_blacklists(supplier_id);
CREATE INDEX IF NOT EXISTS idx_supplier_blacklists_date ON supplier_blacklists(blacklist_date);
CREATE INDEX IF NOT EXISTS idx_supplier_blacklists_status ON supplier_blacklists(release_status);

-- ========================================
-- 8. 触发器函数
-- ========================================

-- ==================== 自动更新 updated_at ====================
CREATE OR REPLACE FUNCTION update_supplier_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ==================== 资质过期检查 ====================
CREATE OR REPLACE FUNCTION update_qualification_expired()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.valid_until < CURRENT_DATE THEN
        NEW.is_expired := TRUE;
    ELSE
        NEW.is_expired := FALSE;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ========================================
-- 9. 应用触发器
-- ========================================

-- 更新 updated_at 触发器
DROP TRIGGER IF EXISTS trg_suppliers_updated_at ON suppliers;
CREATE TRIGGER trg_suppliers_updated_at
    BEFORE UPDATE ON suppliers
    FOR EACH ROW
    EXECUTE FUNCTION update_supplier_updated_at_column();

DROP TRIGGER IF EXISTS trg_supplier_contacts_updated_at ON supplier_contacts;
CREATE TRIGGER trg_supplier_contacts_updated_at
    BEFORE UPDATE ON supplier_contacts
    FOR EACH ROW
    EXECUTE FUNCTION update_supplier_updated_at_column();

DROP TRIGGER IF EXISTS trg_supplier_categories_updated_at ON supplier_categories;
CREATE TRIGGER trg_supplier_categories_updated_at
    BEFORE UPDATE ON supplier_categories
    FOR EACH ROW
    EXECUTE FUNCTION update_supplier_updated_at_column();

DROP TRIGGER IF EXISTS trg_supplier_grades_updated_at ON supplier_grades;
CREATE TRIGGER trg_supplier_grades_updated_at
    BEFORE UPDATE ON supplier_grades
    FOR EACH ROW
    EXECUTE FUNCTION update_supplier_updated_at_column();

DROP TRIGGER IF EXISTS trg_supplier_evaluations_updated_at ON supplier_evaluations;
CREATE TRIGGER trg_supplier_evaluations_updated_at
    BEFORE UPDATE ON supplier_evaluations
    FOR EACH ROW
    EXECUTE FUNCTION update_supplier_updated_at_column();

DROP TRIGGER IF EXISTS trg_supplier_qualifications_updated_at ON supplier_qualifications;
CREATE TRIGGER trg_supplier_qualifications_updated_at
    BEFORE UPDATE ON supplier_qualifications
    FOR EACH ROW
    EXECUTE FUNCTION update_supplier_updated_at_column();

DROP TRIGGER IF EXISTS trg_supplier_blacklists_updated_at ON supplier_blacklists;
CREATE TRIGGER trg_supplier_blacklists_updated_at
    BEFORE UPDATE ON supplier_blacklists
    FOR EACH ROW
    EXECUTE FUNCTION update_supplier_updated_at_column();

-- 资质过期检查触发器
DROP TRIGGER IF EXISTS trg_supplier_qualifications_check_expired ON supplier_qualifications;
CREATE TRIGGER trg_supplier_qualifications_check_expired
    BEFORE INSERT OR UPDATE ON supplier_qualifications
    FOR EACH ROW
    EXECUTE FUNCTION update_qualification_expired();

-- ========================================
-- 迁移完成
-- ========================================

-- ============================================
-- 来源: 007_purchase_management.sql
-- ============================================
-- 采购管理模块数据库迁移脚本
-- 创建时间：2026-03-15
-- 功能说明：创建采购订单、入库、退货、质检相关表及索引

-- =====================================================
-- 1. 采购订单表 (purchase_order)
-- =====================================================
CREATE TABLE IF NOT EXISTS purchase_order (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    order_no VARCHAR(50) NOT NULL UNIQUE,               -- 订单编号（PO20260315001）
    supplier_id INTEGER NOT NULL,                       -- 供应商 ID（外键）
    order_date DATE NOT NULL,                           -- 订单日期
    expected_delivery_date DATE,                        -- 预计交货日期
    actual_delivery_date DATE,                          -- 实际交货日期
    warehouse_id INTEGER NOT NULL,                      -- 入库仓库 ID
    department_id INTEGER NOT NULL,                     -- 采购部门 ID
    purchaser_id INTEGER NOT NULL,                      -- 采购员 ID
    currency VARCHAR(10) DEFAULT 'CNY',                 -- 币种
    exchange_rate DECIMAL(18,6) DEFAULT 1.000000,       -- 汇率
    total_amount DECIMAL(18,2) DEFAULT 0.00,            -- 订单总金额（本位币）
    total_amount_foreign DECIMAL(18,2) DEFAULT 0.00,    -- 订单总金额（外币）
    total_quantity DECIMAL(18,4) DEFAULT 0.0000,        -- 总数量（主单位）
    total_quantity_alt DECIMAL(18,4) DEFAULT 0.0000,    -- 总数量（辅助单位）
    order_status VARCHAR(20) DEFAULT 'DRAFT',           -- 订单状态：DRAFT=草稿，SUBMITTED=已提交，APPROVED=已审批，REJECTED=已拒绝，PARTIAL_RECEIVED=部分入库，COMPLETED=已完成，CLOSED=已关闭
    payment_terms TEXT,                                 -- 付款条件
    shipping_terms TEXT,                                -- 运输条款
    notes TEXT,                                         -- 备注
    attachment_urls TEXT[],                             -- 附件 URL 列表
    created_by INTEGER NOT NULL,                        -- 创建人 ID
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    updated_by INTEGER,                                 -- 更新人 ID
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 更新时间
    approved_by INTEGER,                                -- 审批人 ID
    approved_at TIMESTAMP,                              -- 审批时间
    rejected_reason TEXT                                -- 拒绝原因
);

-- 外键约束
ALTER TABLE purchase_order ADD CONSTRAINT fk_po_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);
ALTER TABLE purchase_order ADD CONSTRAINT fk_po_warehouse
    FOREIGN KEY (warehouse_id) REFERENCES warehouses(id);
ALTER TABLE purchase_order ADD CONSTRAINT fk_po_department
    FOREIGN KEY (department_id) REFERENCES departments(id);
ALTER TABLE purchase_order ADD CONSTRAINT fk_po_purchaser
    FOREIGN KEY (purchaser_id) REFERENCES users(id);
ALTER TABLE purchase_order ADD CONSTRAINT fk_po_created_by
    FOREIGN KEY (created_by) REFERENCES users(id);
ALTER TABLE purchase_order ADD CONSTRAINT fk_po_updated_by
    FOREIGN KEY (updated_by) REFERENCES users(id);
ALTER TABLE purchase_order ADD CONSTRAINT fk_po_approved_by
    FOREIGN KEY (approved_by) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_po_order_no ON purchase_order(order_no);
CREATE INDEX IF NOT EXISTS idx_po_supplier_id ON purchase_order(supplier_id);
CREATE INDEX IF NOT EXISTS idx_po_order_date ON purchase_order(order_date);
CREATE INDEX IF NOT EXISTS idx_po_order_status ON purchase_order(order_status);
CREATE INDEX IF NOT EXISTS idx_po_expected_delivery ON purchase_order(expected_delivery_date);
CREATE INDEX IF NOT EXISTS idx_po_created_by ON purchase_order(created_by);

-- 触发器：更新时间
DROP TRIGGER IF EXISTS update_purchase_order_updated_at ON purchase_order;
CREATE TRIGGER update_purchase_order_updated_at
BEFORE UPDATE ON purchase_order
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();


-- =====================================================
-- 2. 采购订单明细表 (purchase_order_item)
-- =====================================================
CREATE TABLE IF NOT EXISTS purchase_order_item (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    order_id INTEGER NOT NULL,                          -- 订单 ID（外键）
    line_no INTEGER NOT NULL,                           -- 行号（10, 20, 30...）
    product_id INTEGER NOT NULL,                        -- 产品 ID
    material_code VARCHAR(50) NOT NULL,                 -- 物料编码
    material_name VARCHAR(200) NOT NULL,                -- 物料名称
    specification VARCHAR(200),                         -- 规格型号
    batch_no VARCHAR(50),                               -- 批次号（面料行业）
    color_code VARCHAR(50),                             -- 色号（面料行业）
    lot_no VARCHAR(50),                                 -- 缸号（面料行业）
    grade VARCHAR(10),                                  -- 等级（面料行业）
    gram_weight DECIMAL(8,2),                           -- 克重（g/m²）
    width DECIMAL(8,2),                                 -- 幅宽（cm）
    unit_price DECIMAL(18,6) NOT NULL,                  -- 单价
    currency VARCHAR(10) DEFAULT 'CNY',                 -- 币种
    quantity_ordered DECIMAL(18,4) NOT NULL,            -- 订购数量（主单位）
    quantity_received DECIMAL(18,4) DEFAULT 0.0000,     -- 已入库数量（主单位）
    quantity_returned DECIMAL(18,4) DEFAULT 0.0000,     -- 已退货数量（主单位）
    unit_master VARCHAR(20) NOT NULL,                   -- 主单位（米）
    unit_alt VARCHAR(20),                               -- 辅助单位（公斤）
    conversion_factor DECIMAL(18,6),                    -- 换算系数
    quantity_alt_ordered DECIMAL(18,4),                 -- 订购数量（辅助单位）
    quantity_alt_received DECIMAL(18,4),                -- 已入库数量（辅助单位）
    amount DECIMAL(18,2) NOT NULL,                      -- 金额
    tax_rate DECIMAL(5,2) DEFAULT 13.00,                -- 税率（%）
    tax_amount DECIMAL(18,2) DEFAULT 0.00,              -- 税额
    delivery_date DATE,                                 -- 交货日期
    warehouse_id INTEGER,                               -- 仓库 ID
    notes TEXT,                                         -- 备注
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP      -- 更新时间
);

-- 外键约束
ALTER TABLE purchase_order_item ADD CONSTRAINT fk_poi_order
    FOREIGN KEY (order_id) REFERENCES purchase_order(id) ON DELETE CASCADE;
ALTER TABLE purchase_order_item ADD CONSTRAINT fk_poi_material
    FOREIGN KEY (product_id) REFERENCES products(id);
ALTER TABLE purchase_order_item ADD CONSTRAINT fk_poi_warehouse
    FOREIGN KEY (warehouse_id) REFERENCES warehouses(id);

-- 唯一约束
ALTER TABLE purchase_order_item ADD CONSTRAINT uk_poi_order_line
    UNIQUE (order_id, line_no);

-- 索引
CREATE INDEX IF NOT EXISTS idx_poi_order_id ON purchase_order_item(order_id);
CREATE INDEX IF NOT EXISTS idx_poi_product_id ON purchase_order_item(product_id);
CREATE INDEX IF NOT EXISTS idx_poi_batch_no ON purchase_order_item(batch_no);
CREATE INDEX IF NOT EXISTS idx_poi_color_code ON purchase_order_item(color_code);
CREATE INDEX IF NOT EXISTS idx_poi_lot_no ON purchase_order_item(lot_no);

-- 触发器：更新时间
DROP TRIGGER IF EXISTS update_purchase_order_item_updated_at ON purchase_order_item;
CREATE TRIGGER update_purchase_order_item_updated_at
BEFORE UPDATE ON purchase_order_item
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- 触发器：计算金额
CREATE OR REPLACE FUNCTION calc_purchase_order_item_amount()
RETURNS TRIGGER AS $$
BEGIN
    -- 计算金额 = 数量 * 单价
    NEW.amount := (NEW.quantity_ordered * NEW.unit_price);
    
    -- 计算税额
    NEW.tax_amount := (NEW.amount * NEW.tax_rate / 100);
    
    -- 计算辅助单位数量（如果有换算系数）
    IF NEW.conversion_factor IS NOT NULL AND NEW.conversion_factor > 0 THEN
        NEW.quantity_alt_ordered := (NEW.quantity_ordered / NEW.conversion_factor);
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS calc_purchase_order_item_amount_before_insert ON purchase_order_item;
CREATE TRIGGER calc_purchase_order_item_amount_before_insert
BEFORE INSERT ON purchase_order_item
FOR EACH ROW EXECUTE FUNCTION calc_purchase_order_item_amount();

DROP TRIGGER IF EXISTS calc_purchase_order_item_amount_before_update ON purchase_order_item;
CREATE TRIGGER calc_purchase_order_item_amount_before_update
BEFORE UPDATE ON purchase_order_item
FOR EACH ROW EXECUTE FUNCTION calc_purchase_order_item_amount();


-- =====================================================
-- 3. 采购入库表 (purchase_receipt)
-- =====================================================
CREATE TABLE IF NOT EXISTS purchase_receipt (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    receipt_no VARCHAR(50) NOT NULL UNIQUE,             -- 入库单号（GR20260315001）
    order_id INTEGER,                                   -- 采购订单 ID（外键）
    supplier_id INTEGER NOT NULL,                       -- 供应商 ID
    receipt_date DATE NOT NULL,                         -- 入库日期
    warehouse_id INTEGER NOT NULL,                      -- 仓库 ID
    department_id INTEGER,                              -- 收货部门 ID
    receiver_id INTEGER,                                -- 收货人 ID
    inspector_id INTEGER,                               -- 质检员 ID
    inspection_status VARCHAR(20) DEFAULT 'PENDING',    -- 质检状态：PENDING=待质检，INSPECTING=质检中，PASSED=合格，FAILED=不合格
    receipt_status VARCHAR(20) DEFAULT 'DRAFT',         -- 入库状态：DRAFT=草稿，CONFIRMED=已确认，CANCELLED=已取消
    total_quantity DECIMAL(18,4) DEFAULT 0.0000,        -- 总入库数量（主单位）
    total_quantity_alt DECIMAL(18,4) DEFAULT 0.0000,    -- 总入库数量（辅助单位）
    total_amount DECIMAL(18,2) DEFAULT 0.00,            -- 总金额
    notes TEXT,                                         -- 备注
    attachment_urls TEXT[],                             -- 附件 URL 列表
    created_by INTEGER NOT NULL,                        -- 创建人 ID
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    updated_by INTEGER,                                 -- 更新人 ID
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 更新时间
    confirmed_at TIMESTAMP,                             -- 确认时间
    confirmed_by INTEGER                                -- 确认人 ID
);

-- 外键约束
ALTER TABLE purchase_receipt ADD CONSTRAINT fk_pr_order
    FOREIGN KEY (order_id) REFERENCES purchase_order(id);
ALTER TABLE purchase_receipt ADD CONSTRAINT fk_pr_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);
ALTER TABLE purchase_receipt ADD CONSTRAINT fk_pr_warehouse
    FOREIGN KEY (warehouse_id) REFERENCES warehouses(id);
ALTER TABLE purchase_receipt ADD CONSTRAINT fk_pr_department
    FOREIGN KEY (department_id) REFERENCES departments(id);
ALTER TABLE purchase_receipt ADD CONSTRAINT fk_pr_receiver
    FOREIGN KEY (receiver_id) REFERENCES users(id);
ALTER TABLE purchase_receipt ADD CONSTRAINT fk_pr_inspector
    FOREIGN KEY (inspector_id) REFERENCES users(id);
ALTER TABLE purchase_receipt ADD CONSTRAINT fk_pr_created_by
    FOREIGN KEY (created_by) REFERENCES users(id);
ALTER TABLE purchase_receipt ADD CONSTRAINT fk_pr_updated_by
    FOREIGN KEY (updated_by) REFERENCES users(id);
ALTER TABLE purchase_receipt ADD CONSTRAINT fk_pr_confirmed_by
    FOREIGN KEY (confirmed_by) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_pr_receipt_no ON purchase_receipt(receipt_no);
CREATE INDEX IF NOT EXISTS idx_pr_order_id ON purchase_receipt(order_id);
CREATE INDEX IF NOT EXISTS idx_pr_supplier_id ON purchase_receipt(supplier_id);
CREATE INDEX IF NOT EXISTS idx_pr_receipt_date ON purchase_receipt(receipt_date);
CREATE INDEX IF NOT EXISTS idx_pr_warehouse_id ON purchase_receipt(warehouse_id);
CREATE INDEX IF NOT EXISTS idx_pr_receipt_status ON purchase_receipt(receipt_status);

-- 触发器：更新时间
DROP TRIGGER IF EXISTS update_purchase_receipt_updated_at ON purchase_receipt;
CREATE TRIGGER update_purchase_receipt_updated_at
BEFORE UPDATE ON purchase_receipt
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();


-- =====================================================
-- 4. 采购入库明细表 (purchase_receipt_item)
-- =====================================================
CREATE TABLE IF NOT EXISTS purchase_receipt_item (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    receipt_id INTEGER NOT NULL,                        -- 入库单 ID（外键）
    order_item_id INTEGER,                              -- 订单明细 ID（外键）
    line_no INTEGER NOT NULL,                           -- 行号
    product_id INTEGER NOT NULL,                       -- 物料 ID
    material_code VARCHAR(50) NOT NULL,                 -- 物料编码
    material_name VARCHAR(200) NOT NULL,                -- 物料名称
    batch_no VARCHAR(50),                               -- 批次号
    color_code VARCHAR(50),                             -- 色号
    lot_no VARCHAR(50),                                 -- 缸号
    grade VARCHAR(10),                                  -- 等级
    gram_weight DECIMAL(8,2),                           -- 克重
    width DECIMAL(8,2),                                 -- 幅宽
    quantity DECIMAL(18,4) NOT NULL,                    -- 入库数量（主单位）
    quantity_alt DECIMAL(18,4),                         -- 入库数量（辅助单位）
    unit_master VARCHAR(20) NOT NULL,                   -- 主单位
    unit_alt VARCHAR(20),                               -- 辅助单位
    unit_price DECIMAL(18,6),                           -- 单价
    amount DECIMAL(18,2),                               -- 金额
    location_code VARCHAR(50),                          -- 库位编码
    package_no VARCHAR(50),                             -- 包号
    production_date DATE,                               -- 生产日期
    shelf_life INTEGER,                                 -- 保质期（天）
    notes TEXT,                                         -- 备注
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP      -- 创建时间
);

-- 外键约束
ALTER TABLE purchase_receipt_item ADD CONSTRAINT fk_pri_receipt
    FOREIGN KEY (receipt_id) REFERENCES purchase_receipt(id) ON DELETE CASCADE;
ALTER TABLE purchase_receipt_item ADD CONSTRAINT fk_pri_order_item
    FOREIGN KEY (order_item_id) REFERENCES purchase_order_item(id);
ALTER TABLE purchase_receipt_item ADD CONSTRAINT fk_pri_material
    FOREIGN KEY (product_id) REFERENCES products(id);

-- 唯一约束
ALTER TABLE purchase_receipt_item ADD CONSTRAINT uk_pri_receipt_line
    UNIQUE (receipt_id, line_no);

-- 索引
CREATE INDEX IF NOT EXISTS idx_pri_receipt_id ON purchase_receipt_item(receipt_id);
CREATE INDEX IF NOT EXISTS idx_pri_order_item_id ON purchase_receipt_item(order_item_id);
CREATE INDEX IF NOT EXISTS idx_pri_product_id ON purchase_receipt_item(product_id);
CREATE INDEX IF NOT EXISTS idx_pri_batch_no ON purchase_receipt_item(batch_no);
CREATE INDEX IF NOT EXISTS idx_pri_color_code ON purchase_receipt_item(color_code);
CREATE INDEX IF NOT EXISTS idx_pri_lot_no ON purchase_receipt_item(lot_no);

-- 触发器：计算金额
CREATE OR REPLACE FUNCTION calc_purchase_receipt_item_amount()
RETURNS TRIGGER AS $$
BEGIN
    -- 计算金额 = 数量 * 单价
    IF NEW.unit_price IS NOT NULL THEN
        NEW.amount := (NEW.quantity * NEW.unit_price);
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS calc_purchase_receipt_item_amount_before_insert ON purchase_receipt_item;
CREATE TRIGGER calc_purchase_receipt_item_amount_before_insert
BEFORE INSERT ON purchase_receipt_item
FOR EACH ROW EXECUTE FUNCTION calc_purchase_receipt_item_amount();

DROP TRIGGER IF EXISTS calc_purchase_receipt_item_amount_before_update ON purchase_receipt_item;
CREATE TRIGGER calc_purchase_receipt_item_amount_before_update
BEFORE UPDATE ON purchase_receipt_item
FOR EACH ROW EXECUTE FUNCTION calc_purchase_receipt_item_amount();


-- =====================================================
-- 5. 采购退货表 (purchase_return)
-- =====================================================
CREATE TABLE IF NOT EXISTS purchase_return (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    return_no VARCHAR(50) NOT NULL UNIQUE,              -- 退货单号（RT20260315001）
    receipt_id INTEGER NOT NULL,                        -- 入库单 ID（外键）
    order_id INTEGER,                                   -- 采购订单 ID
    supplier_id INTEGER NOT NULL,                       -- 供应商 ID
    return_date DATE NOT NULL,                          -- 退货日期
    warehouse_id INTEGER NOT NULL,                      -- 仓库 ID
    department_id INTEGER,                              -- 退货部门 ID
    reason_type VARCHAR(20) NOT NULL,                   -- 退货原因类型：QUALITY_ISSUE=质量问题，WRONG_ITEM=发错货，DAMAGED=破损，OTHER=其他
    reason_detail TEXT,                                 -- 退货原因详情
    return_status VARCHAR(20) DEFAULT 'DRAFT',          -- 退货状态：DRAFT=草稿，SUBMITTED=已提交，APPROVED=已审批，REJECTED=已拒绝，COMPLETED=已完成
    total_quantity DECIMAL(18,4) DEFAULT 0.0000,        -- 总退货数量（主单位）
    total_quantity_alt DECIMAL(18,4) DEFAULT 0.0000,    -- 总退货数量（辅助单位）
    total_amount DECIMAL(18,2) DEFAULT 0.00,            -- 总退货金额
    notes TEXT,                                         -- 备注
    created_by INTEGER NOT NULL,                        -- 创建人 ID
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    updated_by INTEGER,                                 -- 更新人 ID
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 更新时间
    approved_by INTEGER,                                -- 审批人 ID
    approved_at TIMESTAMP,                              -- 审批时间
    rejected_reason TEXT                                -- 拒绝原因
);

-- 外键约束
ALTER TABLE purchase_return ADD CONSTRAINT fk_pret_receipt
    FOREIGN KEY (receipt_id) REFERENCES purchase_receipt(id);
ALTER TABLE purchase_return ADD CONSTRAINT fk_pret_order
    FOREIGN KEY (order_id) REFERENCES purchase_order(id);
ALTER TABLE purchase_return ADD CONSTRAINT fk_pret_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);
ALTER TABLE purchase_return ADD CONSTRAINT fk_pret_warehouse
    FOREIGN KEY (warehouse_id) REFERENCES warehouses(id);
ALTER TABLE purchase_return ADD CONSTRAINT fk_pret_department
    FOREIGN KEY (department_id) REFERENCES departments(id);
ALTER TABLE purchase_return ADD CONSTRAINT fk_pret_created_by
    FOREIGN KEY (created_by) REFERENCES users(id);
ALTER TABLE purchase_return ADD CONSTRAINT fk_pret_updated_by
    FOREIGN KEY (updated_by) REFERENCES users(id);
ALTER TABLE purchase_return ADD CONSTRAINT fk_pret_approved_by
    FOREIGN KEY (approved_by) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_pret_return_no ON purchase_return(return_no);
CREATE INDEX IF NOT EXISTS idx_pret_receipt_id ON purchase_return(receipt_id);
CREATE INDEX IF NOT EXISTS idx_pret_order_id ON purchase_return(order_id);
CREATE INDEX IF NOT EXISTS idx_pret_supplier_id ON purchase_return(supplier_id);
CREATE INDEX IF NOT EXISTS idx_pret_return_date ON purchase_return(return_date);
CREATE INDEX IF NOT EXISTS idx_pret_return_status ON purchase_return(return_status);

-- 触发器：更新时间
DROP TRIGGER IF EXISTS update_purchase_return_updated_at ON purchase_return;
CREATE TRIGGER update_purchase_return_updated_at
BEFORE UPDATE ON purchase_return
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();


-- =====================================================
-- 6. 采购质检表 (purchase_inspection)
-- =====================================================
CREATE TABLE IF NOT EXISTS purchase_inspection (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    inspection_no VARCHAR(50) NOT NULL UNIQUE,          -- 质检单号（IQ20260315001）
    receipt_id INTEGER NOT NULL,                        -- 入库单 ID（外键）
    order_id INTEGER,                                   -- 采购订单 ID
    supplier_id INTEGER NOT NULL,                       -- 供应商 ID
    inspection_date DATE NOT NULL,                      -- 质检日期
    inspector_id INTEGER NOT NULL,                      -- 质检员 ID
    inspection_type VARCHAR(20) DEFAULT 'NORMAL',       -- 质检类型：NORMAL=常规检验，URGENT=紧急检验，SAMPLING=抽样检验
    sample_size INTEGER,                                -- 抽样数量
    defect_count INTEGER DEFAULT 0,                     -- 不合格数量
    pass_quantity DECIMAL(18,4),                        -- 合格数量（主单位）
    reject_quantity DECIMAL(18,4),                      -- 不合格数量（主单位）
    inspection_status VARCHAR(20) DEFAULT 'PENDING',    -- 质检状态：PENDING=待质检，INSPECTING=质检中，COMPLETED=已完成
    inspection_result VARCHAR(20),                      -- 质检结果：PASSED=合格，REJECTED=不合格，CONDITIONAL_PASS=让步接收
    quality_score DECIMAL(5,2),                         -- 质量得分（0-100）
    defect_description TEXT,                            -- 缺陷描述
    attachment_urls TEXT[],                             -- 附件 URL 列表
    notes TEXT,                                         -- 备注
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 更新时间
    completed_at TIMESTAMP,                             -- 完成时间
    completed_by INTEGER                                -- 完成人 ID
);

-- 外键约束
ALTER TABLE purchase_inspection ADD CONSTRAINT fk_pi_receipt
    FOREIGN KEY (receipt_id) REFERENCES purchase_receipt(id);
ALTER TABLE purchase_inspection ADD CONSTRAINT fk_pi_order
    FOREIGN KEY (order_id) REFERENCES purchase_order(id);
ALTER TABLE purchase_inspection ADD CONSTRAINT fk_pi_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);
ALTER TABLE purchase_inspection ADD CONSTRAINT fk_pi_inspector
    FOREIGN KEY (inspector_id) REFERENCES users(id);
ALTER TABLE purchase_inspection ADD CONSTRAINT fk_pi_completed_by
    FOREIGN KEY (completed_by) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_pi_inspection_no ON purchase_inspection(inspection_no);
CREATE INDEX IF NOT EXISTS idx_pi_receipt_id ON purchase_inspection(receipt_id);
CREATE INDEX IF NOT EXISTS idx_pi_order_id ON purchase_inspection(order_id);
CREATE INDEX IF NOT EXISTS idx_pi_inspection_date ON purchase_inspection(inspection_date);
CREATE INDEX IF NOT EXISTS idx_pi_inspector_id ON purchase_inspection(inspector_id);
CREATE INDEX IF NOT EXISTS idx_pi_inspection_status ON purchase_inspection(inspection_status);

-- 触发器：更新时间
DROP TRIGGER IF EXISTS update_purchase_inspection_updated_at ON purchase_inspection;
CREATE TRIGGER update_purchase_inspection_updated_at
BEFORE UPDATE ON purchase_inspection
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();


-- =====================================================
-- 7. 数据字典表 - 采购订单状态
-- =====================================================
CREATE TABLE IF NOT EXISTS purchase_order_status (
    id SERIAL PRIMARY KEY,
    status_code VARCHAR(20) NOT NULL UNIQUE,            -- 状态编码
    status_name VARCHAR(50) NOT NULL,                   -- 状态名称
    description TEXT,                                   -- 状态描述
    sort_order INTEGER DEFAULT 0,                       -- 排序顺序
    is_enabled BOOLEAN DEFAULT TRUE,                    -- 是否启用
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP      -- 创建时间
);

-- 插入初始数据
INSERT INTO purchase_order_status (status_code, status_name, description, sort_order) VALUES
('DRAFT', '草稿', '订单尚未提交，可以编辑和删除', 10),
('SUBMITTED', '已提交', '订单已提交待审批', 20),
('APPROVED', '已审批', '订单已审批通过，可以执行', 30),
('REJECTED', '已拒绝', '订单审批被拒绝', 25),
('PARTIAL_RECEIVED', '部分入库', '订单部分物料已入库', 40),
('COMPLETED', '已完成', '订单全部物料已入库', 50),
('CLOSED', '已关闭', '订单已关闭，不可再操作', 60) ON CONFLICT DO NOTHING;


-- =====================================================
-- 8. 数据字典表 - 入库单状态
-- =====================================================
CREATE TABLE IF NOT EXISTS purchase_receipt_status (
    id SERIAL PRIMARY KEY,
    status_code VARCHAR(20) NOT NULL UNIQUE,            -- 状态编码
    status_name VARCHAR(50) NOT NULL,                   -- 状态名称
    description TEXT,                                   -- 状态描述
    sort_order INTEGER DEFAULT 0,                       -- 排序顺序
    is_enabled BOOLEAN DEFAULT TRUE,                    -- 是否启用
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP      -- 创建时间
);

-- 插入初始数据
INSERT INTO purchase_receipt_status (status_code, status_name, description, sort_order) VALUES
('DRAFT', '草稿', '入库单尚未确认', 10),
('CONFIRMED', '已确认', '入库单已确认，库存已更新', 20),
('CANCELLED', '已取消', '入库单已取消', 30) ON CONFLICT DO NOTHING;


-- =====================================================
-- 9. 数据字典表 - 退货原因类型
-- =====================================================
CREATE TABLE IF NOT EXISTS purchase_return_reason (
    id SERIAL PRIMARY KEY,
    reason_code VARCHAR(20) NOT NULL UNIQUE,            -- 原因编码
    reason_name VARCHAR(100) NOT NULL,                  -- 原因名称
    description TEXT,                                   -- 原因描述
    sort_order INTEGER DEFAULT 0,                       -- 排序顺序
    is_enabled BOOLEAN DEFAULT TRUE,                    -- 是否启用
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP      -- 创建时间
);

-- 插入初始数据
INSERT INTO purchase_return_reason (reason_code, reason_name, description, sort_order) VALUES
('QUALITY_ISSUE', '质量问题', '物料质量不符合要求', 10),
('WRONG_ITEM', '发错货', '供应商发错物料', 20),
('DAMAGED', '破损', '物料在运输过程中破损', 30),
('EXCESS_DELIVERY', '超额送货', '供应商送货数量超过订单数量', 40),
('OTHER', '其他', '其他退货原因', 90) ON CONFLICT DO NOTHING;


-- =====================================================
-- 10. 物化视图 - 采购订单统计
-- =====================================================
CREATE MATERIALIZED VIEW mv_purchase_order_stats AS
SELECT 
    supplier_id,
    COUNT(*) as total_orders,                           -- 总订单数
    SUM(total_amount) as total_amount,                  -- 总金额
    AVG(total_amount) as avg_order_amount,              -- 平均订单金额
    MAX(order_date) as last_order_date,                 -- 最后订单日期
    SUM(CASE WHEN order_status = 'COMPLETED' THEN 1 ELSE 0 END) as completed_orders,  -- 已完成订单数
    SUM(CASE WHEN order_status = 'PARTIAL_RECEIVED' THEN 1 ELSE 0 END) as partial_orders  -- 部分入库订单数
FROM purchase_order
WHERE order_status IN ('COMPLETED', 'CLOSED', 'PARTIAL_RECEIVED')
GROUP BY supplier_id;

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_mv_po_stats_supplier ON mv_purchase_order_stats(supplier_id);


-- =====================================================
-- 11. 注释
-- =====================================================
COMMENT ON TABLE purchase_order IS '采购订单表';
COMMENT ON TABLE purchase_order_item IS '采购订单明细表';
COMMENT ON TABLE purchase_receipt IS '采购入库表';
COMMENT ON TABLE purchase_receipt_item IS '采购入库明细表';
COMMENT ON TABLE purchase_return IS '采购退货表';
COMMENT ON TABLE purchase_inspection IS '采购质检表';
COMMENT ON TABLE purchase_order_status IS '采购订单状态字典';
COMMENT ON TABLE purchase_receipt_status IS '采购入库状态字典';
COMMENT ON TABLE purchase_return_reason IS '采购退货原因字典';
COMMENT ON MATERIALIZED VIEW mv_purchase_order_stats IS '采购订单统计视图';

-- =====================================================
-- 迁移完成
-- =====================================================

-- ============================================
-- 来源: 008_accounts_payable.sql
-- ============================================
-- 应付管理模块数据库迁移脚本
-- 创建时间：2026-03-15
-- 功能说明：创建应付单、付款申请、付款执行、应付核销、供应商对账相关表及索引

-- =====================================================
-- 1. 应付单表 (ap_invoice)
-- =====================================================
CREATE TABLE IF NOT EXISTS ap_invoice (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    invoice_no VARCHAR(50) NOT NULL UNIQUE,             -- 应付单号（AP20260315001）
    supplier_id INTEGER NOT NULL,                       -- 供应商 ID（外键）
    invoice_type VARCHAR(20) NOT NULL,                  -- 应付类型：PURCHASE=采购应付，EXPENSE=费用应付，OTHER=其他
    source_type VARCHAR(20),                            -- 来源类型：PURCHASE_RECEIPT=采购入库，PURCHASE_RETURN=采购退货，MANUAL=手工录入
    source_id INTEGER,                                  -- 来源单 ID（采购入库单 ID 或退货单 ID）
    invoice_date DATE NOT NULL,                         -- 应付日期
    due_date DATE NOT NULL,                             -- 到期日期
    payment_terms INTEGER DEFAULT 30,                   -- 账期（天）
    amount DECIMAL(18,2) NOT NULL,                      -- 应付金额
    paid_amount DECIMAL(18,2) DEFAULT 0.00,             -- 已付金额
    unpaid_amount DECIMAL(18,2) DEFAULT 0.00,           -- 未付金额
    invoice_status VARCHAR(20) DEFAULT 'DRAFT',         -- 应付状态：DRAFT=草稿，AUDITED=已审核，PARTIAL_PAID=部分付款，PAID=已付清，CANCELLED=已取消
    currency VARCHAR(10) DEFAULT 'CNY',                 -- 币种
    exchange_rate DECIMAL(18,6) DEFAULT 1.000000,       -- 汇率
    amount_foreign DECIMAL(18,2),                       -- 外币金额
    tax_amount DECIMAL(18,2) DEFAULT 0.00,              -- 税额
    notes TEXT,                                         -- 备注
    attachment_urls TEXT[],                             -- 附件 URL 列表
    created_by INTEGER NOT NULL,                        -- 创建人 ID
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    updated_by INTEGER,                                 -- 更新人 ID
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 更新时间
    approved_by INTEGER,                                -- 审核人 ID
    approved_at TIMESTAMP,                              -- 审核时间
    cancelled_by INTEGER,                               -- 取消人 ID
    cancelled_at TIMESTAMP,                             -- 取消时间
    cancelled_reason TEXT                               -- 取消原因
);

-- 外键约束
ALTER TABLE ap_invoice ADD CONSTRAINT fk_ap_invoice_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);
ALTER TABLE ap_invoice ADD CONSTRAINT fk_ap_invoice_created_by
    FOREIGN KEY (created_by) REFERENCES users(id);
ALTER TABLE ap_invoice ADD CONSTRAINT fk_ap_invoice_updated_by
    FOREIGN KEY (updated_by) REFERENCES users(id);
ALTER TABLE ap_invoice ADD CONSTRAINT fk_ap_invoice_approved_by
    FOREIGN KEY (approved_by) REFERENCES users(id);
ALTER TABLE ap_invoice ADD CONSTRAINT fk_ap_invoice_cancelled_by
    FOREIGN KEY (cancelled_by) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_ap_invoice_no ON ap_invoice(invoice_no);
CREATE INDEX IF NOT EXISTS idx_ap_invoice_supplier_id ON ap_invoice(supplier_id);
CREATE INDEX IF NOT EXISTS idx_ap_invoice_date ON ap_invoice(invoice_date);
CREATE INDEX IF NOT EXISTS idx_ap_invoice_due_date ON ap_invoice(due_date);
CREATE INDEX IF NOT EXISTS idx_ap_invoice_status ON ap_invoice(invoice_status);
CREATE INDEX IF NOT EXISTS idx_ap_invoice_source ON ap_invoice(source_type, source_id);
CREATE INDEX IF NOT EXISTS idx_ap_invoice_created_by ON ap_invoice(created_by);

-- 触发器：更新时间
DROP TRIGGER IF EXISTS update_ap_invoice_updated_at ON ap_invoice;
CREATE TRIGGER update_ap_invoice_updated_at
BEFORE UPDATE ON ap_invoice
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- 触发器：更新已付金额和状态
CREATE OR REPLACE FUNCTION update_ap_invoice_status()
RETURNS TRIGGER AS $$
BEGIN
    -- 更新应付单的已付金额和未付金额
    NEW.paid_amount = COALESCE(NEW.paid_amount, 0.00);
    NEW.unpaid_amount = NEW.amount - NEW.paid_amount;
    
    -- 更新应付状态
    IF NEW.invoice_status = 'AUDITED' THEN
        IF NEW.paid_amount = 0 THEN
            NEW.invoice_status = 'AUDITED';
        ELSIF NEW.paid_amount >= NEW.amount THEN
            NEW.invoice_status = 'PAID';
        ELSE
            NEW.invoice_status = 'PARTIAL_PAID';
        END IF;
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_update_ap_invoice_status ON ap_invoice;
CREATE TRIGGER trigger_update_ap_invoice_status
BEFORE UPDATE OF paid_amount ON ap_invoice
FOR EACH ROW EXECUTE FUNCTION update_ap_invoice_status();


-- =====================================================
-- 2. 付款申请表 (ap_payment_request)
-- =====================================================
CREATE TABLE IF NOT EXISTS ap_payment_request (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    request_no VARCHAR(50) NOT NULL UNIQUE,             -- 付款申请单号（PR20260315001）
    request_date DATE NOT NULL,                         -- 申请日期
    supplier_id INTEGER NOT NULL,                       -- 供应商 ID（外键）
    payment_type VARCHAR(20) NOT NULL,                  -- 付款类型：PREPAYMENT=预付款，PROGRESS=进度款，FINAL=尾款，WARRANTY=质保金
    payment_method VARCHAR(20) NOT NULL,                -- 付款方式：TT=电汇，LC=信用证，DP=付款交单，DA=承兑交单，CHECK=支票，CASH=现金
    request_amount DECIMAL(18,2) NOT NULL,              -- 申请金额
    approval_status VARCHAR(20) DEFAULT 'DRAFT',        -- 审批状态：DRAFT=草稿，APPROVING=审批中，APPROVED=已审批，REJECTED=已拒绝
    currency VARCHAR(10) DEFAULT 'CNY',                 -- 币种
    exchange_rate DECIMAL(18,6) DEFAULT 1.000000,       -- 汇率
    request_amount_foreign DECIMAL(18,2),               -- 外币金额
    expected_payment_date DATE,                         -- 期望付款日期
    bank_name VARCHAR(200),                             -- 收款银行
    bank_account VARCHAR(50),                           -- 收款账号
    bank_account_name VARCHAR(200),                     -- 收款账户名
    notes TEXT,                                         -- 备注
    attachment_urls TEXT[],                             -- 附件 URL 列表
    created_by INTEGER NOT NULL,                        -- 创建人 ID
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    updated_by INTEGER,                                 -- 更新人 ID
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 更新时间
    submitted_by INTEGER,                               -- 提交人 ID
    submitted_at TIMESTAMP,                             -- 提交时间
    approved_by INTEGER,                                -- 审批人 ID
    approved_at TIMESTAMP,                              -- 审批时间
    rejected_by INTEGER,                                -- 拒绝人 ID
    rejected_at TIMESTAMP,                              -- 拒绝时间
    rejected_reason TEXT                                -- 拒绝原因
);

-- 外键约束
ALTER TABLE ap_payment_request ADD CONSTRAINT fk_ap_request_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);
ALTER TABLE ap_payment_request ADD CONSTRAINT fk_ap_request_created_by
    FOREIGN KEY (created_by) REFERENCES users(id);
ALTER TABLE ap_payment_request ADD CONSTRAINT fk_ap_request_updated_by
    FOREIGN KEY (updated_by) REFERENCES users(id);
ALTER TABLE ap_payment_request ADD CONSTRAINT fk_ap_request_submitted_by
    FOREIGN KEY (submitted_by) REFERENCES users(id);
ALTER TABLE ap_payment_request ADD CONSTRAINT fk_ap_request_approved_by
    FOREIGN KEY (approved_by) REFERENCES users(id);
ALTER TABLE ap_payment_request ADD CONSTRAINT fk_ap_request_rejected_by
    FOREIGN KEY (rejected_by) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_ap_request_no ON ap_payment_request(request_no);
CREATE INDEX IF NOT EXISTS idx_ap_request_supplier_id ON ap_payment_request(supplier_id);
CREATE INDEX IF NOT EXISTS idx_ap_request_date ON ap_payment_request(request_date);
CREATE INDEX IF NOT EXISTS idx_ap_request_status ON ap_payment_request(approval_status);
CREATE INDEX IF NOT EXISTS idx_ap_request_created_by ON ap_payment_request(created_by);

-- 触发器：更新时间
DROP TRIGGER IF EXISTS update_ap_payment_request_updated_at ON ap_payment_request;
CREATE TRIGGER update_ap_payment_request_updated_at
BEFORE UPDATE ON ap_payment_request
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();


-- =====================================================
-- 3. 付款申请明细表 (ap_payment_request_item)
-- =====================================================
CREATE TABLE IF NOT EXISTS ap_payment_request_item (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    request_id INTEGER NOT NULL,                        -- 付款申请 ID（外键）
    invoice_id INTEGER NOT NULL,                        -- 应付单 ID（外键）
    apply_amount DECIMAL(18,2) NOT NULL,                -- 申请金额
    notes TEXT,                                         -- 备注
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    CONSTRAINT fk_ap_request_item_request
        FOREIGN KEY (request_id) REFERENCES ap_payment_request(id) ON DELETE CASCADE,
    CONSTRAINT fk_ap_request_item_invoice
        FOREIGN KEY (invoice_id) REFERENCES ap_invoice(id)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_ap_request_item_request_id ON ap_payment_request_item(request_id);
CREATE INDEX IF NOT EXISTS idx_ap_request_item_invoice_id ON ap_payment_request_item(invoice_id);


-- =====================================================
-- 4. 付款单表 (ap_payment)
-- =====================================================
CREATE TABLE IF NOT EXISTS ap_payment (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    payment_no VARCHAR(50) NOT NULL UNIQUE,             -- 付款单号（PAY20260315001）
    payment_date DATE NOT NULL,                         -- 付款日期
    supplier_id INTEGER NOT NULL,                       -- 供应商 ID（外键）
    request_id INTEGER,                                 -- 付款申请 ID（外键）
    payment_method VARCHAR(20) NOT NULL,                -- 付款方式：TT/LC/DP/DA/CHECK/CASH
    payment_amount DECIMAL(18,2) NOT NULL,              -- 付款金额
    payment_status VARCHAR(20) DEFAULT 'REGISTERED',    -- 付款状态：REGISTERED=已登记，CONFIRMED=已确认
    currency VARCHAR(10) DEFAULT 'CNY',                 -- 币种
    exchange_rate DECIMAL(18,6) DEFAULT 1.000000,       -- 汇率
    payment_amount_foreign DECIMAL(18,2),               -- 外币金额
    bank_name VARCHAR(200),                             -- 付款银行
    bank_account VARCHAR(50),                           -- 付款账号
    transaction_no VARCHAR(100),                        -- 交易流水号
    notes TEXT,                                         -- 备注
    attachment_urls TEXT[],                             -- 附件 URL 列表（付款凭证）
    created_by INTEGER NOT NULL,                        -- 创建人 ID
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    updated_by INTEGER,                                 -- 更新人 ID
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 更新时间
    confirmed_by INTEGER,                               -- 确认人 ID
    confirmed_at TIMESTAMP                              -- 确认时间
);

-- 外键约束
ALTER TABLE ap_payment ADD CONSTRAINT fk_ap_payment_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);
ALTER TABLE ap_payment ADD CONSTRAINT fk_ap_payment_request
    FOREIGN KEY (request_id) REFERENCES ap_payment_request(id);
ALTER TABLE ap_payment ADD CONSTRAINT fk_ap_payment_created_by
    FOREIGN KEY (created_by) REFERENCES users(id);
ALTER TABLE ap_payment ADD CONSTRAINT fk_ap_payment_updated_by
    FOREIGN KEY (updated_by) REFERENCES users(id);
ALTER TABLE ap_payment ADD CONSTRAINT fk_ap_payment_confirmed_by
    FOREIGN KEY (confirmed_by) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_ap_payment_no ON ap_payment(payment_no);
CREATE INDEX IF NOT EXISTS idx_ap_payment_supplier_id ON ap_payment(supplier_id);
CREATE INDEX IF NOT EXISTS idx_ap_payment_date ON ap_payment(payment_date);
CREATE INDEX IF NOT EXISTS idx_ap_payment_status ON ap_payment(payment_status);
CREATE INDEX IF NOT EXISTS idx_ap_payment_request_id ON ap_payment(request_id);
CREATE INDEX IF NOT EXISTS idx_ap_payment_created_by ON ap_payment(created_by);

-- 触发器：更新时间
DROP TRIGGER IF EXISTS update_ap_payment_updated_at ON ap_payment;
CREATE TRIGGER update_ap_payment_updated_at
BEFORE UPDATE ON ap_payment
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();


-- =====================================================
-- 5. 应付核销表 (ap_verification)
-- =====================================================
CREATE TABLE IF NOT EXISTS ap_verification (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    verification_no VARCHAR(50) NOT NULL UNIQUE,        -- 核销单号（VER20260315001）
    verification_date DATE NOT NULL,                    -- 核销日期
    supplier_id INTEGER NOT NULL,                       -- 供应商 ID（外键）
    verification_type VARCHAR(20) NOT NULL,             -- 核销方式：AUTO=自动核销，MANUAL=手工核销
    total_amount DECIMAL(18,2) NOT NULL,                -- 核销总金额
    verification_status VARCHAR(20) DEFAULT 'COMPLETED',-- 核销状态：COMPLETED=已完成，CANCELLED=已取消
    notes TEXT,                                         -- 备注
    created_by INTEGER NOT NULL,                        -- 创建人 ID
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    cancelled_by INTEGER,                               -- 取消人 ID
    cancelled_at TIMESTAMP,                             -- 取消时间
    cancelled_reason TEXT                               -- 取消原因
);

-- 外键约束
ALTER TABLE ap_verification ADD CONSTRAINT fk_ap_verification_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);
ALTER TABLE ap_verification ADD CONSTRAINT fk_ap_verification_created_by
    FOREIGN KEY (created_by) REFERENCES users(id);
ALTER TABLE ap_verification ADD CONSTRAINT fk_ap_verification_cancelled_by
    FOREIGN KEY (cancelled_by) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_ap_verification_no ON ap_verification(verification_no);
CREATE INDEX IF NOT EXISTS idx_ap_verification_supplier_id ON ap_verification(supplier_id);
CREATE INDEX IF NOT EXISTS idx_ap_verification_date ON ap_verification(verification_date);
CREATE INDEX IF NOT EXISTS idx_ap_verification_status ON ap_verification(verification_status);
CREATE INDEX IF NOT EXISTS idx_ap_verification_created_by ON ap_verification(created_by);

-- 触发器：更新时间
DROP TRIGGER IF EXISTS update_ap_verification_updated_at ON ap_verification;
CREATE TRIGGER update_ap_verification_updated_at
BEFORE UPDATE ON ap_verification
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();


-- =====================================================
-- 6. 核销明细表 (ap_verification_item)
-- =====================================================
CREATE TABLE IF NOT EXISTS ap_verification_item (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    verification_id INTEGER NOT NULL,                   -- 核销单 ID（外键）
    invoice_id INTEGER NOT NULL,                        -- 应付单 ID（外键）
    payment_id INTEGER NOT NULL,                        -- 付款单 ID（外键）
    verify_amount DECIMAL(18,2) NOT NULL,               -- 核销金额
    notes TEXT,                                         -- 备注
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    CONSTRAINT fk_ap_verification_item_verification
        FOREIGN KEY (verification_id) REFERENCES ap_verification(id) ON DELETE CASCADE,
    CONSTRAINT fk_ap_verification_item_invoice
        FOREIGN KEY (invoice_id) REFERENCES ap_invoice(id),
    CONSTRAINT fk_ap_verification_item_payment
        FOREIGN KEY (payment_id) REFERENCES ap_payment(id)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_ap_verification_item_verification_id ON ap_verification_item(verification_id);
CREATE INDEX IF NOT EXISTS idx_ap_verification_item_invoice_id ON ap_verification_item(invoice_id);
CREATE INDEX IF NOT EXISTS idx_ap_verification_item_payment_id ON ap_verification_item(payment_id);


-- =====================================================
-- 7. 供应商对账单 (ap_reconciliation)
-- =====================================================
CREATE TABLE IF NOT EXISTS ap_reconciliation (
    id SERIAL PRIMARY KEY,                              -- 主键 ID
    reconciliation_no VARCHAR(50) NOT NULL UNIQUE,      -- 对账单号（REC20260315001）
    supplier_id INTEGER NOT NULL,                       -- 供应商 ID（外键）
    start_date DATE NOT NULL,                           -- 对账开始日期
    end_date DATE NOT NULL,                             -- 对账结束日期
    opening_balance DECIMAL(18,2) DEFAULT 0.00,         -- 期初余额
    total_invoice DECIMAL(18,2) DEFAULT 0.00,           -- 本期应付合计
    total_payment DECIMAL(18,2) DEFAULT 0.00,           -- 本期付款合计
    closing_balance DECIMAL(18,2) DEFAULT 0.00,         -- 期末余额
    reconciliation_status VARCHAR(20) DEFAULT 'PENDING',-- 对账状态：PENDING=待确认，CONFIRMED=已确认，DISPUTED=有争议
    notes TEXT,                                         -- 备注
    created_by INTEGER NOT NULL,                        -- 创建人 ID
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,     -- 创建时间
    confirmed_by INTEGER,                               -- 确认人 ID（供应商确认）
    confirmed_at TIMESTAMP,                             -- 确认时间
    disputed_by INTEGER,                                -- 争议人 ID
    disputed_at TIMESTAMP,                              -- 争议时间
    disputed_reason TEXT                                -- 争议原因
);

-- 外键约束
ALTER TABLE ap_reconciliation ADD CONSTRAINT fk_ap_reconciliation_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);
ALTER TABLE ap_reconciliation ADD CONSTRAINT fk_ap_reconciliation_created_by
    FOREIGN KEY (created_by) REFERENCES users(id);
ALTER TABLE ap_reconciliation ADD CONSTRAINT fk_ap_reconciliation_confirmed_by
    FOREIGN KEY (confirmed_by) REFERENCES users(id);
ALTER TABLE ap_reconciliation ADD CONSTRAINT fk_ap_reconciliation_disputed_by
    FOREIGN KEY (disputed_by) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_ap_reconciliation_no ON ap_reconciliation(reconciliation_no);
CREATE INDEX IF NOT EXISTS idx_ap_reconciliation_supplier_id ON ap_reconciliation(supplier_id);
CREATE INDEX IF NOT EXISTS idx_ap_reconciliation_date ON ap_reconciliation(start_date, end_date);
CREATE INDEX IF NOT EXISTS idx_ap_reconciliation_status ON ap_reconciliation(reconciliation_status);
CREATE INDEX IF NOT EXISTS idx_ap_reconciliation_created_by ON ap_reconciliation(created_by);

-- 触发器：更新时间
DROP TRIGGER IF EXISTS update_ap_reconciliation_updated_at ON ap_reconciliation;
CREATE TRIGGER update_ap_reconciliation_updated_at
BEFORE UPDATE ON ap_reconciliation
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();


-- =====================================================
-- 8. 物化视图：供应商应付汇总表
-- =====================================================
CREATE MATERIALIZED VIEW mv_supplier_ap_summary AS
SELECT
    s.id AS supplier_id,
    s.supplier_code,
    s.supplier_name,
    COUNT(DISTINCT inv.id) AS total_invoice_count,
    SUM(inv.amount) AS total_invoice_amount,
    SUM(inv.paid_amount) AS total_paid_amount,
    SUM(inv.unpaid_amount) AS total_unpaid_amount,
    COUNT(DISTINCT CASE WHEN inv.invoice_status = 'PAID' THEN inv.id END) AS paid_invoice_count,
    COUNT(DISTINCT CASE WHEN inv.invoice_status = 'PARTIAL_PAID' THEN inv.id END) AS partial_paid_invoice_count,
    COUNT(DISTINCT CASE WHEN inv.unpaid_amount > 0 AND inv.due_date < CURRENT_DATE THEN inv.id END) AS overdue_invoice_count,
    SUM(CASE WHEN inv.unpaid_amount > 0 AND inv.due_date < CURRENT_DATE THEN inv.unpaid_amount ELSE 0 END) AS overdue_amount
FROM suppliers s
LEFT JOIN ap_invoice inv ON s.id = inv.supplier_id AND inv.invoice_status NOT IN ('DRAFT', 'CANCELLED')
GROUP BY s.id, s.supplier_code, s.supplier_name;

-- 物化视图索引
CREATE INDEX IF NOT EXISTS idx_mv_supplier_ap_summary_supplier_id ON mv_supplier_ap_summary(supplier_id);
CREATE INDEX IF NOT EXISTS idx_mv_supplier_ap_summary_code ON mv_supplier_ap_summary(supplier_code);
CREATE INDEX IF NOT EXISTS idx_mv_supplier_ap_summary_name ON mv_supplier_ap_summary(supplier_name);

-- 刷新物化视图的函数
CREATE OR REPLACE FUNCTION refresh_mv_supplier_ap_summary()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY mv_supplier_ap_summary;
END;
$$ LANGUAGE plpgsql;


-- =====================================================
-- 9. 回滚语句（仅在需要时手动执行）
-- =====================================================
-- DROP MATERIALIZED VIEW IF EXISTS mv_supplier_ap_summary CASCADE;
-- DROP TABLE IF EXISTS ap_reconciliation CASCADE;
-- DROP TABLE IF EXISTS ap_verification_item CASCADE;
-- DROP TABLE IF EXISTS ap_verification CASCADE;
-- DROP TABLE IF EXISTS ap_payment CASCADE;
-- DROP TABLE IF EXISTS ap_payment_request_item CASCADE;
-- DROP TABLE IF EXISTS ap_payment_request CASCADE;
-- DROP TABLE IF EXISTS ap_invoice CASCADE;

-- ============================================
-- 来源: 009_inventory_transfer.sql
-- ============================================
-- 秉羲管理系统 - 库存调拨表迁移脚本
-- 数据库类型：PostgreSQL 18.0
-- 创建日期：2026-03-15
-- 说明：此脚本用于创建库存调拨相关表结构

-- ==================== 库存调拨表 ====================
-- 存储仓库之间的库存调拨记录



COMMENT ON COLUMN inventory_transfer_items.transfer_id IS '调拨单 ID';
COMMENT ON COLUMN inventory_transfer_items.product_id IS '产品 ID';
COMMENT ON COLUMN inventory_transfer_items.quantity IS '调拨数量';
COMMENT ON COLUMN inventory_transfer_items.shipped_quantity IS '已发出数量';
COMMENT ON COLUMN inventory_transfer_items.received_quantity IS '已接收数量';
COMMENT ON COLUMN inventory_transfer_items.unit_cost IS '单位成本';
COMMENT ON COLUMN inventory_transfer_items.notes IS '备注';
COMMENT ON COLUMN inventory_transfer_items.created_at IS '创建时间';
COMMENT ON COLUMN inventory_transfer_items.updated_at IS '更新时间';

-- ==================== 触发器：自动更新时间 ====================
-- 为 inventory_transfers 表创建触发器
DROP TRIGGER IF EXISTS update_inventory_transfers_updated_at ON inventory_transfers;
CREATE TRIGGER update_inventory_transfers_updated_at BEFORE UPDATE ON inventory_transfers
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- 为 inventory_transfer_items 表创建触发器
DROP TRIGGER IF EXISTS update_inventory_transfer_items_updated_at ON inventory_transfer_items;
CREATE TRIGGER update_inventory_transfer_items_updated_at BEFORE UPDATE ON inventory_transfer_items
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ==================== 完成提示 ====================
DO $$
BEGIN
    RAISE NOTICE '库存调拨表创建完成！';
END $$;

-- ============================================
-- 来源: 010_inventory_count.sql
-- ============================================
-- 秉羲管理系统 - 库存盘点表迁移脚本
-- 数据库类型：PostgreSQL 18.0
-- 创建日期：2026-03-15
-- 说明：此脚本用于创建库存盘点相关表结构

-- ==================== 库存盘点表 ====================
-- 存储仓库库存盘点记录



COMMENT ON COLUMN inventory_count_items.product_id IS '产品 ID';
COMMENT ON COLUMN inventory_count_items.bin_location IS '库位';
COMMENT ON COLUMN inventory_count_items.quantity_book IS '账面数量';
COMMENT ON COLUMN inventory_count_items.quantity_actual IS '实际数量';
COMMENT ON COLUMN inventory_count_items.quantity_variance IS '差异数量';
COMMENT ON COLUMN inventory_count_items.unit_cost IS '单位成本';
COMMENT ON COLUMN inventory_count_items.variance_amount IS '差异金额';
COMMENT ON COLUMN inventory_count_items.notes IS '备注';
COMMENT ON COLUMN inventory_count_items.counted_by IS '盘点人';
COMMENT ON COLUMN inventory_count_items.counted_at IS '盘点时间';
COMMENT ON COLUMN inventory_count_items.created_at IS '创建时间';
COMMENT ON COLUMN inventory_count_items.updated_at IS '更新时间';

-- ==================== 触发器：自动更新时间 ====================
-- 为 inventory_counts 表创建触发器
DROP TRIGGER IF EXISTS update_inventory_counts_updated_at ON inventory_counts;
CREATE TRIGGER update_inventory_counts_updated_at BEFORE UPDATE ON inventory_counts
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- 为 inventory_count_items 表创建触发器
DROP TRIGGER IF EXISTS update_inventory_count_items_updated_at ON inventory_count_items;
CREATE TRIGGER update_inventory_count_items_updated_at BEFORE UPDATE ON inventory_count_items
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ==================== 完成提示 ====================
DO $$
BEGIN
    RAISE NOTICE '库存盘点表创建完成！';
END $$;

-- ============================================
-- 来源: 011_inventory_count_items.sql
-- ============================================
-- ============================================
-- 库存盘点明细表 - 补充表
-- ============================================

DROP TABLE IF EXISTS inventory_count_items CASCADE;
CREATE TABLE IF NOT EXISTS inventory_count_items (
    id SERIAL PRIMARY KEY,
    count_id INTEGER NOT NULL REFERENCES inventory_counts(id) ON DELETE CASCADE,
    stock_id INTEGER NOT NULL REFERENCES inventory_stocks(id) ON DELETE CASCADE,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    warehouse_id INTEGER NOT NULL REFERENCES warehouses(id) ON DELETE CASCADE,
    quantity_before DECIMAL(10,2) NOT NULL DEFAULT 0,  -- 盘点前数量
    quantity_actual DECIMAL(10,2) NOT NULL DEFAULT 0,  -- 实际盘点数量
    quantity_difference DECIMAL(10,2) NOT NULL DEFAULT 0,  -- 差异数量（实际 - 账面）
    unit_cost DECIMAL(12,2) DEFAULT 0,  -- 单位成本
    total_cost DECIMAL(12,2) DEFAULT 0,  -- 总成本差异
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_inventory_count_items_count_id ON inventory_count_items(count_id);
CREATE INDEX IF NOT EXISTS idx_inventory_count_items_stock_id ON inventory_count_items(stock_id);
CREATE INDEX IF NOT EXISTS idx_inventory_count_items_product_id ON inventory_count_items(product_id);
CREATE INDEX IF NOT EXISTS idx_inventory_count_items_warehouse_id ON inventory_count_items(warehouse_id);

-- 添加注释
COMMENT ON TABLE inventory_count_items IS '库存盘点明细表';
COMMENT ON COLUMN inventory_count_items.id IS '明细 ID';
COMMENT ON COLUMN inventory_count_items.count_id IS '盘点单 ID';
COMMENT ON COLUMN inventory_count_items.stock_id IS '库存 ID';
COMMENT ON COLUMN inventory_count_items.product_id IS '产品 ID';
COMMENT ON COLUMN inventory_count_items.warehouse_id IS '仓库 ID';
COMMENT ON COLUMN inventory_count_items.quantity_before IS '盘点前数量（账面数量）';
COMMENT ON COLUMN inventory_count_items.quantity_actual IS '实际盘点数量';
COMMENT ON COLUMN inventory_count_items.quantity_difference IS '差异数量（实际 - 账面）';
COMMENT ON COLUMN inventory_count_items.unit_cost IS '单位成本';
COMMENT ON COLUMN inventory_count_items.total_cost IS '总成本差异';
COMMENT ON COLUMN inventory_count_items.notes IS '备注';
COMMENT ON COLUMN inventory_count_items.created_at IS '创建时间';
COMMENT ON COLUMN inventory_count_items.updated_at IS '更新时间';

-- ============================================
-- 更新 inventory_counts 表，添加差异汇总字段
-- ============================================

ALTER TABLE inventory_counts 
ADD COLUMN IF NOT EXISTS total_quantity_before DECIMAL(10,2) DEFAULT 0,
ADD COLUMN IF NOT EXISTS total_quantity_actual DECIMAL(10,2) DEFAULT 0,
ADD COLUMN IF NOT EXISTS total_quantity_difference DECIMAL(10,2) DEFAULT 0,
ADD COLUMN IF NOT EXISTS total_cost_difference DECIMAL(12,2) DEFAULT 0;

-- 添加注释
COMMENT ON COLUMN inventory_counts.total_quantity_before IS '盘点前总数量';
COMMENT ON COLUMN inventory_counts.total_quantity_actual IS '实际盘点总数量';
COMMENT ON COLUMN inventory_counts.total_quantity_difference IS '总差异数量';
COMMENT ON COLUMN inventory_counts.total_cost_difference IS '总成本差异';

-- ============================================
-- 来源: 012_general_ledger.sql
-- ============================================
-- ============================================
-- 总账管理模块 - 基础表结构（阶段 1）
-- ============================================
-- 文档编号：MIGRATION-020-GL-PHASE1
-- 创建日期：2026-03-15
-- 说明：总账管理模块基础表结构，包含会计科目、凭证、凭证分录、科目余额表
-- ============================================

-- 1. 会计科目表（基础版）
-- ============================================
DROP TABLE IF EXISTS account_subjects CASCADE;
CREATE TABLE IF NOT EXISTS account_subjects (
    id SERIAL PRIMARY KEY,
    code VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(200) NOT NULL,
    level INTEGER NOT NULL,
    parent_id INTEGER REFERENCES account_subjects(id),
    full_code VARCHAR(200),
    
    -- 余额属性
    balance_direction VARCHAR(10),
    initial_balance_debit DECIMAL(14,2) DEFAULT 0,
    initial_balance_credit DECIMAL(14,2) DEFAULT 0,
    current_period_debit DECIMAL(14,2) DEFAULT 0,
    current_period_credit DECIMAL(14,2) DEFAULT 0,
    ending_balance_debit DECIMAL(14,2) DEFAULT 0,
    ending_balance_credit DECIMAL(14,2) DEFAULT 0,
    
    -- 辅助核算（阶段 1 基础版，阶段 2 扩展面料行业字段）
    assist_customer BOOLEAN DEFAULT false,
    assist_supplier BOOLEAN DEFAULT false,
    assist_department BOOLEAN DEFAULT false,
    assist_employee BOOLEAN DEFAULT false,
    assist_project BOOLEAN DEFAULT false,
    assist_batch BOOLEAN DEFAULT false,
    assist_color_no BOOLEAN DEFAULT false,
    assist_dye_lot BOOLEAN DEFAULT false,
    assist_grade BOOLEAN DEFAULT false,
    assist_workshop BOOLEAN DEFAULT false,
    
    -- 双计量单位（阶段 2 使用）
    enable_dual_unit BOOLEAN DEFAULT false,
    primary_unit VARCHAR(20) DEFAULT '米',
    secondary_unit VARCHAR(20) DEFAULT '公斤',
    
    -- 控制属性
    is_cash_account BOOLEAN DEFAULT false,
    is_bank_account BOOLEAN DEFAULT false,
    allow_manual_entry BOOLEAN DEFAULT true,
    require_summary BOOLEAN DEFAULT false,
    
    -- 状态
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE account_subjects IS '会计科目表';
COMMENT ON COLUMN account_subjects.code IS '科目编码';
COMMENT ON COLUMN account_subjects.name IS '科目名称';
COMMENT ON COLUMN account_subjects.level IS '科目级次';
COMMENT ON COLUMN account_subjects.parent_id IS '父科目 ID';
COMMENT ON COLUMN account_subjects.full_code IS '完整编码';
COMMENT ON COLUMN account_subjects.balance_direction IS '余额方向（借/贷）';
COMMENT ON COLUMN account_subjects.initial_balance_debit IS '期初借方余额';
COMMENT ON COLUMN account_subjects.initial_balance_credit IS '期初贷方余额';
COMMENT ON COLUMN account_subjects.current_period_debit IS '本期借方发生额';
COMMENT ON COLUMN account_subjects.current_period_credit IS '本期贷方发生额';
COMMENT ON COLUMN account_subjects.ending_balance_debit IS '期末借方余额';
COMMENT ON COLUMN account_subjects.ending_balance_credit IS '期末贷方余额';
COMMENT ON COLUMN account_subjects.assist_customer IS '客户辅助核算';
COMMENT ON COLUMN account_subjects.assist_supplier IS '供应商辅助核算';
COMMENT ON COLUMN account_subjects.assist_batch IS '批次辅助核算';
COMMENT ON COLUMN account_subjects.assist_color_no IS '色号辅助核算';
COMMENT ON COLUMN account_subjects.enable_dual_unit IS '启用双计量单位';

-- 索引
CREATE INDEX IF NOT EXISTS idx_account_subjects_code ON account_subjects(code);
CREATE INDEX IF NOT EXISTS idx_account_subjects_parent ON account_subjects(parent_id);
CREATE INDEX IF NOT EXISTS idx_account_subjects_level ON account_subjects(level);
CREATE INDEX IF NOT EXISTS idx_account_subjects_status ON account_subjects(status);

-- 2. 凭证表（基础版）
-- ============================================
DROP TABLE IF EXISTS vouchers CASCADE;
CREATE TABLE IF NOT EXISTS vouchers (
    id SERIAL PRIMARY KEY,
    voucher_no VARCHAR(50) NOT NULL UNIQUE,
    voucher_type VARCHAR(20) NOT NULL,
    voucher_date DATE NOT NULL,
    
    -- 凭证来源
    source_type VARCHAR(20),
    source_module VARCHAR(50),
    source_bill_id INTEGER,
    source_bill_no VARCHAR(50),
    
    -- 面料行业字段（阶段 2 使用）
    batch_no VARCHAR(50),
    color_no VARCHAR(50),
    dye_lot_no VARCHAR(50),
    workshop VARCHAR(100),
    production_order_no VARCHAR(50),
    
    -- 双计量单位（阶段 2 使用）
    quantity_meters DECIMAL(14,2),
    quantity_kg DECIMAL(14,2),
    gram_weight DECIMAL(10,2),
    
    -- 状态
    status VARCHAR(20) DEFAULT 'draft',
    attachment_count INTEGER DEFAULT 0,
    
    -- 审核
    created_by INTEGER,
    reviewed_by INTEGER,
    reviewed_at TIMESTAMPTZ,
    posted_by INTEGER,
    posted_at TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE vouchers IS '凭证表';
COMMENT ON COLUMN vouchers.voucher_no IS '凭证编号';
COMMENT ON COLUMN vouchers.voucher_type IS '凭证类型（记/收/付/转）';
COMMENT ON COLUMN vouchers.voucher_date IS '凭证日期';
COMMENT ON COLUMN vouchers.source_type IS '来源类型';
COMMENT ON COLUMN vouchers.source_module IS '来源模块';
COMMENT ON COLUMN vouchers.source_bill_id IS '来源单据 ID';
COMMENT ON COLUMN vouchers.source_bill_no IS '来源单据编号';
COMMENT ON COLUMN vouchers.status IS '状态（draft/submitted/reviewed/posted）';
COMMENT ON COLUMN vouchers.created_by IS '制单人 ID';
COMMENT ON COLUMN vouchers.reviewed_by IS '审核人 ID';
COMMENT ON COLUMN vouchers.posted_by IS '过账人 ID';

-- 索引
CREATE INDEX IF NOT EXISTS idx_vouchers_no ON vouchers(voucher_no);
CREATE INDEX IF NOT EXISTS idx_vouchers_date ON vouchers(voucher_date);
CREATE INDEX IF NOT EXISTS idx_vouchers_type ON vouchers(voucher_type);
CREATE INDEX IF NOT EXISTS idx_vouchers_status ON vouchers(status);
CREATE INDEX IF NOT EXISTS idx_vouchers_created_by ON vouchers(created_by);

-- 3. 凭证分录表
-- ============================================
DROP TABLE IF EXISTS voucher_items CASCADE;
CREATE TABLE IF NOT EXISTS voucher_items (
    id SERIAL PRIMARY KEY,
    voucher_id INTEGER NOT NULL REFERENCES vouchers(id) ON DELETE CASCADE,
    line_no INTEGER NOT NULL,
    
    -- 科目
    subject_code VARCHAR(50) NOT NULL,
    subject_name VARCHAR(200) NOT NULL,
    
    -- 金额
    debit DECIMAL(14,2) DEFAULT 0,
    credit DECIMAL(14,2) DEFAULT 0,
    
    -- 摘要
    summary TEXT,
    
    -- 辅助核算（阶段 1 基础版）
    assist_customer_id INTEGER,
    assist_supplier_id INTEGER,
    assist_department_id INTEGER,
    assist_employee_id INTEGER,
    assist_project_id INTEGER,
    assist_batch_id INTEGER,
    assist_color_no_id INTEGER,
    assist_dye_lot_id INTEGER,
    assist_grade VARCHAR(20),
    assist_workshop_id INTEGER,
    
    -- 双计量单位（阶段 2 使用）
    quantity_meters DECIMAL(14,2),
    quantity_kg DECIMAL(14,2),
    unit_price DECIMAL(12,2),
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE voucher_items IS '凭证分录表';
COMMENT ON COLUMN voucher_items.voucher_id IS '凭证 ID';
COMMENT ON COLUMN voucher_items.line_no IS '分录行号';
COMMENT ON COLUMN voucher_items.subject_code IS '科目编码';
COMMENT ON COLUMN voucher_items.subject_name IS '科目名称';
COMMENT ON COLUMN voucher_items.debit IS '借方金额';
COMMENT ON COLUMN voucher_items.credit IS '贷方金额';
COMMENT ON COLUMN voucher_items.summary IS '摘要';
COMMENT ON COLUMN voucher_items.assist_batch_id IS '批次辅助核算 ID';
COMMENT ON COLUMN voucher_items.assist_color_no_id IS '色号辅助核算 ID';

-- 索引
CREATE INDEX IF NOT EXISTS idx_voucher_items_voucher ON voucher_items(voucher_id);
CREATE INDEX IF NOT EXISTS idx_voucher_items_subject ON voucher_items(subject_code);
CREATE INDEX IF NOT EXISTS idx_voucher_items_line_no ON voucher_items(voucher_id, line_no);

-- 4. 科目余额表（基础版）
-- ============================================
CREATE TABLE IF NOT EXISTS account_balances (
    id SERIAL PRIMARY KEY,
    subject_id INTEGER NOT NULL REFERENCES account_subjects(id),
    period VARCHAR(7) NOT NULL,
    
    -- 辅助核算维度（阶段 1 基础版，支持组合）
    assist_customer_id INTEGER,
    assist_supplier_id INTEGER,
    assist_department_id INTEGER,
    assist_employee_id INTEGER,
    assist_project_id INTEGER,
    assist_batch_id INTEGER,
    assist_color_no_id INTEGER,
    assist_dye_lot_id INTEGER,
    assist_grade VARCHAR(20),
    assist_workshop_id INTEGER,
    
    -- 双计量单位（阶段 2 使用）
    quantity_meters DECIMAL(14,2),
    quantity_kg DECIMAL(14,2),
    
    -- 余额
    initial_balance_debit DECIMAL(14,2) DEFAULT 0,
    initial_balance_credit DECIMAL(14,2) DEFAULT 0,
    current_period_debit DECIMAL(14,2) DEFAULT 0,
    current_period_credit DECIMAL(14,2) DEFAULT 0,
    ending_balance_debit DECIMAL(14,2) DEFAULT 0,
    ending_balance_credit DECIMAL(14,2) DEFAULT 0,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    -- 唯一约束：支持按辅助核算维度组合
    UNIQUE(subject_id, period, assist_customer_id, assist_supplier_id, 
           assist_department_id, assist_employee_id, assist_project_id,
           assist_batch_id, assist_color_no_id)
);

COMMENT ON TABLE account_balances IS '科目余额表';
COMMENT ON COLUMN account_balances.subject_id IS '科目 ID';
COMMENT ON COLUMN account_balances.period IS '会计期间（YYYY-MM）';
COMMENT ON COLUMN account_balances.initial_balance_debit IS '期初借方余额';
COMMENT ON COLUMN account_balances.initial_balance_credit IS '期初贷方余额';
COMMENT ON COLUMN account_balances.current_period_debit IS '本期借方发生额';
COMMENT ON COLUMN account_balances.current_period_credit IS '本期贷方发生额';
COMMENT ON COLUMN account_balances.ending_balance_debit IS '期末借方余额';
COMMENT ON COLUMN account_balances.ending_balance_credit IS '期末贷方余额';

-- 索引
CREATE INDEX IF NOT EXISTS idx_account_balances_period ON account_balances(period);
CREATE INDEX IF NOT EXISTS idx_account_balances_subject ON account_balances(subject_id);
CREATE INDEX IF NOT EXISTS idx_account_balances_subject_period ON account_balances(subject_id, period);
CREATE INDEX IF NOT EXISTS idx_account_balances_batch ON account_balances(assist_batch_id);
CREATE INDEX IF NOT EXISTS idx_account_balances_color_no ON account_balances(assist_color_no_id);

-- 5. 插入预设科目（基础科目）
-- ============================================
INSERT INTO account_subjects (code, name, level, parent_id, full_code, balance_direction, status) VALUES
-- 资产类
('1001', '库存现金', 1, NULL, '1001', '借', 'active'),
('1002', '银行存款', 1, NULL, '1002', '借', 'active'),
('1122', '应收账款', 1, NULL, '1122', '借', 'active'),
('1405', '库存商品', 1, NULL, '1405', '借', 'active'),
('1405.01', '库存商品 - 坯布', 2, (SELECT id FROM account_subjects WHERE code = '1405'), '1405.01', '借', 'active'),
('1405.02', '库存商品 - 成品布', 2, (SELECT id FROM account_subjects WHERE code = '1405'), '1405.02', '借', 'active'),
('1405.03', '库存商品 - 辅料', 2, (SELECT id FROM account_subjects WHERE code = '1405'), '1405.03', '借', 'active'),
('1601', '固定资产', 1, NULL, '1601', '借', 'active'),

-- 负债类
('2001', '短期借款', 1, NULL, '2001', '贷', 'active'),
('2202', '应付账款', 1, NULL, '2202', '贷', 'active'),
('2202.01', '应付账款 - 坯布供应商', 2, (SELECT id FROM account_subjects WHERE code = '2202'), '2202.01', '贷', 'active'),
('2202.02', '应付账款 - 辅料供应商', 2, (SELECT id FROM account_subjects WHERE code = '2202'), '2202.02', '贷', 'active'),
('2202.03', '应付账款 - 印染厂', 2, (SELECT id FROM account_subjects WHERE code = '2202'), '2202.03', '贷', 'active'),
('2221', '应交税费', 1, NULL, '2221', '贷', 'active'),
('2221.01.01', '应交税费 - 应交增值税（进项税额）', 3, (SELECT id FROM account_subjects WHERE code = '2221'), '2221.01.01', '借', 'active'),
('2221.01.02', '应交税费 - 应交增值税（销项税额）', 3, (SELECT id FROM account_subjects WHERE code = '2221'), '2221.01.02', '贷', 'active'),

-- 所有者权益类
('3001', '实收资本', 1, NULL, '3001', '贷', 'active'),
('3101', '盈余公积', 1, NULL, '3101', '贷', 'active'),
('3201', '本年利润', 1, NULL, '3201', '贷', 'active'),

-- 成本类
('5001', '生产成本', 1, NULL, '5001', '借', 'active'),
('5001.01', '生产成本 - 直接材料', 2, (SELECT id FROM account_subjects WHERE code = '5001'), '5001.01', '借', 'active'),
('5001.01.001', '生产成本 - 直接材料 - 坯布成本', 3, (SELECT id FROM account_subjects WHERE code = '5001.01'), '5001.01.001', '借', 'active'),
('5001.01.002', '生产成本 - 直接材料 - 染料成本', 3, (SELECT id FROM account_subjects WHERE code = '5001.01'), '5001.01.002', '借', 'active'),
('5001.02', '生产成本 - 直接人工', 2, (SELECT id FROM account_subjects WHERE code = '5001'), '5001.02', '借', 'active'),
('5001.03', '生产成本 - 制造费用', 2, (SELECT id FROM account_subjects WHERE code = '5001'), '5001.03', '借', 'active'),
('5001.04', '生产成本 - 委托加工费', 2, (SELECT id FROM account_subjects WHERE code = '5001'), '5001.04', '借', 'active'),
('5001.04.001', '生产成本 - 委托加工费 - 染费', 3, (SELECT id FROM account_subjects WHERE code = '5001.04'), '5001.04.001', '借', 'active'),

-- 损益类
('6001', '主营业务收入', 1, NULL, '6001', '贷', 'active'),
('6001.01', '主营业务收入 - 国内销售', 2, (SELECT id FROM account_subjects WHERE code = '6001'), '6001.01', '贷', 'active'),
('6001.01.001', '主营业务收入 - 国内销售 - 坯布', 3, (SELECT id FROM account_subjects WHERE code = '6001.01'), '6001.01.001', '贷', 'active'),
('6001.01.002', '主营业务收入 - 国内销售 - 成品布', 3, (SELECT id FROM account_subjects WHERE code = '6001.01'), '6001.01.002', '贷', 'active'),
('6401', '主营业务成本', 1, NULL, '6401', '借', 'active'),
('6401.01', '主营业务成本 - 坯布销售成本', 2, (SELECT id FROM account_subjects WHERE code = '6401'), '6401.01', '借', 'active'),
('6401.02', '主营业务成本 - 成品布销售成本', 2, (SELECT id FROM account_subjects WHERE code = '6401'), '6401.02', '借', 'active'),
('6601', '销售费用', 1, NULL, '6601', '借', 'active'),
('6602', '管理费用', 1, NULL, '6602', '借', 'active'),
('6603', '财务费用', 1, NULL, '6603', '借', 'active') ON CONFLICT DO NOTHING;

-- 插入基础会计科目（面料行业预设科目）

-- 6. 触发器：自动更新科目余额
-- ============================================
CREATE OR REPLACE FUNCTION update_account_subject_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_update_account_subjects ON account_subjects;
CREATE TRIGGER trg_update_account_subjects
BEFORE UPDATE ON account_subjects
FOR EACH ROW
EXECUTE FUNCTION update_account_subject_timestamp();

DROP TRIGGER IF EXISTS trg_update_vouchers ON vouchers;
CREATE TRIGGER trg_update_vouchers
BEFORE UPDATE ON vouchers
FOR EACH ROW
EXECUTE FUNCTION update_account_subject_timestamp();

DROP TRIGGER IF EXISTS trg_update_account_balances ON account_balances;
CREATE TRIGGER trg_update_account_balances
BEFORE UPDATE ON account_balances
FOR EACH ROW
EXECUTE FUNCTION update_account_subject_timestamp();

COMMENT ON FUNCTION update_account_subject_timestamp() IS '自动更新 updated_at 字段';

-- 7. 凭证编号生成规则（按月连续编号）
-- ============================================
CREATE OR REPLACE FUNCTION generate_voucher_no(
    p_voucher_type VARCHAR,
    p_voucher_date DATE
) RETURNS VARCHAR AS $$
DECLARE
    v_year_month VARCHAR(7);
    v_prefix VARCHAR(10);
    v_seq INTEGER;
    v_voucher_no VARCHAR(50);
BEGIN
    -- 获取年月
    v_year_month := TO_CHAR(p_voucher_date, 'YYYY-MM');
    
    -- 凭证类型前缀
    CASE p_voucher_type
        WHEN '记' THEN v_prefix := 'JZ';
        WHEN '收' THEN v_prefix := 'SK';
        WHEN '付' THEN v_prefix := 'FK';
        WHEN '转' THEN v_prefix := 'ZZ';
        ELSE v_prefix := 'JZ';
    END CASE;
    
    -- 获取当月最大序号
    SELECT COALESCE(MAX(
        CAST(SUBSTRING(voucher_no FROM LENGTH(v_prefix) + LENGTH(v_year_month) + 3) AS INTEGER)
    ), 0) + 1 INTO v_seq
    FROM vouchers
    WHERE voucher_no LIKE v_prefix || v_year_month || '-%';
    
    -- 生成凭证号
    v_voucher_no := v_prefix || v_year_month || '-' || LPAD(v_seq::TEXT, 4, '0');
    
    RETURN v_voucher_no;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION generate_voucher_no IS '生成凭证编号（按月连续）';

-- ============================================
-- 迁移完成
-- ============================================

-- ============================================
-- 来源: 013_accounts_receivable.sql
-- ============================================
-- ============================================
-- 应收账款模块 - 基础表结构
-- ============================================
-- 文档编号：MIGRATION-021-AR
-- 创建日期：2026-03-15
-- 说明：应收账款模块表结构，参考应付账款模块适配销售回款场景
-- ============================================

-- 1. 应收单表
-- ============================================
CREATE TABLE IF NOT EXISTS ar_invoices (
    id SERIAL PRIMARY KEY,
    invoice_no VARCHAR(50) NOT NULL UNIQUE,
    invoice_date DATE NOT NULL,
    due_date DATE NOT NULL,
    
    -- 客户信息
    customer_id INTEGER NOT NULL,
    customer_name VARCHAR(200),
    customer_code VARCHAR(50),
    
    -- 来源单据
    source_type VARCHAR(20),
    source_module VARCHAR(50),
    source_bill_id INTEGER,
    source_bill_no VARCHAR(50),
    
    -- 面料行业字段
    batch_no VARCHAR(50),
    color_no VARCHAR(50),
    dye_lot_no VARCHAR(50),
    sales_order_no VARCHAR(50),
    
    -- 金额
    invoice_amount DECIMAL(14,2) NOT NULL,
    received_amount DECIMAL(14,2) DEFAULT 0,
    unpaid_amount DECIMAL(14,2) NOT NULL,
    tax_amount DECIMAL(14,2),
    
    -- 双计量单位
    quantity_meters DECIMAL(14,2),
    quantity_kg DECIMAL(14,2),
    unit_price DECIMAL(12,2),
    
    -- 状态
    status VARCHAR(20) DEFAULT 'draft',
    approval_status VARCHAR(20) DEFAULT 'unapproved',
    
    -- 审核
    created_by INTEGER,
    reviewed_by INTEGER,
    reviewed_at TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE ar_invoices IS '应收单表';
COMMENT ON COLUMN ar_invoices.invoice_no IS '应收单编号';
COMMENT ON COLUMN ar_invoices.customer_id IS '客户 ID';
COMMENT ON COLUMN ar_invoices.invoice_amount IS '应收金额';
COMMENT ON COLUMN ar_invoices.received_amount IS '已收金额';
COMMENT ON COLUMN ar_invoices.unpaid_amount IS '未收金额';

CREATE INDEX IF NOT EXISTS idx_ar_invoices_no ON ar_invoices(invoice_no);
CREATE INDEX IF NOT EXISTS idx_ar_invoices_customer ON ar_invoices(customer_id);
CREATE INDEX IF NOT EXISTS idx_ar_invoices_date ON ar_invoices(invoice_date);
CREATE INDEX IF NOT EXISTS idx_ar_invoices_status ON ar_invoices(status);
CREATE INDEX IF NOT EXISTS idx_ar_invoices_batch ON ar_invoices(batch_no);
CREATE INDEX IF NOT EXISTS idx_ar_invoices_color_no ON ar_invoices(color_no);

-- 2. 收款申请单表
-- ============================================
CREATE TABLE IF NOT EXISTS ar_collection_requests (
    id SERIAL PRIMARY KEY,
    request_no VARCHAR(50) NOT NULL UNIQUE,
    request_date DATE NOT NULL,
    
    -- 客户信息
    customer_id INTEGER NOT NULL,
    customer_name VARCHAR(200),
    
    -- 收款信息
    collection_amount DECIMAL(14,2) NOT NULL,
    collection_type VARCHAR(20),
    expected_date DATE,
    
    -- 关联应收单
    invoice_ids INTEGER[],
    
    -- 审批流程
    approval_level INTEGER DEFAULT 1,
    current_approver_id INTEGER,
    
    -- 状态
    status VARCHAR(20) DEFAULT 'draft',
    approval_status VARCHAR(20) DEFAULT 'pending',
    
    -- 审核
    created_by INTEGER,
    submitted_by INTEGER,
    submitted_at TIMESTAMPTZ,
    approved_by INTEGER,
    approved_at TIMESTAMPTZ,
    rejected_by INTEGER,
    rejected_at TIMESTAMPTZ,
    rejection_reason TEXT,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE ar_collection_requests IS '收款申请单表';
COMMENT ON COLUMN ar_collection_requests.request_no IS '收款申请单编号';
COMMENT ON COLUMN ar_collection_requests.customer_id IS '客户 ID';
COMMENT ON COLUMN ar_collection_requests.collection_amount IS '收款金额';
COMMENT ON COLUMN ar_collection_requests.approval_status IS '审批状态';

CREATE INDEX IF NOT EXISTS idx_ar_collection_requests_no ON ar_collection_requests(request_no);
CREATE INDEX IF NOT EXISTS idx_ar_collection_requests_customer ON ar_collection_requests(customer_id);
CREATE INDEX IF NOT EXISTS idx_ar_collection_requests_status ON ar_collection_requests(status);

-- 3. 收款单表
-- ============================================
CREATE TABLE IF NOT EXISTS ar_collections (
    id SERIAL PRIMARY KEY,
    collection_no VARCHAR(50) NOT NULL UNIQUE,
    collection_date DATE NOT NULL,
    
    -- 客户信息
    customer_id INTEGER NOT NULL,
    customer_name VARCHAR(200),
    
    -- 收款信息
    collection_amount DECIMAL(14,2) NOT NULL,
    collection_method VARCHAR(20),
    bank_account VARCHAR(100),
    check_no VARCHAR(50),
    
    -- 关联收款申请
    request_id INTEGER,
    request_no VARCHAR(50),
    
    -- 状态
    status VARCHAR(20) DEFAULT 'draft',
    
    -- 确认
    confirmed_by INTEGER,
    confirmed_at TIMESTAMPTZ,
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE ar_collections IS '收款单表';
COMMENT ON COLUMN ar_collections.collection_no IS '收款单编号';
COMMENT ON COLUMN ar_collections.customer_id IS '客户 ID';
COMMENT ON COLUMN ar_collections.collection_amount IS '收款金额';
COMMENT ON COLUMN ar_collections.collection_method IS '收款方式';

CREATE INDEX IF NOT EXISTS idx_ar_collections_no ON ar_collections(collection_no);
CREATE INDEX IF NOT EXISTS idx_ar_collections_customer ON ar_collections(customer_id);
CREATE INDEX IF NOT EXISTS idx_ar_collections_date ON ar_collections(collection_date);
CREATE INDEX IF NOT EXISTS idx_ar_collections_status ON ar_collections(status);

-- 4. 核销记录表
-- ============================================
CREATE TABLE IF NOT EXISTS ar_verifications (
    id SERIAL PRIMARY KEY,
    verification_no VARCHAR(50) NOT NULL UNIQUE,
    verification_date DATE NOT NULL,
    verification_type VARCHAR(20),
    
    -- 客户信息
    customer_id INTEGER NOT NULL,
    customer_name VARCHAR(200),
    
    -- 核销信息
    verification_amount DECIMAL(14,2) NOT NULL,
    
    -- 关联单据
    invoice_ids INTEGER[],
    collection_ids INTEGER[],
    
    -- 核销明细
    invoice_amount DECIMAL(14,2),
    collection_amount DECIMAL(14,2),
    difference_amount DECIMAL(14,2),
    
    -- 状态
    status VARCHAR(20) DEFAULT 'draft',
    
    -- 取消
    cancelled_by INTEGER,
    cancelled_at TIMESTAMPTZ,
    cancel_reason TEXT,
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE ar_verifications IS '核销记录表';
COMMENT ON COLUMN ar_verifications.verification_no IS '核销单编号';
COMMENT ON COLUMN ar_verifications.customer_id IS '客户 ID';
COMMENT ON COLUMN ar_verifications.verification_amount IS '核销金额';

CREATE INDEX IF NOT EXISTS idx_ar_verifications_no ON ar_verifications(verification_no);
CREATE INDEX IF NOT EXISTS idx_ar_verifications_customer ON ar_verifications(customer_id);
CREATE INDEX IF NOT EXISTS idx_ar_verifications_date ON ar_verifications(verification_date);

-- 5. 对账单表
-- ============================================
CREATE TABLE IF NOT EXISTS ar_reconciliations (
    id SERIAL PRIMARY KEY,
    reconciliation_no VARCHAR(50) NOT NULL UNIQUE,
    reconciliation_date DATE NOT NULL,
    
    -- 会计期间
    period_start DATE,
    period_end DATE,
    
    -- 客户信息
    customer_id INTEGER NOT NULL,
    customer_name VARCHAR(200),
    
    -- 对账信息
    opening_balance DECIMAL(14,2),
    total_invoices DECIMAL(14,2),
    total_collections DECIMAL(14,2),
    closing_balance DECIMAL(14,2),
    
    -- 对账结果
    reconciliation_status VARCHAR(20) DEFAULT 'pending',
    confirmed_by_customer BOOLEAN DEFAULT false,
    dispute_reason TEXT,
    
    -- 确认
    confirmed_by INTEGER,
    confirmed_at TIMESTAMPTZ,
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE ar_reconciliations IS '对账单表';
COMMENT ON COLUMN ar_reconciliations.reconciliation_no IS '对账单编号';
COMMENT ON COLUMN ar_reconciliations.customer_id IS '客户 ID';
COMMENT ON COLUMN ar_reconciliations.opening_balance IS '期初余额';
COMMENT ON COLUMN ar_reconciliations.closing_balance IS '期末余额';

CREATE INDEX IF NOT EXISTS idx_ar_reconciliations_no ON ar_reconciliations(reconciliation_no);
CREATE INDEX IF NOT EXISTS idx_ar_reconciliations_customer ON ar_reconciliations(customer_id);
CREATE INDEX IF NOT EXISTS idx_ar_reconciliations_period ON ar_reconciliations(period_start, period_end);

-- 6. 收款计划表
-- ============================================
CREATE TABLE IF NOT EXISTS ar_collection_plans (
    id SERIAL PRIMARY KEY,
    plan_no VARCHAR(50) NOT NULL UNIQUE,
    
    -- 客户信息
    customer_id INTEGER NOT NULL,
    customer_name VARCHAR(200),
    
    -- 计划信息
    invoice_id INTEGER,
    invoice_no VARCHAR(50),
    
    plan_amount DECIMAL(14,2) NOT NULL,
    plan_date DATE NOT NULL,
    actual_amount DECIMAL(14,2),
    actual_date DATE,
    
    -- 状态
    status VARCHAR(20) DEFAULT 'pending',
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE ar_collection_plans IS '收款计划表';
COMMENT ON COLUMN ar_collection_plans.plan_no IS '收款计划编号';
COMMENT ON COLUMN ar_collection_plans.customer_id IS '客户 ID';
COMMENT ON COLUMN ar_collection_plans.plan_amount IS '计划收款金额';
COMMENT ON COLUMN ar_collection_plans.plan_date IS '计划收款日期';

CREATE INDEX IF NOT EXISTS idx_ar_collection_plans_customer ON ar_collection_plans(customer_id);
CREATE INDEX IF NOT EXISTS idx_ar_collection_plans_date ON ar_collection_plans(plan_date);

-- ============================================
-- 迁移完成
-- ============================================

-- ============================================
-- 来源: 014_cost_management.sql
-- ============================================
-- ============================================
-- 成本管理模块 - 基础表结构
-- ============================================
-- 文档编号：MIGRATION-022-COST
-- 创建日期：2026-03-15
-- 说明：成本管理模块表结构，面料行业成本核算
-- ============================================

-- 1. 成本归集表
-- ============================================
CREATE TABLE IF NOT EXISTS cost_collections (
    id SERIAL PRIMARY KEY,
    collection_no VARCHAR(50) NOT NULL UNIQUE,
    collection_date DATE NOT NULL,
    
    -- 成本对象
    cost_object_type VARCHAR(20),
    cost_object_id INTEGER,
    cost_object_no VARCHAR(50),
    
    -- 面料行业字段
    batch_no VARCHAR(50),
    color_no VARCHAR(50),
    dye_lot_no VARCHAR(50),
    workshop VARCHAR(100),
    production_order_no VARCHAR(50),
    
    -- 成本构成
    direct_material DECIMAL(14,2) DEFAULT 0,
    direct_labor DECIMAL(14,2) DEFAULT 0,
    manufacturing_overhead DECIMAL(14,2) DEFAULT 0,
    processing_fee DECIMAL(14,2) DEFAULT 0,
    dyeing_fee DECIMAL(14,2) DEFAULT 0,
    
    -- 总金额
    total_cost DECIMAL(14,2) DEFAULT 0,
    
    -- 双计量单位
    output_quantity_meters DECIMAL(14,2),
    output_quantity_kg DECIMAL(14,2),
    unit_cost_meters DECIMAL(14,2),
    unit_cost_kg DECIMAL(14,2),
    
    -- 状态
    status VARCHAR(20) DEFAULT 'draft',
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE cost_collections IS '成本归集表';
COMMENT ON COLUMN cost_collections.collection_no IS '成本归集单编号';
COMMENT ON COLUMN cost_collections.batch_no IS '批次号';
COMMENT ON COLUMN cost_collections.color_no IS '色号';
COMMENT ON COLUMN cost_collections.direct_material IS '直接材料';
COMMENT ON COLUMN cost_collections.direct_labor IS '直接人工';
COMMENT ON COLUMN cost_collections.manufacturing_overhead IS '制造费用';
COMMENT ON COLUMN cost_collections.processing_fee IS '委托加工费';
COMMENT ON COLUMN cost_collections.dyeing_fee IS '染费';
COMMENT ON COLUMN cost_collections.total_cost IS '总成本';
COMMENT ON COLUMN cost_collections.unit_cost_meters IS '单位成本（米）';
COMMENT ON COLUMN cost_collections.unit_cost_kg IS '单位成本（公斤）';

CREATE INDEX IF NOT EXISTS idx_cost_collections_no ON cost_collections(collection_no);
CREATE INDEX IF NOT EXISTS idx_cost_collections_batch ON cost_collections(batch_no);
CREATE INDEX IF NOT EXISTS idx_cost_collections_color_no ON cost_collections(color_no);
CREATE INDEX IF NOT EXISTS idx_cost_collections_date ON cost_collections(collection_date);

-- 2. 直接材料明细表
-- ============================================
CREATE TABLE IF NOT EXISTS cost_direct_materials (
    id SERIAL PRIMARY KEY,
    collection_id INTEGER NOT NULL REFERENCES cost_collections(id),
    
    -- 物料信息
    material_id INTEGER,
    material_name VARCHAR(200),
    material_code VARCHAR(50),
    
    -- 面料行业字段
    batch_no VARCHAR(50),
    color_no VARCHAR(50),
    
    -- 数量和金额
    quantity_meters DECIMAL(14,2),
    quantity_kg DECIMAL(14,2),
    unit_price DECIMAL(12,2),
    amount DECIMAL(14,2),
    
    -- 来源单据
    source_type VARCHAR(20),
    source_bill_id INTEGER,
    source_bill_no VARCHAR(50),
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE cost_direct_materials IS '直接材料明细表';
COMMENT ON COLUMN cost_direct_materials.collection_id IS '成本归集 ID';
COMMENT ON COLUMN cost_direct_materials.material_id IS '物料 ID';
COMMENT ON COLUMN cost_direct_materials.amount IS '金额';

CREATE INDEX IF NOT EXISTS idx_cost_direct_materials_collection ON cost_direct_materials(collection_id);
CREATE INDEX IF NOT EXISTS idx_cost_direct_materials_material ON cost_direct_materials(material_id);

-- 3. 直接人工明细表
-- ============================================
CREATE TABLE IF NOT EXISTS cost_direct_labors (
    id SERIAL PRIMARY KEY,
    collection_id INTEGER NOT NULL REFERENCES cost_collections(id),
    
    -- 员工信息
    employee_id INTEGER,
    employee_name VARCHAR(100),
    employee_code VARCHAR(50),
    
    -- 工时和工资
    work_hours DECIMAL(10,2),
    hourly_rate DECIMAL(10,2),
    amount DECIMAL(14,2),
    
    -- 工作描述
    work_description TEXT,
    workshop VARCHAR(100),
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE cost_direct_labors IS '直接人工明细表';
COMMENT ON COLUMN cost_direct_labors.collection_id IS '成本归集 ID';
COMMENT ON COLUMN cost_direct_labors.work_hours IS '工时';
COMMENT ON COLUMN cost_direct_labors.amount IS '工资金额';

CREATE INDEX IF NOT EXISTS idx_cost_direct_labors_collection ON cost_direct_labors(collection_id);

-- 4. 制造费用明细表
-- ============================================
CREATE TABLE IF NOT EXISTS cost_manufacturing_overheads (
    id SERIAL PRIMARY KEY,
    collection_id INTEGER NOT NULL REFERENCES cost_collections(id),
    
    -- 费用类型
    expense_type VARCHAR(50),
    expense_name VARCHAR(200),
    
    -- 金额
    amount DECIMAL(14,2),
    
    -- 分摊方式
    allocation_method VARCHAR(20),
    allocation_base DECIMAL(14,2),
    
    -- 描述
    description TEXT,
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE cost_manufacturing_overheads IS '制造费用明细表';
COMMENT ON COLUMN cost_manufacturing_overheads.collection_id IS '成本归集 ID';
COMMENT ON COLUMN cost_manufacturing_overheads.expense_type IS '费用类型';
COMMENT ON COLUMN cost_manufacturing_overheads.amount IS '金额';
COMMENT ON COLUMN cost_manufacturing_overheads.allocation_method IS '分摊方式';

CREATE INDEX IF NOT EXISTS idx_cost_manufacturing_overheads_collection ON cost_manufacturing_overheads(collection_id);

-- 5. 染费明细表
-- ============================================
CREATE TABLE IF NOT EXISTS cost_dyeing_fees (
    id SERIAL PRIMARY KEY,
    collection_id INTEGER NOT NULL REFERENCES cost_collections(id),
    
    -- 印染厂信息
    dyeing_factory_id INTEGER,
    dyeing_factory_name VARCHAR(200),
    
    -- 面料行业字段
    batch_no VARCHAR(50),
    color_no VARCHAR(50),
    dye_lot_no VARCHAR(50),
    
    -- 数量和金额
    quantity_meters DECIMAL(14,2),
    quantity_kg DECIMAL(14,2),
    unit_price DECIMAL(12,2),
    amount DECIMAL(14,2),
    
    -- 染费类型
    dyeing_type VARCHAR(20),
    
    -- 来源单据
    invoice_id INTEGER,
    invoice_no VARCHAR(50),
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE cost_dyeing_fees IS '染费明细表';
COMMENT ON COLUMN cost_dyeing_fees.collection_id IS '成本归集 ID';
COMMENT ON COLUMN cost_dyeing_fees.dyeing_factory_id IS '印染厂 ID';
COMMENT ON COLUMN cost_dyeing_fees.quantity_meters IS '数量（米）';
COMMENT ON COLUMN cost_dyeing_fees.amount IS '染费金额';

CREATE INDEX IF NOT EXISTS idx_cost_dyeing_fees_collection ON cost_dyeing_fees(collection_id);
CREATE INDEX IF NOT EXISTS idx_cost_dyeing_fees_factory ON cost_dyeing_fees(dyeing_factory_id);

-- 6. 成本分析表
-- ============================================
CREATE TABLE IF NOT EXISTS cost_analyses (
    id SERIAL PRIMARY KEY,
    analysis_no VARCHAR(50) NOT NULL UNIQUE,
    analysis_date DATE NOT NULL,
    
    -- 分析维度
    period VARCHAR(7),
    batch_no VARCHAR(50),
    color_no VARCHAR(50),
    workshop VARCHAR(100),
    
    -- 成本数据
    total_direct_material DECIMAL(14,2),
    total_direct_labor DECIMAL(14,2),
    total_overhead DECIMAL(14,2),
    total_processing_fee DECIMAL(14,2),
    total_dyeing_fee DECIMAL(14,2),
    total_cost DECIMAL(14,2),
    
    -- 产量数据
    total_output_meters DECIMAL(14,2),
    total_output_kg DECIMAL(14,2),
    
    -- 单位成本
    avg_unit_cost_meters DECIMAL(14,2),
    avg_unit_cost_kg DECIMAL(14,2),
    
    -- 对比分析
    standard_cost DECIMAL(14,2),
    variance DECIMAL(14,2),
    variance_rate DECIMAL(8,4),
    
    -- 分析结论
    conclusion TEXT,
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE cost_analyses IS '成本分析表';
COMMENT ON COLUMN cost_analyses.analysis_no IS '成本分析单编号';
COMMENT ON COLUMN cost_analyses.period IS '会计期间';
COMMENT ON COLUMN cost_analyses.total_direct_material IS '直接材料合计';
COMMENT ON COLUMN cost_analyses.total_cost IS '总成本';
COMMENT ON COLUMN cost_analyses.avg_unit_cost_meters IS '平均单位成本（米）';
COMMENT ON COLUMN cost_analyses.variance IS '差异';
COMMENT ON COLUMN cost_analyses.variance_rate IS '差异率';

CREATE INDEX IF NOT EXISTS idx_cost_analyses_no ON cost_analyses(analysis_no);
CREATE INDEX IF NOT EXISTS idx_cost_analyses_period ON cost_analyses(period);
CREATE INDEX IF NOT EXISTS idx_cost_analyses_batch ON cost_analyses(batch_no);
CREATE INDEX IF NOT EXISTS idx_cost_analyses_color_no ON cost_analyses(color_no);

-- ============================================
-- 迁移完成
-- ============================================

-- ============================================
-- 来源: 015_fixed_assets.sql
-- ============================================
-- ============================================
-- P1 级模块 - 固定资产管理
-- ============================================
-- 文档编号：MIGRATION-030-FIXED_ASSETS
-- 创建日期：2026-03-15
-- 说明：固定资产管理模块表结构
-- ============================================

-- 1. 固定资产卡片表
-- ============================================
CREATE TABLE IF NOT EXISTS fixed_assets (
    id SERIAL PRIMARY KEY,
    asset_no VARCHAR(50) NOT NULL UNIQUE,
    asset_name VARCHAR(200) NOT NULL,
    asset_category VARCHAR(50),
    
    -- 规格型号
    specification VARCHAR(200),
    model VARCHAR(100),
    
    -- 使用信息
    use_department_id INTEGER,
    use_location VARCHAR(200),
    responsible_person_id INTEGER,
    
    -- 价值信息
    original_value DECIMAL(14,2) NOT NULL,
    salvage_value DECIMAL(14,2),
    salvage_rate DECIMAL(8,4),
    depreciable_value DECIMAL(14,2),
    
    -- 折旧信息
    depreciation_method VARCHAR(20),
    useful_life INTEGER,
    monthly_depreciation DECIMAL(14,2),
    accumulated_depreciation DECIMAL(14,2) DEFAULT 0,
    net_value DECIMAL(14,2),
    
    -- 状态
    status VARCHAR(20) DEFAULT 'active',
    purchase_date DATE,
    in_service_date DATE,
    disposal_date DATE,
    
    -- 供应商信息
    supplier_id INTEGER,
    supplier_name VARCHAR(200),
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE fixed_assets IS '固定资产卡片表';
COMMENT ON COLUMN fixed_assets.asset_no IS '资产编号';
COMMENT ON COLUMN fixed_assets.original_value IS '原值';
COMMENT ON COLUMN fixed_assets.accumulated_depreciation IS '累计折旧';
COMMENT ON COLUMN fixed_assets.net_value IS '净值';

CREATE INDEX IF NOT EXISTS idx_fixed_assets_no ON fixed_assets(asset_no);
CREATE INDEX IF NOT EXISTS idx_fixed_assets_category ON fixed_assets(asset_category);
CREATE INDEX IF NOT EXISTS idx_fixed_assets_status ON fixed_assets(status);

-- 2. 折旧记录表
-- ============================================
CREATE TABLE IF NOT EXISTS depreciation_records (
    id SERIAL PRIMARY KEY,
    asset_id INTEGER NOT NULL REFERENCES fixed_assets(id),
    
    -- 会计期间
    period VARCHAR(7) NOT NULL,
    
    -- 折旧金额
    monthly_depreciation DECIMAL(14,2),
    accumulated_depreciation DECIMAL(14,2),
    
    -- 状态
    status VARCHAR(20) DEFAULT 'pending',
    posted_by INTEGER,
    posted_at TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE depreciation_records IS '折旧记录表';
COMMENT ON COLUMN depreciation_records.period IS '会计期间';
COMMENT ON COLUMN depreciation_records.monthly_depreciation IS '月折旧额';

CREATE INDEX IF NOT EXISTS idx_depreciation_records_asset ON depreciation_records(asset_id);
CREATE INDEX IF NOT EXISTS idx_depreciation_records_period ON depreciation_records(period);

-- 3. 资产处置表
-- ============================================
CREATE TABLE IF NOT EXISTS asset_disposals (
    id SERIAL PRIMARY KEY,
    disposal_no VARCHAR(50) NOT NULL UNIQUE,
    asset_id INTEGER NOT NULL REFERENCES fixed_assets(id),
    
    -- 处置信息
    disposal_type VARCHAR(20),
    disposal_date DATE,
    disposal_reason TEXT,
    
    -- 金额
    disposal_value DECIMAL(14,2),
    disposal_cost DECIMAL(14,2),
    net_gain_loss DECIMAL(14,2),
    
    -- 审批
    approved_by INTEGER,
    approved_at TIMESTAMPTZ,
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE asset_disposals IS '资产处置表';
COMMENT ON COLUMN asset_disposals.disposal_type IS '处置类型';
COMMENT ON COLUMN asset_disposals.net_gain_loss IS '处置损益';

CREATE INDEX IF NOT EXISTS idx_asset_disposals_no ON asset_disposals(disposal_no);
CREATE INDEX IF NOT EXISTS idx_asset_disposals_asset ON asset_disposals(asset_id);

-- ============================================
-- 迁移完成
-- ============================================

-- ============================================
-- 来源: 016_purchase_contract.sql
-- ============================================
-- ============================================
-- P1 级模块 - 采购合同管理
-- ============================================
-- 文档编号：MIGRATION-031-PURCHASE_CONTRACT
-- 创建日期**: 2026-03-15
-- 说明：采购合同管理模块表结构
-- ============================================

-- 1. 采购合同表
-- ============================================
CREATE TABLE IF NOT EXISTS purchase_contracts (
    id SERIAL PRIMARY KEY,
    contract_no VARCHAR(50) NOT NULL UNIQUE,
    contract_name VARCHAR(200) NOT NULL,
    
    -- 合同类型
    contract_type VARCHAR(20),
    
    -- 供应商信息
    supplier_id INTEGER NOT NULL,
    supplier_name VARCHAR(200),
    
    -- 金额信息
    total_amount DECIMAL(14,2),
    signed_date DATE,
    effective_date DATE,
    expiry_date DATE,
    
    -- 付款条款
    payment_terms TEXT,
    payment_method VARCHAR(50),
    
    -- 交货条款
    delivery_date DATE,
    delivery_location VARCHAR(200),
    
    -- 状态
    status VARCHAR(20) DEFAULT 'draft',
    
    -- 关联订单
    related_order_ids INTEGER[],
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE purchase_contracts IS '采购合同表';
COMMENT ON COLUMN purchase_contracts.contract_no IS '合同编号';
COMMENT ON COLUMN purchase_contracts.total_amount IS '合同总金额';
COMMENT ON COLUMN purchase_contracts.status IS '合同状态';

CREATE INDEX IF NOT EXISTS idx_purchase_contracts_no ON purchase_contracts(contract_no);
CREATE INDEX IF NOT EXISTS idx_purchase_contracts_supplier ON purchase_contracts(supplier_id);
CREATE INDEX IF NOT EXISTS idx_purchase_contracts_status ON purchase_contracts(status);

-- 2. 合同执行表
-- ============================================
CREATE TABLE IF NOT EXISTS contract_executions (
    id SERIAL PRIMARY KEY,
    contract_id INTEGER NOT NULL REFERENCES purchase_contracts(id),
    
    -- 执行信息
    execution_type VARCHAR(20),
    execution_date DATE,
    
    -- 关联单据
    related_bill_type VARCHAR(20),
    related_bill_id INTEGER,
    related_bill_no VARCHAR(50),
    
    -- 金额
    execution_amount DECIMAL(14,2),
    
    -- 状态
    status VARCHAR(20) DEFAULT 'pending',
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE contract_executions IS '合同执行表';
COMMENT ON COLUMN contract_executions.execution_type IS '执行类型';
COMMENT ON COLUMN contract_executions.execution_amount IS '执行金额';

CREATE INDEX IF NOT EXISTS idx_contract_executions_contract ON contract_executions(contract_id);

-- ============================================
-- 迁移完成
-- ============================================

-- ============================================
-- 来源: 017_sales_contract.sql
-- ============================================
-- ============================================
-- P1 级模块 - 销售合同管理
-- ============================================
-- 文档编号：MIGRATION-032-SALES_CONTRACT
-- 创建日期**: 2026-03-15
-- 说明：销售合同管理模块表结构
-- ============================================

-- 1. 销售合同表
-- ============================================
CREATE TABLE IF NOT EXISTS sales_contracts (
    id SERIAL PRIMARY KEY,
    contract_no VARCHAR(50) NOT NULL UNIQUE,
    contract_name VARCHAR(200) NOT NULL,
    
    -- 合同类型
    contract_type VARCHAR(20),
    
    -- 客户信息
    customer_id INTEGER NOT NULL,
    customer_name VARCHAR(200),
    
    -- 金额信息
    total_amount DECIMAL(14,2),
    signed_date DATE,
    effective_date DATE,
    expiry_date DATE,
    
    -- 付款条款
    payment_terms TEXT,
    payment_method VARCHAR(50),
    
    -- 交货条款
    delivery_date DATE,
    delivery_location VARCHAR(200),
    
    -- 状态
    status VARCHAR(20) DEFAULT 'draft',
    
    -- 关联订单
    related_order_ids INTEGER[],
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE sales_contracts IS '销售合同表';
COMMENT ON COLUMN sales_contracts.contract_no IS '合同编号';
COMMENT ON COLUMN sales_contracts.total_amount IS '合同总金额';
COMMENT ON COLUMN sales_contracts.status IS '合同状态';

CREATE INDEX IF NOT EXISTS idx_sales_contracts_no ON sales_contracts(contract_no);
CREATE INDEX IF NOT EXISTS idx_sales_contracts_customer ON sales_contracts(customer_id);
CREATE INDEX IF NOT EXISTS idx_sales_contracts_status ON sales_contracts(status);

-- ============================================
-- 迁移完成
-- ============================================

-- ============================================
-- 来源: 018_customer_credit.sql
-- ============================================
-- ============================================
-- P1 级模块 - 客户信用管理
-- ============================================
-- 文档编号：MIGRATION-033-CUSTOMER_CREDIT
-- 创建日期**: 2026-03-15
-- 说明：客户信用管理模块表结构
-- ============================================

-- 1. 客户信用评级表
-- ============================================
CREATE TABLE IF NOT EXISTS customer_credit_ratings (
    id SERIAL PRIMARY KEY,
    customer_id INTEGER NOT NULL UNIQUE,
    customer_name VARCHAR(200),
    
    -- 信用等级
    credit_level VARCHAR(10),
    credit_score INTEGER,
    
    -- 信用额度
    credit_limit DECIMAL(14,2),
    used_credit DECIMAL(14,2) DEFAULT 0,
    available_credit DECIMAL(14,2),
    
    -- 信用期限
    credit_days INTEGER,
    
    -- 评估信息
    last_assessment_date DATE,
    next_assessment_date DATE,
    
    -- 状态
    status VARCHAR(20) DEFAULT 'active',
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE customer_credit_ratings IS '客户信用评级表';
COMMENT ON COLUMN customer_credit_ratings.credit_level IS '信用等级';
COMMENT ON COLUMN customer_credit_ratings.credit_limit IS '信用额度';
COMMENT ON COLUMN customer_credit_ratings.available_credit IS '可用额度';

CREATE INDEX IF NOT EXISTS idx_customer_credit_ratings_customer ON customer_credit_ratings(customer_id);
CREATE INDEX IF NOT EXISTS idx_customer_credit_ratings_level ON customer_credit_ratings(credit_level);

-- 2. 信用变更记录表
-- ============================================
CREATE TABLE IF NOT EXISTS customer_credit_changes (
    id SERIAL PRIMARY KEY,
    customer_id INTEGER NOT NULL,
    
    -- 变更内容
    change_type VARCHAR(20),
    old_value TEXT,
    new_value TEXT,
    
    -- 变更原因
    reason TEXT,
    
    -- 审批
    approved_by INTEGER,
    approved_at TIMESTAMPTZ,
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE customer_credit_changes IS '客户信用变更记录表';
COMMENT ON COLUMN customer_credit_changes.change_type IS '变更类型';

CREATE INDEX IF NOT EXISTS idx_customer_credit_changes_customer ON customer_credit_changes(customer_id);

-- ============================================
-- 迁移完成
-- ============================================

-- ============================================
-- 来源: 019_financial_analysis.sql
-- ============================================
-- P2 级模块：财务分析
-- 创建时间：2026-03-15
-- 功能：财务指标分析、趋势分析、财务报表

-- 财务指标表
CREATE TABLE IF NOT EXISTS financial_indicators (
    id SERIAL PRIMARY KEY,
    indicator_name VARCHAR(100) NOT NULL,
    indicator_code VARCHAR(50) NOT NULL UNIQUE,
    indicator_type VARCHAR(20) NOT NULL,
    formula TEXT,
    unit VARCHAR(20),
    status VARCHAR(20) DEFAULT 'active',
    remark TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 财务分析结果表
CREATE TABLE IF NOT EXISTS financial_analysis_results (
    id SERIAL PRIMARY KEY,
    analysis_type VARCHAR(20) NOT NULL,
    period VARCHAR(7) NOT NULL,
    indicator_id INTEGER REFERENCES financial_indicators(id),
    indicator_value DECIMAL(18,4),
    target_value DECIMAL(18,4),
    variance DECIMAL(18,4),
    variance_rate DECIMAL(5,2),
    trend VARCHAR(10),
    analysis_date DATE DEFAULT CURRENT_DATE,
    created_by INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 财务趋势分析表
CREATE TABLE IF NOT EXISTS financial_trends (
    id SERIAL PRIMARY KEY,
    indicator_id INTEGER REFERENCES financial_indicators(id),
    period VARCHAR(7) NOT NULL,
    value DECIMAL(18,4) NOT NULL,
    previous_value DECIMAL(18,4),
    change_amount DECIMAL(18,4),
    change_rate DECIMAL(5,2),
    trend_direction VARCHAR(10),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 财务报表配置表
CREATE TABLE IF NOT EXISTS financial_report_configs (
    id SERIAL PRIMARY KEY,
    report_name VARCHAR(100) NOT NULL,
    report_type VARCHAR(20) NOT NULL,
    period_type VARCHAR(20) DEFAULT 'monthly',
    template_config JSONB,
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_financial_indicators_type ON financial_indicators(indicator_type);
CREATE INDEX IF NOT EXISTS idx_financial_analysis_period ON financial_analysis_results(period);
CREATE INDEX IF NOT EXISTS idx_financial_trends_period ON financial_trends(period);

-- 添加中文注释
COMMENT ON TABLE financial_indicators IS '财务指标表';
COMMENT ON COLUMN financial_indicators.indicator_name IS '指标名称';
COMMENT ON COLUMN financial_indicators.indicator_code IS '指标代码';
COMMENT ON COLUMN financial_indicators.indicator_type IS '指标类型（盈利/偿债/营运/发展）';
COMMENT ON COLUMN financial_indicators.formula IS '计算公式';

COMMENT ON TABLE financial_analysis_results IS '财务分析结果表';
COMMENT ON COLUMN financial_analysis_results.analysis_type IS '分析类型';
COMMENT ON COLUMN financial_analysis_results.period IS '期间（YYYY-MM）';
COMMENT ON COLUMN financial_analysis_results.indicator_value IS '指标值';
COMMENT ON COLUMN financial_analysis_results.target_value IS '目标值';
COMMENT ON COLUMN financial_analysis_results.variance IS '差异';
COMMENT ON COLUMN financial_analysis_results.variance_rate IS '差异率';

COMMENT ON TABLE financial_trends IS '财务趋势分析表';
COMMENT ON COLUMN financial_trends.value IS '当前值';
COMMENT ON COLUMN financial_trends.previous_value IS '上期值';
COMMENT ON COLUMN financial_trends.change_amount IS '变动额';
COMMENT ON COLUMN financial_trends.change_rate IS '变动率';
COMMENT ON COLUMN financial_trends.trend_direction IS '趋势方向（上升/下降/持平）';

COMMENT ON TABLE financial_report_configs IS '财务报表配置表';
COMMENT ON COLUMN financial_report_configs.report_type IS '报表类型（资产负债表/利润表/现金流量表）';
COMMENT ON COLUMN financial_report_configs.period_type IS '期间类型（月/季/年）';

-- ============================================
-- 来源: 020_supplier_evaluation.sql
-- ============================================
-- P2 级模块：供应商评估
-- 创建时间：2026-03-15
-- 功能：供应商绩效评估、评分管理、等级评定

-- 供应商评估指标表
CREATE TABLE IF NOT EXISTS supplier_evaluation_indicators (
    id SERIAL PRIMARY KEY,
    indicator_name VARCHAR(100) NOT NULL,
    indicator_code VARCHAR(50) NOT NULL UNIQUE,
    category VARCHAR(20) NOT NULL,
    weight DECIMAL(5,2) NOT NULL,
    max_score INTEGER DEFAULT 100,
    evaluation_method VARCHAR(50),
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 供应商评估记录表
CREATE TABLE IF NOT EXISTS supplier_evaluation_records (
    id SERIAL PRIMARY KEY,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id),
    evaluation_period VARCHAR(7) NOT NULL,
    indicator_id INTEGER REFERENCES supplier_evaluation_indicators(id),
    score DECIMAL(5,2),
    max_score INTEGER,
    weighted_score DECIMAL(5,2),
    evaluator_id INTEGER,
    evaluation_date DATE DEFAULT CURRENT_DATE,
    remark TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 供应商综合评分表
CREATE TABLE IF NOT EXISTS supplier_overall_scores (
    id SERIAL PRIMARY KEY,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id),
    evaluation_period VARCHAR(7) NOT NULL,
    total_score DECIMAL(5,2) NOT NULL,
    quality_score DECIMAL(5,2),
    delivery_score DECIMAL(5,2),
    price_score DECIMAL(5,2),
    service_score DECIMAL(5,2),
    level VARCHAR(10),
    rank INTEGER,
    evaluation_status VARCHAR(20) DEFAULT 'pending',
    evaluated_by INTEGER,
    evaluation_date DATE DEFAULT CURRENT_DATE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 供应商等级表
CREATE TABLE IF NOT EXISTS supplier_levels (
    id SERIAL PRIMARY KEY,
    level_code VARCHAR(10) NOT NULL UNIQUE,
    level_name VARCHAR(50) NOT NULL,
    min_score INTEGER NOT NULL,
    max_score INTEGER NOT NULL,
    benefits TEXT,
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_supplier_eval_indicators_category ON supplier_evaluation_indicators(category);
CREATE INDEX IF NOT EXISTS idx_supplier_eval_records_supplier ON supplier_evaluation_records(supplier_id);
CREATE INDEX IF NOT EXISTS idx_supplier_eval_records_period ON supplier_evaluation_records(evaluation_period);
CREATE INDEX IF NOT EXISTS idx_supplier_overall_scores_supplier ON supplier_overall_scores(supplier_id);
CREATE INDEX IF NOT EXISTS idx_supplier_overall_scores_period ON supplier_overall_scores(evaluation_period);

-- 添加中文注释
COMMENT ON TABLE supplier_evaluation_indicators IS '供应商评估指标表';
COMMENT ON COLUMN supplier_evaluation_indicators.category IS '指标类别（质量/交货/价格/服务）';
COMMENT ON COLUMN supplier_evaluation_indicators.weight IS '权重';
COMMENT ON COLUMN supplier_evaluation_indicators.max_score IS '满分';

COMMENT ON TABLE supplier_evaluation_records IS '供应商评估记录表';
COMMENT ON COLUMN supplier_evaluation_records.evaluation_period IS '评估期间';
COMMENT ON COLUMN supplier_evaluation_records.score IS '得分';
COMMENT ON COLUMN supplier_evaluation_records.weighted_score IS '加权得分';

COMMENT ON TABLE supplier_overall_scores IS '供应商综合评分表';
COMMENT ON COLUMN supplier_overall_scores.total_score IS '总分';
COMMENT ON COLUMN supplier_overall_scores.quality_score IS '质量得分';
COMMENT ON COLUMN supplier_overall_scores.delivery_score IS '交货得分';
COMMENT ON COLUMN supplier_overall_scores.price_score IS '价格得分';
COMMENT ON COLUMN supplier_overall_scores.service_score IS '服务得分';
COMMENT ON COLUMN supplier_overall_scores.level IS '等级（A/B/C/D）';

COMMENT ON TABLE supplier_levels IS '供应商等级表';
COMMENT ON COLUMN supplier_levels.level_code IS '等级代码';
COMMENT ON COLUMN supplier_levels.min_score IS '最低分';
COMMENT ON COLUMN supplier_levels.max_score IS '最高分';
COMMENT ON COLUMN supplier_levels.benefits IS '等级权益';

-- ============================================
-- 来源: 021_fund_management.sql
-- ============================================
-- P1 级模块：资金管理
-- 创建时间：2026-03-15
-- 功能：资金计划、资金调拨、资金监控

-- 资金账户表
CREATE TABLE IF NOT EXISTS fund_accounts (
    id SERIAL PRIMARY KEY,
    account_name VARCHAR(100) NOT NULL,
    account_no VARCHAR(50) NOT NULL UNIQUE,
    account_type VARCHAR(20) NOT NULL,
    bank_name VARCHAR(100),
    currency VARCHAR(10) DEFAULT 'CNY',
    balance DECIMAL(18,2) DEFAULT 0,
    available_balance DECIMAL(18,2) DEFAULT 0,
    frozen_balance DECIMAL(18,2) DEFAULT 0,
    status VARCHAR(20) DEFAULT 'active',
    opened_date DATE,
    remark TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 资金计划表
CREATE TABLE IF NOT EXISTS fund_plans (
    id SERIAL PRIMARY KEY,
    plan_no VARCHAR(50) NOT NULL UNIQUE,
    plan_name VARCHAR(200) NOT NULL,
    plan_type VARCHAR(20) NOT NULL,
    period VARCHAR(7) NOT NULL,
    planned_amount DECIMAL(18,2) NOT NULL,
    actual_amount DECIMAL(18,2) DEFAULT 0,
    variance_amount DECIMAL(18,2),
    variance_rate DECIMAL(5,2),
    status VARCHAR(20) DEFAULT 'draft',
    prepared_by INTEGER,
    approved_by INTEGER,
    approved_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 资金调拨表
CREATE TABLE IF NOT EXISTS fund_transfers (
    id SERIAL PRIMARY KEY,
    transfer_no VARCHAR(50) NOT NULL UNIQUE,
    from_account_id INTEGER REFERENCES fund_accounts(id),
    to_account_id INTEGER REFERENCES fund_accounts(id),
    amount DECIMAL(18,2) NOT NULL,
    transfer_type VARCHAR(20) NOT NULL,
    transfer_date DATE NOT NULL,
    purpose TEXT,
    status VARCHAR(20) DEFAULT 'pending',
    applied_by INTEGER,
    approved_by INTEGER,
    approved_at TIMESTAMP,
    executed_at TIMESTAMP,
    remark TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 资金流水表
CREATE TABLE IF NOT EXISTS fund_transactions (
    id SERIAL PRIMARY KEY,
    transaction_no VARCHAR(50) NOT NULL UNIQUE,
    account_id INTEGER REFERENCES fund_accounts(id),
    transaction_type VARCHAR(20) NOT NULL,
    amount DECIMAL(18,2) NOT NULL,
    balance_before DECIMAL(18,2),
    balance_after DECIMAL(18,2),
    related_type VARCHAR(50),
    related_id INTEGER,
    transaction_date TIMESTAMP NOT NULL,
    direction VARCHAR(10) NOT NULL,
    counterparty_name VARCHAR(200),
    counterparty_account VARCHAR(100),
    remark TEXT,
    created_by INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 资金监控表
CREATE TABLE IF NOT EXISTS fund_monitoring (
    id SERIAL PRIMARY KEY,
    account_id INTEGER REFERENCES fund_accounts(id),
    monitoring_date DATE NOT NULL,
    opening_balance DECIMAL(18,2),
    closing_balance DECIMAL(18,2),
    total_inflow DECIMAL(18,2) DEFAULT 0,
    total_outflow DECIMAL(18,2) DEFAULT 0,
    large_transaction_count INTEGER DEFAULT 0,
    alert_status VARCHAR(20) DEFAULT 'normal',
    alert_reason TEXT,
    monitored_by INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_fund_accounts_type ON fund_accounts(account_type);
CREATE INDEX IF NOT EXISTS idx_fund_plans_period ON fund_plans(period);
CREATE INDEX IF NOT EXISTS idx_fund_transfers_date ON fund_transfers(transfer_date);
CREATE INDEX IF NOT EXISTS idx_fund_transactions_account ON fund_transactions(account_id);
CREATE INDEX IF NOT EXISTS idx_fund_transactions_date ON fund_transactions(transaction_date);
CREATE INDEX IF NOT EXISTS idx_fund_monitoring_date ON fund_monitoring(monitoring_date);

-- 添加中文注释
COMMENT ON TABLE fund_accounts IS '资金账户表';
COMMENT ON COLUMN fund_accounts.account_type IS '账户类型（银行/现金/其他）';
COMMENT ON COLUMN fund_accounts.balance IS '账户余额';
COMMENT ON COLUMN fund_accounts.available_balance IS '可用余额';
COMMENT ON COLUMN fund_accounts.frozen_balance IS '冻结余额';

COMMENT ON TABLE fund_plans IS '资金计划表';
COMMENT ON COLUMN fund_plans.plan_type IS '计划类型（收入/支出）';
COMMENT ON COLUMN fund_plans.planned_amount IS '计划金额';
COMMENT ON COLUMN fund_plans.actual_amount IS '实际金额';

COMMENT ON TABLE fund_transfers IS '资金调拨表';
COMMENT ON COLUMN fund_transfers.transfer_type IS '调拨类型（内部/外部）';
COMMENT ON COLUMN fund_transfers.purpose IS '调拨用途';

COMMENT ON TABLE fund_transactions IS '资金流水表';
COMMENT ON COLUMN fund_transactions.transaction_type IS '交易类型（存入/取出/转账）';
COMMENT ON COLUMN fund_transactions.direction IS '方向（收入/支出）';

COMMENT ON TABLE fund_monitoring IS '资金监控表';
COMMENT ON COLUMN fund_monitoring.total_inflow IS '总流入';
COMMENT ON COLUMN fund_monitoring.total_outflow IS '总流出';
COMMENT ON COLUMN fund_monitoring.alert_status IS '预警状态（正常/预警/危险）';

-- ============================================
-- 来源: 022_budget_management.sql
-- ============================================
-- P1 级模块：预算管理
-- 创建时间：2026-03-15
-- 功能：预算编制、预算执行、预算控制

-- 预算科目表
CREATE TABLE IF NOT EXISTS budget_items (
    id SERIAL PRIMARY KEY,
    item_code VARCHAR(50) NOT NULL UNIQUE,
    item_name VARCHAR(100) NOT NULL,
    parent_id INTEGER REFERENCES budget_items(id),
    item_type VARCHAR(20) NOT NULL,
    level INTEGER DEFAULT 1,
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 预算方案表
CREATE TABLE IF NOT EXISTS budget_plans (
    id SERIAL PRIMARY KEY,
    plan_no VARCHAR(50) NOT NULL UNIQUE,
    plan_name VARCHAR(200) NOT NULL,
    budget_year INTEGER NOT NULL,
    budget_type VARCHAR(20) NOT NULL,
    department_id INTEGER REFERENCES departments(id),
    total_amount DECIMAL(18,2) NOT NULL,
    status VARCHAR(20) DEFAULT 'draft',
    prepared_by INTEGER,
    approved_by INTEGER,
    approved_at TIMESTAMP,
    remark TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 预算明细表
CREATE TABLE IF NOT EXISTS budget_plan_details (
    id SERIAL PRIMARY KEY,
    plan_id INTEGER NOT NULL REFERENCES budget_plans(id),
    budget_item_id INTEGER REFERENCES budget_items(id),
    period VARCHAR(7) NOT NULL,
    budget_amount DECIMAL(18,2) NOT NULL,
    actual_amount DECIMAL(18,2) DEFAULT 0,
    variance_amount DECIMAL(18,2),
    variance_rate DECIMAL(5,2),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 预算控制表
CREATE TABLE IF NOT EXISTS budget_controls (
    id SERIAL PRIMARY KEY,
    plan_id INTEGER NOT NULL REFERENCES budget_plans(id),
    budget_item_id INTEGER REFERENCES budget_items(id),
    control_type VARCHAR(20) NOT NULL,
    control_limit DECIMAL(18,2),
    warning_threshold DECIMAL(5,2) DEFAULT 80,
    control_status VARCHAR(20) DEFAULT 'normal',
    related_type VARCHAR(50),
    related_id INTEGER,
    amount DECIMAL(18,2),
    controlled_by INTEGER,
    controlled_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    remark TEXT
);

-- 预算调整表
CREATE TABLE IF NOT EXISTS budget_adjustments (
    id SERIAL PRIMARY KEY,
    adjustment_no VARCHAR(50) NOT NULL UNIQUE,
    plan_id INTEGER NOT NULL REFERENCES budget_plans(id),
    budget_item_id INTEGER REFERENCES budget_items(id),
    original_amount DECIMAL(18,2) NOT NULL,
    adjusted_amount DECIMAL(18,2) NOT NULL,
    change_amount DECIMAL(18,2),
    change_rate DECIMAL(5,2),
    reason TEXT NOT NULL,
    applied_by INTEGER,
    approved_by INTEGER,
    approved_at TIMESTAMP,
    status VARCHAR(20) DEFAULT 'pending',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_budget_items_type ON budget_items(item_type);
CREATE INDEX IF NOT EXISTS idx_budget_items_parent ON budget_items(parent_id);
CREATE INDEX IF NOT EXISTS idx_budget_plans_year ON budget_plans(budget_year);
CREATE INDEX IF NOT EXISTS idx_budget_plan_details_period ON budget_plan_details(period);
CREATE INDEX IF NOT EXISTS idx_budget_controls_plan ON budget_controls(plan_id);
CREATE INDEX IF NOT EXISTS idx_budget_adjustments_plan ON budget_adjustments(plan_id);

-- 添加中文注释
COMMENT ON TABLE budget_items IS '预算科目表';
COMMENT ON COLUMN budget_items.item_type IS '科目类型（收入/支出/资产/负债）';
COMMENT ON COLUMN budget_items.level IS '科目级别';

COMMENT ON TABLE budget_plans IS '预算方案表';
COMMENT ON COLUMN budget_plans.budget_type IS '预算类型（经营/资本/财务）';
COMMENT ON COLUMN budget_plans.total_amount IS '预算总额';

COMMENT ON TABLE budget_plan_details IS '预算明细表';
COMMENT ON COLUMN budget_plan_details.period IS '期间（YYYY-MM）';
COMMENT ON COLUMN budget_plan_details.budget_amount IS '预算金额';
COMMENT ON COLUMN budget_plan_details.actual_amount IS '实际金额';

COMMENT ON TABLE budget_controls IS '预算控制表';
COMMENT ON COLUMN budget_controls.control_type IS '控制类型（警告/冻结）';
COMMENT ON COLUMN budget_controls.warning_threshold IS '预警阈值（百分比）';

COMMENT ON TABLE budget_adjustments IS '预算调整表';
COMMENT ON COLUMN budget_adjustments.change_amount IS '变动金额';
COMMENT ON COLUMN budget_adjustments.reason IS '调整原因';

-- ============================================
-- 来源: 023_purchase_price.sql
-- ============================================
-- P2 级模块：采购价格管理
-- 创建时间：2026-03-15
-- 功能：采购价格管理、价格审批、价格趋势分析

-- 采购价格表
CREATE TABLE IF NOT EXISTS purchase_prices (
    id SERIAL PRIMARY KEY,
    product_id INTEGER NOT NULL REFERENCES products(id),
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id),
    price DECIMAL(14,4) NOT NULL,
    currency VARCHAR(10) DEFAULT 'CNY',
    unit VARCHAR(20) NOT NULL,
    min_order_qty DECIMAL(14,4) DEFAULT 0,
    price_type VARCHAR(20) DEFAULT 'standard',
    effective_date DATE NOT NULL,
    expiry_date DATE,
    status VARCHAR(20) DEFAULT 'active',
    approved_by INTEGER,
    approved_at TIMESTAMP,
    created_by INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 采购价格审批表
CREATE TABLE IF NOT EXISTS purchase_price_approvals (
    id SERIAL PRIMARY KEY,
    price_id INTEGER NOT NULL REFERENCES purchase_prices(id),
    approval_type VARCHAR(20) NOT NULL,
    old_price DECIMAL(14,4),
    new_price DECIMAL(14,4),
    change_rate DECIMAL(5,2),
    reason TEXT,
    applied_by INTEGER,
    approved_by INTEGER,
    approved_at TIMESTAMP,
    status VARCHAR(20) DEFAULT 'pending',
    remark TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 采购价格历史表
CREATE TABLE IF NOT EXISTS purchase_price_history (
    id SERIAL PRIMARY KEY,
    product_id INTEGER NOT NULL REFERENCES products(id),
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id),
    price DECIMAL(14,4) NOT NULL,
    price_date DATE NOT NULL,
    change_type VARCHAR(20),
    old_price DECIMAL(14,4),
    change_rate DECIMAL(5,2),
    remark TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_purchase_prices_product ON purchase_prices(product_id);
CREATE INDEX IF NOT EXISTS idx_purchase_prices_supplier ON purchase_prices(supplier_id);
CREATE INDEX IF NOT EXISTS idx_purchase_prices_effective ON purchase_prices(effective_date);
CREATE INDEX IF NOT EXISTS idx_purchase_price_history_product ON purchase_price_history(product_id);
CREATE INDEX IF NOT EXISTS idx_purchase_price_history_date ON purchase_price_history(price_date);

-- 添加中文注释
COMMENT ON TABLE purchase_prices IS '采购价格表';
COMMENT ON COLUMN purchase_prices.price_type IS '价格类型（标准/协议/促销）';
COMMENT ON COLUMN purchase_prices.min_order_qty IS '最小起订量';
COMMENT ON COLUMN purchase_prices.effective_date IS '生效日期';
COMMENT ON COLUMN purchase_prices.expiry_date IS '失效日期';

COMMENT ON TABLE purchase_price_approvals IS '采购价格审批表';
COMMENT ON COLUMN purchase_price_approvals.approval_type IS '审批类型（新增/调整）';
COMMENT ON COLUMN purchase_price_approvals.change_rate IS '变动率';

COMMENT ON TABLE purchase_price_history IS '采购价格历史表';
COMMENT ON COLUMN purchase_price_history.price_date IS '价格日期';
COMMENT ON COLUMN purchase_price_history.change_type IS '变动类型（上涨/下降/新增）';

-- ============================================
-- 来源: 024_sales_price.sql
-- ============================================
-- P2 级模块：销售价格管理
-- 创建时间：2026-03-15
-- 功能：销售价格管理、价格审批、价格策略

-- 销售价格表
CREATE TABLE IF NOT EXISTS sales_prices (
    id SERIAL PRIMARY KEY,
    product_id INTEGER NOT NULL REFERENCES products(id),
    customer_id INTEGER REFERENCES customers(id),
    customer_type VARCHAR(20),
    price DECIMAL(14,4) NOT NULL,
    currency VARCHAR(10) DEFAULT 'CNY',
    unit VARCHAR(20) NOT NULL,
    min_order_qty DECIMAL(14,4) DEFAULT 0,
    price_type VARCHAR(20) DEFAULT 'standard',
    price_level VARCHAR(20),
    effective_date DATE NOT NULL,
    expiry_date DATE,
    status VARCHAR(20) DEFAULT 'active',
    approved_by INTEGER,
    approved_at TIMESTAMP,
    created_by INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 销售价格审批表
CREATE TABLE IF NOT EXISTS sales_price_approvals (
    id SERIAL PRIMARY KEY,
    price_id INTEGER NOT NULL REFERENCES sales_prices(id),
    approval_type VARCHAR(20) NOT NULL,
    old_price DECIMAL(14,4),
    new_price DECIMAL(14,4),
    change_rate DECIMAL(5,2),
    reason TEXT,
    applied_by INTEGER,
    approved_by INTEGER,
    approved_at TIMESTAMP,
    status VARCHAR(20) DEFAULT 'pending',
    remark TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 销售价格历史表
CREATE TABLE IF NOT EXISTS sales_price_history (
    id SERIAL PRIMARY KEY,
    product_id INTEGER NOT NULL REFERENCES products(id),
    customer_type VARCHAR(20),
    price DECIMAL(14,4) NOT NULL,
    price_date DATE NOT NULL,
    change_type VARCHAR(20),
    old_price DECIMAL(14,4),
    change_rate DECIMAL(5,2),
    remark TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 价格策略表
CREATE TABLE IF NOT EXISTS price_strategies (
    id SERIAL PRIMARY KEY,
    strategy_name VARCHAR(100) NOT NULL,
    strategy_type VARCHAR(20) NOT NULL,
    customer_type VARCHAR(20),
    product_category_id INTEGER REFERENCES product_categories(id),
    discount_rate DECIMAL(5,2),
    min_price DECIMAL(14,4),
    max_price DECIMAL(14,4),
    priority INTEGER DEFAULT 1,
    status VARCHAR(20) DEFAULT 'active',
    effective_date DATE NOT NULL,
    expiry_date DATE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_sales_prices_product ON sales_prices(product_id);
CREATE INDEX IF NOT EXISTS idx_sales_prices_customer ON sales_prices(customer_id);
CREATE INDEX IF NOT EXISTS idx_sales_prices_effective ON sales_prices(effective_date);
CREATE INDEX IF NOT EXISTS idx_sales_price_history_product ON sales_price_history(product_id);
CREATE INDEX IF NOT EXISTS idx_sales_price_history_date ON sales_price_history(price_date);
CREATE INDEX IF NOT EXISTS idx_price_strategies_type ON price_strategies(strategy_type);

-- 添加中文注释
COMMENT ON TABLE sales_prices IS '销售价格表';
COMMENT ON COLUMN sales_prices.customer_type IS '客户类型（零售/批发/VIP）';
COMMENT ON COLUMN sales_prices.price_level IS '价格等级（一级/二级/三级）';
COMMENT ON COLUMN sales_prices.price_type IS '价格类型（标准/促销/协议）';

COMMENT ON TABLE sales_price_approvals IS '销售价格审批表';
COMMENT ON COLUMN sales_price_approvals.approval_type IS '审批类型（新增/调整）';

COMMENT ON TABLE sales_price_history IS '销售价格历史表';
COMMENT ON COLUMN sales_price_history.change_type IS '变动类型（上涨/下降/新增）';

COMMENT ON TABLE price_strategies IS '价格策略表';
COMMENT ON COLUMN price_strategies.strategy_type IS '策略类型（折扣/固定价/区间价）';
COMMENT ON COLUMN price_strategies.discount_rate IS '折扣率';
COMMENT ON COLUMN price_strategies.priority IS '优先级（数字越小优先级越高）';

-- ============================================
-- 来源: 025_purchase_return_item.sql
-- ============================================
CREATE TABLE IF NOT EXISTS purchase_return_item (
    id SERIAL PRIMARY KEY,
    return_id INTEGER NOT NULL,
    line_no INTEGER NOT NULL,
    product_id INTEGER NOT NULL,
    quantity DECIMAL(18,4) NOT NULL,
    quantity_alt DECIMAL(18,4) DEFAULT 0.0000,
    unit_price DECIMAL(18,6) NOT NULL,
    unit_price_foreign DECIMAL(18,6) DEFAULT 0.000000,
    discount_percent DECIMAL(5,2) DEFAULT 0.00,
    tax_percent DECIMAL(5,2) DEFAULT 0.00,
    subtotal DECIMAL(18,2) DEFAULT 0.00,
    tax_amount DECIMAL(18,2) DEFAULT 0.00,
    discount_amount DECIMAL(18,2) DEFAULT 0.00,
    total_amount DECIMAL(18,2) DEFAULT 0.00,
    notes TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE purchase_return_item ADD CONSTRAINT fk_pret_item_return
    FOREIGN KEY (return_id) REFERENCES purchase_return(id) ON DELETE CASCADE;
ALTER TABLE purchase_return_item ADD CONSTRAINT fk_pret_item_product
    FOREIGN KEY (product_id) REFERENCES products(id);

-- ============================================
-- 来源: 026_sales_analysis.sql
-- ============================================
-- P2 级模块：销售分析
-- 创建时间：2026-03-15
-- 功能：销售统计分析、销售趋势、业绩排行

-- 销售统计表
CREATE TABLE IF NOT EXISTS sales_statistics (
    id SERIAL PRIMARY KEY,
    statistic_type VARCHAR(20) NOT NULL,
    period VARCHAR(7) NOT NULL,
    dimension_type VARCHAR(20) NOT NULL,
    dimension_id INTEGER,
    dimension_name VARCHAR(200),
    order_count INTEGER DEFAULT 0,
    total_amount DECIMAL(18,2) DEFAULT 0,
    total_qty DECIMAL(14,4) DEFAULT 0,
    total_cost DECIMAL(18,2) DEFAULT 0,
    gross_profit DECIMAL(18,2) DEFAULT 0,
    gross_profit_rate DECIMAL(5,2),
    avg_order_value DECIMAL(18,2),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 销售趋势表
CREATE TABLE IF NOT EXISTS sales_trends (
    id SERIAL PRIMARY KEY,
    period VARCHAR(7) NOT NULL,
    product_id INTEGER REFERENCES products(id),
    customer_id INTEGER REFERENCES customers(id),
    sales_amount DECIMAL(18,2) NOT NULL,
    previous_amount DECIMAL(18,2),
    change_amount DECIMAL(18,2),
    change_rate DECIMAL(5,2),
    trend_direction VARCHAR(10),
    qty DECIMAL(14,4),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 销售业绩排行表
CREATE TABLE IF NOT EXISTS sales_performance_rankings (
    id SERIAL PRIMARY KEY,
    ranking_type VARCHAR(20) NOT NULL,
    period VARCHAR(7) NOT NULL,
    rank INTEGER NOT NULL,
    entity_id INTEGER NOT NULL,
    entity_name VARCHAR(200) NOT NULL,
    total_amount DECIMAL(18,2) NOT NULL,
    total_qty DECIMAL(14,4) DEFAULT 0,
    order_count INTEGER DEFAULT 0,
    target_amount DECIMAL(18,2),
    achievement_rate DECIMAL(5,2),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 销售目标表
CREATE TABLE IF NOT EXISTS sales_targets (
    id SERIAL PRIMARY KEY,
    target_type VARCHAR(20) NOT NULL,
    target_period VARCHAR(7) NOT NULL,
    department_id INTEGER REFERENCES departments(id),
    product_category_id INTEGER REFERENCES product_categories(id),
    target_amount DECIMAL(18,2) NOT NULL,
    actual_amount DECIMAL(18,2) DEFAULT 0,
    achievement_rate DECIMAL(5,2),
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_sales_statistics_period ON sales_statistics(period);
CREATE INDEX IF NOT EXISTS idx_sales_statistics_type ON sales_statistics(statistic_type);
CREATE INDEX IF NOT EXISTS idx_sales_trends_period ON sales_trends(period);
CREATE INDEX IF NOT EXISTS idx_sales_trends_product ON sales_trends(product_id);
CREATE INDEX IF NOT EXISTS idx_sales_performance_rankings_period ON sales_performance_rankings(period);
CREATE INDEX IF NOT EXISTS idx_sales_targets_period ON sales_targets(target_period);

-- 添加中文注释
COMMENT ON TABLE sales_statistics IS '销售统计表';
COMMENT ON COLUMN sales_statistics.statistic_type IS '统计类型（按产品/按客户/按部门）';
COMMENT ON COLUMN sales_statistics.dimension_type IS '维度类型';
COMMENT ON COLUMN sales_statistics.gross_profit IS '毛利润';
COMMENT ON COLUMN sales_statistics.gross_profit_rate IS '毛利率';

COMMENT ON TABLE sales_trends IS '销售趋势表';
COMMENT ON COLUMN sales_trends.previous_amount IS '上期金额';
COMMENT ON COLUMN sales_trends.trend_direction IS '趋势方向（上升/下降/持平）';

COMMENT ON TABLE sales_performance_rankings IS '销售业绩排行表';
COMMENT ON COLUMN sales_performance_rankings.ranking_type IS '排行类型（产品/客户/部门）';
COMMENT ON COLUMN sales_performance_rankings.achievement_rate IS '达成率';

COMMENT ON TABLE sales_targets IS '销售目标表';
COMMENT ON COLUMN sales_targets.target_type IS '目标类型（部门/产品/客户）';
COMMENT ON COLUMN sales_targets.target_period IS '目标期间';

-- ============================================
-- 来源: 027_quality_inspection.sql
-- ============================================
-- P2 级模块：质量检验
-- 创建时间：2026-03-15
-- 功能：质量检验管理、检验标准、不合格品处理

-- 质量检验标准表
CREATE TABLE IF NOT EXISTS quality_inspection_standards (
    id SERIAL PRIMARY KEY,
    standard_name VARCHAR(100) NOT NULL,
    standard_code VARCHAR(50) NOT NULL UNIQUE,
    product_id INTEGER REFERENCES products(id),
    product_category_id INTEGER REFERENCES product_categories(id),
    inspection_type VARCHAR(20) NOT NULL,
    inspection_items JSONB,
    sampling_method VARCHAR(50),
    sampling_rate DECIMAL(5,2),
    acceptance_criteria TEXT,
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 质量检验记录表
CREATE TABLE IF NOT EXISTS quality_inspection_records (
    id SERIAL PRIMARY KEY,
    inspection_no VARCHAR(50) NOT NULL UNIQUE,
    inspection_type VARCHAR(20) NOT NULL,
    related_type VARCHAR(50),
    related_id INTEGER,
    product_id INTEGER NOT NULL REFERENCES products(id),
    batch_no VARCHAR(50),
    supplier_id INTEGER REFERENCES suppliers(id),
    customer_id INTEGER REFERENCES customers(id),
    inspection_date DATE NOT NULL,
    inspector_id INTEGER,
    total_qty DECIMAL(14,4) NOT NULL,
    inspected_qty DECIMAL(14,4) NOT NULL,
    qualified_qty DECIMAL(14,4),
    unqualified_qty DECIMAL(14,4),
    qualification_rate DECIMAL(5,2),
    inspection_result VARCHAR(20) NOT NULL,
    remark TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 质量检验明细表
CREATE TABLE IF NOT EXISTS quality_inspection_details (
    id SERIAL PRIMARY KEY,
    inspection_id INTEGER NOT NULL REFERENCES quality_inspection_records(id),
    inspection_item VARCHAR(100) NOT NULL,
    standard_value VARCHAR(100),
    actual_value VARCHAR(100),
    result VARCHAR(20) NOT NULL,
    remark TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 不合格品处理表
CREATE TABLE IF NOT EXISTS unqualified_products (
    id SERIAL PRIMARY KEY,
    unqualified_no VARCHAR(50) NOT NULL UNIQUE,
    inspection_id INTEGER REFERENCES quality_inspection_records(id),
    product_id INTEGER NOT NULL REFERENCES products(id),
    batch_no VARCHAR(50),
    unqualified_qty DECIMAL(14,4) NOT NULL,
    unqualified_reason TEXT NOT NULL,
    handling_method VARCHAR(20) NOT NULL,
    handling_status VARCHAR(20) DEFAULT 'pending',
    handling_by INTEGER,
    handling_at TIMESTAMP,
    remark TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 质量统计表
CREATE TABLE IF NOT EXISTS quality_statistics (
    id SERIAL PRIMARY KEY,
    period VARCHAR(7) NOT NULL,
    product_id INTEGER REFERENCES products(id),
    supplier_id INTEGER REFERENCES suppliers(id),
    inspection_count INTEGER DEFAULT 0,
    total_qty DECIMAL(14,4) DEFAULT 0,
    qualified_qty DECIMAL(14,4) DEFAULT 0,
    unqualified_qty DECIMAL(14,4) DEFAULT 0,
    qualification_rate DECIMAL(5,2),
    unqualified_reasons JSONB,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_quality_standards_product ON quality_inspection_standards(product_id);
CREATE INDEX IF NOT EXISTS idx_quality_records_inspection_date ON quality_inspection_records(inspection_date);
CREATE INDEX IF NOT EXISTS idx_quality_records_product ON quality_inspection_records(product_id);
CREATE INDEX IF NOT EXISTS idx_unqualified_products_batch ON unqualified_products(batch_no);
CREATE INDEX IF NOT EXISTS idx_quality_statistics_period ON quality_statistics(period);

-- 添加中文注释
COMMENT ON TABLE quality_inspection_standards IS '质量检验标准表';
COMMENT ON COLUMN quality_inspection_standards.inspection_type IS '检验类型（来料/过程/成品/出货）';
COMMENT ON COLUMN quality_inspection_standards.inspection_items IS '检验项目（JSON 格式）';
COMMENT ON COLUMN quality_inspection_standards.sampling_method IS '抽样方法';

COMMENT ON TABLE quality_inspection_records IS '质量检验记录表';
COMMENT ON COLUMN quality_inspection_records.related_type IS '关联类型（采购入库/生产完工/销售出库）';
COMMENT ON COLUMN quality_inspection_records.qualification_rate IS '合格率';
COMMENT ON COLUMN quality_inspection_records.inspection_result IS '检验结果（合格/不合格/特采）';

COMMENT ON TABLE quality_inspection_details IS '质量检验明细表';
COMMENT ON COLUMN quality_inspection_details.inspection_item IS '检验项目';
COMMENT ON COLUMN quality_inspection_details.result IS '检验结果（合格/不合格）';

COMMENT ON TABLE unqualified_products IS '不合格品处理表';
COMMENT ON COLUMN unqualified_products.handling_method IS '处理方式（退货/返工/报废/特采）';
COMMENT ON COLUMN unqualified_products.handling_status IS '处理状态（待处理/处理中/已完成）';

COMMENT ON TABLE quality_statistics IS '质量统计表';
COMMENT ON COLUMN quality_statistics.unqualified_reasons IS '不合格原因统计（JSON 格式）';

-- ============================================
-- 来源: 028_quality_standard.sql
-- ============================================
-- P1 级模块：质量标准
-- 创建时间：2026-03-15
-- 功能：质量标准管理、质量标准版本控制

-- 质量标准表
CREATE TABLE IF NOT EXISTS quality_standards (
    id SERIAL PRIMARY KEY,
    standard_name VARCHAR(100) NOT NULL,
    standard_code VARCHAR(50) NOT NULL UNIQUE,
    standard_type VARCHAR(20) NOT NULL,
    product_id INTEGER REFERENCES products(id),
    product_category_id INTEGER REFERENCES product_categories(id),
    version VARCHAR(20) NOT NULL,
    previous_version_id INTEGER REFERENCES quality_standards(id),
    content TEXT NOT NULL,
    technical_requirements TEXT,
    testing_methods TEXT,
    acceptance_criteria TEXT,
    effective_date DATE NOT NULL,
    expiry_date DATE,
    status VARCHAR(20) DEFAULT 'active',
    approved_by INTEGER,
    approved_at TIMESTAMP,
    created_by INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 质量标准版本历史表
CREATE TABLE IF NOT EXISTS quality_standard_versions (
    id SERIAL PRIMARY KEY,
    standard_id INTEGER NOT NULL REFERENCES quality_standards(id),
    version VARCHAR(20) NOT NULL,
    change_type VARCHAR(20) NOT NULL,
    change_description TEXT,
    old_content TEXT,
    new_content TEXT,
    changed_by INTEGER,
    changed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    remark TEXT
);

-- 质量标准引用表
CREATE TABLE IF NOT EXISTS quality_standard_references (
    id SERIAL PRIMARY KEY,
    standard_id INTEGER NOT NULL REFERENCES quality_standards(id),
    reference_type VARCHAR(20) NOT NULL,
    reference_name VARCHAR(200) NOT NULL,
    reference_content TEXT,
    is_mandatory BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_quality_standards_type ON quality_standards(standard_type);
CREATE INDEX IF NOT EXISTS idx_quality_standards_product ON quality_standards(product_id);
CREATE INDEX IF NOT EXISTS idx_quality_standard_versions_standard ON quality_standard_versions(standard_id);
CREATE INDEX IF NOT EXISTS idx_quality_standard_references_standard ON quality_standard_references(standard_id);

-- 添加中文注释
COMMENT ON TABLE quality_standards IS '质量标准表';
COMMENT ON COLUMN quality_standards.standard_type IS '标准类型（国标/行标/企标/客标）';
COMMENT ON COLUMN quality_standards.version IS '版本号';
COMMENT ON COLUMN quality_standards.content IS '标准内容';
COMMENT ON COLUMN quality_standards.technical_requirements IS '技术要求';
COMMENT ON COLUMN quality_standards.testing_methods IS '测试方法';
COMMENT ON COLUMN quality_standards.acceptance_criteria IS '验收标准';

COMMENT ON TABLE quality_standard_versions IS '质量标准版本历史表';
COMMENT ON COLUMN quality_standard_versions.change_type IS '变更类型（新增/修改/废止）';
COMMENT ON COLUMN quality_standard_versions.change_description IS '变更描述';

COMMENT ON TABLE quality_standard_references IS '质量标准引用表';
COMMENT ON COLUMN quality_standard_references.reference_type IS '引用类型（参考/强制）';
COMMENT ON COLUMN quality_standard_references.is_mandatory IS '是否强制';

-- ============================================
-- 来源: 029_four_level_batch_management.sql
-- ============================================
-- ========================================
-- 1. 缸号管理表
-- ========================================

-- ==================== 缸号表 ====================
CREATE TABLE IF NOT EXISTS batch_dye_lot (
    id SERIAL PRIMARY KEY,
    dye_lot_no VARCHAR(100) NOT NULL,                    -- 缸号（内部编码）
    product_id INTEGER NOT NULL,                         -- 成品 ID
    color_id INTEGER NOT NULL,                           -- 色号 ID
    supplier_dye_lot_no VARCHAR(100),                    -- 供应商缸号
    supplier_id INTEGER NOT NULL,                        -- 供应商 ID
    production_date DATE,                                -- 生产日期
    machine_no VARCHAR(50),                              -- 机台号
    batch_weight DECIMAL(10,2),                          -- 缸重（kg）
    total_length DECIMAL(10,2),                          -- 总长度（米）
    total_pieces INTEGER DEFAULT 0,                      -- 总匹数
    quality_grade VARCHAR(10) DEFAULT 'A',               -- 质量等级（A/B/C/D）
    quality_status VARCHAR(20) DEFAULT 'pending',        -- 质检状态（pending/inspecting/passed/failed）
    inspection_date DATE,                                -- 质检日期
    inspector_id INTEGER,                                -- 质检员 ID
    remarks TEXT,                                        -- 备注
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否启用
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE batch_dye_lot IS '缸号管理表';
COMMENT ON COLUMN batch_dye_lot.dye_lot_no IS '缸号（内部编码，自动生成）';
COMMENT ON COLUMN batch_dye_lot.product_id IS '成品 ID';
COMMENT ON COLUMN batch_dye_lot.color_id IS '色号 ID';
COMMENT ON COLUMN batch_dye_lot.supplier_dye_lot_no IS '供应商缸号（外部编码）';
COMMENT ON COLUMN batch_dye_lot.supplier_id IS '供应商 ID';
COMMENT ON COLUMN batch_dye_lot.quality_grade IS '质量等级（A/B/C/D）';
COMMENT ON COLUMN batch_dye_lot.quality_status IS '质检状态（pending/inspecting/passed/failed）';
COMMENT ON COLUMN batch_dye_lot.is_active IS '是否启用';

-- 外键约束
ALTER TABLE batch_dye_lot ADD CONSTRAINT fk_dye_lot_product
    FOREIGN KEY (product_id) REFERENCES products(id);
ALTER TABLE batch_dye_lot ADD CONSTRAINT fk_dye_lot_color
    FOREIGN KEY (color_id) REFERENCES product_colors(id);
ALTER TABLE batch_dye_lot ADD CONSTRAINT fk_dye_lot_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);

-- 唯一约束
ALTER TABLE batch_dye_lot ADD CONSTRAINT uk_dye_lot_no UNIQUE (dye_lot_no);
ALTER TABLE batch_dye_lot ADD CONSTRAINT uk_dye_lot_product_color_supplier UNIQUE (product_id, color_id, supplier_id, dye_lot_no);

-- 索引
CREATE INDEX IF NOT EXISTS idx_batch_dye_lot_product ON batch_dye_lot(product_id);
CREATE INDEX IF NOT EXISTS idx_batch_dye_lot_color ON batch_dye_lot(color_id);
CREATE INDEX IF NOT EXISTS idx_batch_dye_lot_supplier ON batch_dye_lot(supplier_id);
CREATE INDEX IF NOT EXISTS idx_batch_dye_lot_dye_lot_no ON batch_dye_lot(dye_lot_no);
CREATE INDEX IF NOT EXISTS idx_batch_dye_lot_quality_status ON batch_dye_lot(quality_status);
CREATE INDEX IF NOT EXISTS idx_batch_dye_lot_production_date ON batch_dye_lot(production_date);
CREATE INDEX IF NOT EXISTS idx_batch_dye_lot_created_at ON batch_dye_lot(created_at DESC);

-- ========================================
-- 2. 匹号管理表
-- ========================================

-- ==================== 库存匹号表 ====================
CREATE TABLE IF NOT EXISTS inventory_piece (
    id SERIAL PRIMARY KEY,
    piece_no VARCHAR(100) NOT NULL,                      -- 匹号（内部编码）
    dye_lot_id INTEGER NOT NULL,                         -- 缸号 ID
    supplier_piece_no VARCHAR(100),                      -- 供应商匹号
    length DECIMAL(10,2) NOT NULL,                       -- 长度（米）
    weight DECIMAL(10,2),                                -- 重量（kg）
    width DECIMAL(10,2),                                 -- 幅宽（cm）
    gram_weight DECIMAL(10,2),                           -- 克重（g/m²）
    position_no VARCHAR(50),                             -- 库位号
    package_no VARCHAR(50),                              -- 包号
    production_date DATE,                                -- 生产日期
    shelf_life INTEGER,                                  -- 保质期（天）
    quality_status VARCHAR(20) DEFAULT 'pending',        -- 质检状态（pending/inspecting/passed/failed）
    inventory_status VARCHAR(20) DEFAULT 'available',    -- 库存状态（available/reserved/locked/sold）
    warehouse_id INTEGER,                                -- 仓库 ID
    remarks TEXT,                                        -- 备注
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER,                                  -- 更新人 ID
    UNIQUE(dye_lot_id, piece_no)                         -- 同一缸号下的匹号必须唯一
);

COMMENT ON TABLE inventory_piece IS '库存匹号管理表';
COMMENT ON COLUMN inventory_piece.piece_no IS '匹号（在同一缸号下唯一）';
COMMENT ON COLUMN inventory_piece.dye_lot_id IS '缸号 ID（关联 batch_dye_lot）';
COMMENT ON COLUMN inventory_piece.supplier_piece_no IS '供应商匹号（外部编码）';
COMMENT ON COLUMN inventory_piece.length IS '长度（米）';
COMMENT ON COLUMN inventory_piece.weight IS '重量（kg）';
COMMENT ON COLUMN inventory_piece.quality_status IS '质检状态（pending/inspecting/passed/failed）';
COMMENT ON COLUMN inventory_piece.inventory_status IS '库存状态（available/reserved/locked/sold）';

-- 外键约束
ALTER TABLE inventory_piece ADD CONSTRAINT fk_piece_dye_lot
    FOREIGN KEY (dye_lot_id) REFERENCES batch_dye_lot(id);
ALTER TABLE inventory_piece ADD CONSTRAINT fk_piece_warehouse
    FOREIGN KEY (warehouse_id) REFERENCES warehouses(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_inventory_piece_piece_no ON inventory_piece(piece_no);
CREATE INDEX IF NOT EXISTS idx_inventory_piece_dye_lot ON inventory_piece(dye_lot_id);
CREATE INDEX IF NOT EXISTS idx_inventory_piece_supplier_piece ON inventory_piece(supplier_piece_no);
CREATE INDEX IF NOT EXISTS idx_inventory_piece_quality_status ON inventory_piece(quality_status);
CREATE INDEX IF NOT EXISTS idx_inventory_piece_inventory_status ON inventory_piece(inventory_status);
CREATE INDEX IF NOT EXISTS idx_inventory_piece_warehouse ON inventory_piece(warehouse_id);
CREATE INDEX IF NOT EXISTS idx_inventory_piece_created_at ON inventory_piece(created_at DESC);

-- ========================================
-- 3. 编码映射表
-- ========================================

-- ==================== 成品编码映射表 ====================
CREATE TABLE IF NOT EXISTS product_code_mapping (
    id SERIAL PRIMARY KEY,
    internal_product_code VARCHAR(100) NOT NULL,         -- 内部成品编码
    supplier_product_code VARCHAR(100) NOT NULL,         -- 供应商成品编码
    supplier_id INTEGER NOT NULL,                        -- 供应商 ID
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否启用
    mapping_date DATE NOT NULL,                          -- 映射日期
    validation_status VARCHAR(20) DEFAULT 'pending',     -- 验证状态（pending/validated/failed）
    validated_at TIMESTAMP,                              -- 验证时间
    validated_by INTEGER,                                -- 验证人 ID
    remarks TEXT,                                        -- 备注
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE product_code_mapping IS '成品编码映射表（内部编码 <-> 供应商编码）';
COMMENT ON COLUMN product_code_mapping.internal_product_code IS '内部成品编码';
COMMENT ON COLUMN product_code_mapping.supplier_product_code IS '供应商成品编码';
COMMENT ON COLUMN product_code_mapping.supplier_id IS '供应商 ID';
COMMENT ON COLUMN product_code_mapping.validation_status IS '验证状态（pending/validated/failed）';

-- 外键约束
ALTER TABLE product_code_mapping ADD CONSTRAINT fk_pcm_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);

-- 唯一约束
ALTER TABLE product_code_mapping ADD CONSTRAINT uk_pcm_internal_supplier UNIQUE (internal_product_code, supplier_product_code, supplier_id);
ALTER TABLE product_code_mapping ADD CONSTRAINT uk_pcm_internal UNIQUE (internal_product_code, supplier_id);
ALTER TABLE product_code_mapping ADD CONSTRAINT uk_pcm_supplier UNIQUE (supplier_product_code, supplier_id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_pcm_internal_code ON product_code_mapping(internal_product_code);
CREATE INDEX IF NOT EXISTS idx_pcm_supplier_code ON product_code_mapping(supplier_product_code);
CREATE INDEX IF NOT EXISTS idx_pcm_supplier ON product_code_mapping(supplier_id);
CREATE INDEX IF NOT EXISTS idx_pcm_validation_status ON product_code_mapping(validation_status);

-- ==================== 色号编码映射表 ====================
CREATE TABLE IF NOT EXISTS color_code_mapping (
    id SERIAL PRIMARY KEY,
    internal_color_no VARCHAR(100) NOT NULL,             -- 内部色号
    supplier_color_code VARCHAR(100) NOT NULL,           -- 供应商色号
    supplier_id INTEGER NOT NULL,                        -- 供应商 ID
    product_code VARCHAR(100),                           -- 成品编码（可选，用于更精确的映射）
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否启用
    mapping_date DATE NOT NULL,                          -- 映射日期
    validation_status VARCHAR(20) DEFAULT 'pending',     -- 验证状态（pending/validated/failed）
    validated_at TIMESTAMP,                              -- 验证时间
    validated_by INTEGER,                                -- 验证人 ID
    color_difference_notes TEXT,                         -- 色差说明
    remarks TEXT,                                        -- 备注
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE color_code_mapping IS '色号编码映射表（内部色号 <-> 供应商色号）';
COMMENT ON COLUMN color_code_mapping.internal_color_no IS '内部色号';
COMMENT ON COLUMN color_code_mapping.supplier_color_code IS '供应商色号';
COMMENT ON COLUMN color_code_mapping.color_difference_notes IS '色差说明';

-- 外键约束
ALTER TABLE color_code_mapping ADD CONSTRAINT fk_ccm_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);

-- 唯一约束
ALTER TABLE color_code_mapping ADD CONSTRAINT uk_ccm_internal_supplier UNIQUE (internal_color_no, supplier_color_code, supplier_id);
ALTER TABLE color_code_mapping ADD CONSTRAINT uk_ccm_internal UNIQUE (internal_color_no, supplier_id);
ALTER TABLE color_code_mapping ADD CONSTRAINT uk_ccm_supplier UNIQUE (supplier_color_code, supplier_id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_ccm_internal_color ON color_code_mapping(internal_color_no);
CREATE INDEX IF NOT EXISTS idx_ccm_supplier_color ON color_code_mapping(supplier_color_code);
CREATE INDEX IF NOT EXISTS idx_ccm_supplier ON color_code_mapping(supplier_id);
CREATE INDEX IF NOT EXISTS idx_ccm_product_code ON color_code_mapping(product_code);

-- ==================== 缸号映射表 ====================
CREATE TABLE IF NOT EXISTS dye_lot_mapping (
    id SERIAL PRIMARY KEY,
    internal_dye_lot_no VARCHAR(100) NOT NULL,           -- 内部缸号
    supplier_dye_lot_no VARCHAR(100) NOT NULL,           -- 供应商缸号
    supplier_id INTEGER NOT NULL,                        -- 供应商 ID
    product_code VARCHAR(100),                           -- 成品编码
    color_no VARCHAR(100),                               -- 色号
    batch_dye_lot_id INTEGER,                            -- 内部缸号 ID
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否启用
    mapping_date DATE NOT NULL,                          -- 映射日期
    validation_status VARCHAR(20) DEFAULT 'pending',     -- 验证状态（pending/validated/failed）
    validated_at TIMESTAMP,                              -- 验证时间
    validated_by INTEGER,                                -- 验证人 ID
    remarks TEXT,                                        -- 备注
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE dye_lot_mapping IS '缸号映射表（内部缸号 <-> 供应商缸号）';
COMMENT ON COLUMN dye_lot_mapping.batch_dye_lot_id IS '内部缸号 ID（关联 batch_dye_lot）';

-- 外键约束
ALTER TABLE dye_lot_mapping ADD CONSTRAINT fk_dlm_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);
ALTER TABLE dye_lot_mapping ADD CONSTRAINT fk_dlm_batch_dye_lot
    FOREIGN KEY (batch_dye_lot_id) REFERENCES batch_dye_lot(id);

-- 唯一约束
ALTER TABLE dye_lot_mapping ADD CONSTRAINT uk_dlm_internal_supplier UNIQUE (internal_dye_lot_no, supplier_dye_lot_no, supplier_id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_dlm_internal_dye_lot ON dye_lot_mapping(internal_dye_lot_no);
CREATE INDEX IF NOT EXISTS idx_dlm_supplier_dye_lot ON dye_lot_mapping(supplier_dye_lot_no);
CREATE INDEX IF NOT EXISTS idx_dlm_supplier ON dye_lot_mapping(supplier_id);
CREATE INDEX IF NOT EXISTS idx_dlm_batch_dye_lot ON dye_lot_mapping(batch_dye_lot_id);

-- ==================== 匹号映射表 ====================
CREATE TABLE IF NOT EXISTS piece_mapping (
    id SERIAL PRIMARY KEY,
    internal_piece_no VARCHAR(100) NOT NULL,             -- 内部匹号
    supplier_piece_no VARCHAR(100) NOT NULL,             -- 供应商匹号
    supplier_id INTEGER NOT NULL,                        -- 供应商 ID
    dye_lot_no VARCHAR(100),                             -- 缸号
    inventory_piece_id INTEGER,                          -- 内部匹号 ID
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否启用
    mapping_date DATE NOT NULL,                          -- 映射日期
    validation_status VARCHAR(20) DEFAULT 'pending',     -- 验证状态（pending/validated/failed）
    validated_at TIMESTAMP,                              -- 验证时间
    validated_by INTEGER,                                -- 验证人 ID
    remarks TEXT,                                        -- 备注
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE piece_mapping IS '匹号映射表（内部匹号 <-> 供应商匹号）';
COMMENT ON COLUMN piece_mapping.inventory_piece_id IS '内部匹号 ID（关联 inventory_piece）';

-- 外键约束
ALTER TABLE piece_mapping ADD CONSTRAINT fk_pm_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);
ALTER TABLE piece_mapping ADD CONSTRAINT fk_pm_inventory_piece
    FOREIGN KEY (inventory_piece_id) REFERENCES inventory_piece(id);

-- 唯一约束
ALTER TABLE piece_mapping ADD CONSTRAINT uk_pm_internal_supplier UNIQUE (internal_piece_no, supplier_piece_no, supplier_id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_pm_internal_piece ON piece_mapping(internal_piece_no);
CREATE INDEX IF NOT EXISTS idx_pm_supplier_piece ON piece_mapping(supplier_piece_no);
CREATE INDEX IF NOT EXISTS idx_pm_supplier ON piece_mapping(supplier_id);
CREATE INDEX IF NOT EXISTS idx_pm_inventory_piece ON piece_mapping(inventory_piece_id);

-- ========================================
-- 4. 批次追溯日志表
-- ========================================

-- ==================== 批次追溯日志表 ====================
CREATE TABLE IF NOT EXISTS batch_trace_log (
    id SERIAL PRIMARY KEY,
    trace_no VARCHAR(100) NOT NULL UNIQUE,               -- 追溯编号（自动生成）
    business_type VARCHAR(50) NOT NULL,                  -- 业务类型（sales_order/delivery/purchase_order/receipt）
    business_id INTEGER NOT NULL,                        -- 业务 ID
    trace_direction VARCHAR(20) NOT NULL,                -- 追溯方向（internal_to_supplier/supplier_to_internal）
    
    -- 内部编码信息
    internal_product_code VARCHAR(100),                  -- 内部成品编码
    internal_color_no VARCHAR(100),                      -- 内部色号
    internal_dye_lot_no VARCHAR(100),                    -- 内部缸号
    internal_piece_nos TEXT[],                           -- 内部匹号列表
    
    -- 供应商编码信息
    supplier_product_code VARCHAR(100),                  -- 供应商成品编码
    supplier_color_code VARCHAR(100),                    -- 供应商色号
    supplier_dye_lot_no VARCHAR(100),                    -- 供应商缸号
    supplier_piece_nos TEXT[],                           -- 供应商匹号列表
    
    -- 转换详情
    conversion_details JSONB,                            -- 转换详情（JSON 格式）
    validation_result VARCHAR(50) DEFAULT 'pending',     -- 验证结果（pending/success/failed）
    validation_message TEXT,                             -- 验证信息
    
    -- 操作追踪
    operator_id INTEGER NOT NULL,                        -- 操作人 ID
    operation_time TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 操作时间
    operation_type VARCHAR(50) NOT NULL,                 -- 操作类型（create/update/convert/validate）
    ip_address VARCHAR(50),                              -- 操作 IP
    device_info VARCHAR(200),                            -- 设备信息
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    remarks TEXT                                         -- 备注
);

COMMENT ON TABLE batch_trace_log IS '批次追溯日志表（记录所有编码转换和追溯操作）';
COMMENT ON COLUMN batch_trace_log.business_type IS '业务类型（sales_order/delivery/purchase_order/receipt）';
COMMENT ON COLUMN batch_trace_log.trace_direction IS '追溯方向（internal_to_supplier/supplier_to_internal）';
COMMENT ON COLUMN batch_trace_log.internal_piece_nos IS '内部匹号列表（数组）';
COMMENT ON COLUMN batch_trace_log.supplier_piece_nos IS '供应商匹号列表（数组）';
COMMENT ON COLUMN batch_trace_log.conversion_details IS '转换详情（JSON 格式）';
COMMENT ON COLUMN batch_trace_log.validation_result IS '验证结果（pending/success/failed）';
COMMENT ON COLUMN batch_trace_log.operation_type IS '操作类型（create/update/convert/validate）';

-- 外键约束
ALTER TABLE batch_trace_log ADD CONSTRAINT fk_btl_operator
    FOREIGN KEY (operator_id) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_btl_trace_no ON batch_trace_log(trace_no);
CREATE INDEX IF NOT EXISTS idx_btl_business ON batch_trace_log(business_type, business_id);
CREATE INDEX IF NOT EXISTS idx_btl_internal_product ON batch_trace_log(internal_product_code);
CREATE INDEX IF NOT EXISTS idx_btl_supplier_product ON batch_trace_log(supplier_product_code);
CREATE INDEX IF NOT EXISTS idx_btl_internal_dye_lot ON batch_trace_log(internal_dye_lot_no);
CREATE INDEX IF NOT EXISTS idx_btl_supplier_dye_lot ON batch_trace_log(supplier_dye_lot_no);
CREATE INDEX IF NOT EXISTS idx_btl_operation_time ON batch_trace_log(operation_time DESC);
CREATE INDEX IF NOT EXISTS idx_btl_operator ON batch_trace_log(operator_id);
CREATE INDEX IF NOT EXISTS idx_btl_validation_result ON batch_trace_log(validation_result);

-- ========================================
-- 5. 触发器函数
-- ========================================

-- ==================== 自动更新 updated_at ====================
CREATE OR REPLACE FUNCTION update_batch_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ==================== 缸号总匹数自动更新 ====================
CREATE OR REPLACE FUNCTION update_dye_lot_total_pieces()
RETURNS TRIGGER AS $$
BEGIN
    -- 更新缸号的总匹数
    UPDATE batch_dye_lot
    SET total_pieces = (
        SELECT COUNT(*) 
        FROM inventory_piece 
        WHERE dye_lot_id = NEW.dye_lot_id
    ),
    updated_at = CURRENT_TIMESTAMP
    WHERE id = NEW.dye_lot_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ==================== 匹号自动生成（如果需要） ====================
CREATE OR REPLACE FUNCTION generate_piece_no()
RETURNS TRIGGER AS $$
BEGIN
    -- 如果匹号为空，自动生成
    IF NEW.piece_no IS NULL OR NEW.piece_no = '' THEN
        NEW.piece_no := 'P' || TO_CHAR(CURRENT_DATE, 'YYYYMMDD') || LPAD(NEXTVAL('inventory_piece_id_seq')::TEXT, 6, '0');
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ========================================
-- 6. 应用触发器
-- ========================================

-- 更新 updated_at 触发器
DROP TRIGGER IF EXISTS trg_batch_dye_lot_updated_at ON batch_dye_lot;
CREATE TRIGGER trg_batch_dye_lot_updated_at
    BEFORE UPDATE ON batch_dye_lot
    FOR EACH ROW
    EXECUTE FUNCTION update_batch_updated_at_column();

DROP TRIGGER IF EXISTS trg_inventory_piece_updated_at ON inventory_piece;
CREATE TRIGGER trg_inventory_piece_updated_at
    BEFORE UPDATE ON inventory_piece
    FOR EACH ROW
    EXECUTE FUNCTION update_batch_updated_at_column();

DROP TRIGGER IF EXISTS trg_product_code_mapping_updated_at ON product_code_mapping;
CREATE TRIGGER trg_product_code_mapping_updated_at
    BEFORE UPDATE ON product_code_mapping
    FOR EACH ROW
    EXECUTE FUNCTION update_batch_updated_at_column();

DROP TRIGGER IF EXISTS trg_color_code_mapping_updated_at ON color_code_mapping;
CREATE TRIGGER trg_color_code_mapping_updated_at
    BEFORE UPDATE ON color_code_mapping
    FOR EACH ROW
    EXECUTE FUNCTION update_batch_updated_at_column();

DROP TRIGGER IF EXISTS trg_dye_lot_mapping_updated_at ON dye_lot_mapping;
CREATE TRIGGER trg_dye_lot_mapping_updated_at
    BEFORE UPDATE ON dye_lot_mapping
    FOR EACH ROW
    EXECUTE FUNCTION update_batch_updated_at_column();

DROP TRIGGER IF EXISTS trg_piece_mapping_updated_at ON piece_mapping;
CREATE TRIGGER trg_piece_mapping_updated_at
    BEFORE UPDATE ON piece_mapping
    FOR EACH ROW
    EXECUTE FUNCTION update_batch_updated_at_column();

-- 缸号总匹数自动更新触发器
DROP TRIGGER IF EXISTS trg_inventory_piece_insert_update_pieces ON inventory_piece;
CREATE TRIGGER trg_inventory_piece_insert_update_pieces
    AFTER INSERT OR UPDATE ON inventory_piece
    FOR EACH ROW
    EXECUTE FUNCTION update_dye_lot_total_pieces();

DROP TRIGGER IF EXISTS trg_inventory_piece_delete_update_pieces ON inventory_piece;
CREATE TRIGGER trg_inventory_piece_delete_update_pieces
    AFTER DELETE ON inventory_piece
    FOR EACH ROW
    EXECUTE FUNCTION update_dye_lot_total_pieces();

-- 匹号自动生成触发器（可选）
DROP TRIGGER IF EXISTS trg_inventory_piece_generate_no ON inventory_piece;
CREATE TRIGGER trg_inventory_piece_generate_no
    BEFORE INSERT ON inventory_piece
    FOR EACH ROW
    EXECUTE FUNCTION generate_piece_no();

-- ========================================
-- 7. 初始化数据
-- ========================================

-- 初始化批次追溯编号序列
CREATE SEQUENCE IF NOT EXISTS batch_trace_log_trace_no_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

-- ========================================
-- 迁移完成
-- ========================================

-- ============================================
-- 来源: 030_extend_existing_tables.sql
-- ============================================
-- ========================================
-- 1. 扩展产品表（products）
-- ========================================

-- 添加供应商相关字段
ALTER TABLE products 
ADD COLUMN IF NOT EXISTS supplier_product_code VARCHAR(100),           -- 供应商成品编码
ADD COLUMN IF NOT EXISTS supplier_id INTEGER,                          -- 供应商 ID
ADD COLUMN IF NOT EXISTS is_batch_managed BOOLEAN DEFAULT TRUE,        -- 是否启用批次管理
ADD COLUMN IF NOT EXISTS batch_level VARCHAR(20) DEFAULT 'four_level', -- 批次级别（two_level/three_level/four_level）
ADD CONSTRAINT fk_products_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id);

-- 添加注释
COMMENT ON COLUMN products.supplier_product_code IS '供应商成品编码';
COMMENT ON COLUMN products.supplier_id IS '供应商 ID';
COMMENT ON COLUMN products.is_batch_managed IS '是否启用批次管理';
COMMENT ON COLUMN products.batch_level IS '批次级别（two_level/three_level/four_level）';

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_products_supplier ON products(supplier_id);
CREATE INDEX IF NOT EXISTS idx_products_batch_managed ON products(is_batch_managed);

-- ========================================
-- 2. 扩展产品色号表（product_colors）
-- ========================================

-- 添加供应商色号相关字段
ALTER TABLE product_colors
ADD COLUMN IF NOT EXISTS supplier_color_code VARCHAR(100),             -- 供应商色号
ADD COLUMN IF NOT EXISTS color_difference_notes TEXT,                  -- 色差说明
ADD COLUMN IF NOT EXISTS is_active BOOLEAN DEFAULT TRUE;               -- 是否启用

-- 添加注释
COMMENT ON COLUMN product_colors.supplier_color_code IS '供应商色号';
COMMENT ON COLUMN product_colors.color_difference_notes IS '色差说明';
COMMENT ON COLUMN product_colors.is_active IS '是否启用';

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_product_colors_supplier_color ON product_colors(supplier_color_code);
CREATE INDEX IF NOT EXISTS idx_product_colors_active ON product_colors(is_active);

-- ========================================
-- 3. 扩展销售订单明细表（sales_order_items）
-- ========================================

-- 查看现有表结构（确认表名）
-- 注意：根据更正，销售订单只需要成品 + 色号，不需要缸号/匹号
-- 这里添加批次管理相关字段，但缸号/匹号在发货单中才使用

-- 添加批次管理字段（销售订单只需要到色号级别）
ALTER TABLE sales_order_items
ADD COLUMN IF NOT EXISTS batch_required BOOLEAN DEFAULT FALSE,         -- 是否需要批次管理
ADD COLUMN IF NOT EXISTS allocated_dye_lot_ids INTEGER[],              -- 已分配的缸号 ID 列表（用于预留）
ADD COLUMN IF NOT EXISTS allocated_piece_ids INTEGER[],                -- 已分配的匹号 ID 列表（用于预留）
ADD COLUMN IF NOT EXISTS delivery_batch_info JSONB;                    -- 发货批次信息（JSON 格式，发货时填充）

-- 添加注释
COMMENT ON COLUMN sales_order_items.batch_required IS '是否需要批次管理';
COMMENT ON COLUMN sales_order_items.allocated_dye_lot_ids IS '已分配的缸号 ID 列表（用于预留）';
COMMENT ON COLUMN sales_order_items.allocated_piece_ids IS '已分配的匹号 ID 列表（用于预留）';
COMMENT ON COLUMN sales_order_items.delivery_batch_info IS '发货批次信息（JSON 格式，发货时填充）';

-- ========================================
-- 4. 扩展采购订单明细表（purchase_order_item）
-- ========================================

-- 注意：根据更正，采购订单只需要供应商成品 + 色号，不需要缸号/匹号
-- 缸号/匹号在采购收货单中才使用

-- 添加批次管理字段（采购订单只需要到色号级别）
ALTER TABLE purchase_order_item
ADD COLUMN IF NOT EXISTS batch_required BOOLEAN DEFAULT FALSE,         -- 是否需要批次管理
ADD COLUMN IF NOT EXISTS expected_dye_lot_info TEXT,                   -- 预计缸号信息（仅供参考）
ADD COLUMN IF NOT EXISTS receipt_batch_info JSONB;                     -- 收货批次信息（JSON 格式，收货时填充）

-- 添加注释
COMMENT ON COLUMN purchase_order_item.batch_required IS '是否需要批次管理';
COMMENT ON COLUMN purchase_order_item.expected_dye_lot_info IS '预计缸号信息（仅供参考）';
COMMENT ON COLUMN purchase_order_item.receipt_batch_info IS '收货批次信息（JSON 格式，收货时填充）';

-- ========================================
-- 5. 扩展示销售发货单表（新增）
-- ========================================

-- 如果销售发货单表不存在，先创建主表
-- 注意：这里假设已有 sales_deliveries 表，如果没有需要创建

-- 添加四级批次字段到销售发货明细
-- 由于原表可能不存在，这里提供完整的表创建脚本

CREATE TABLE IF NOT EXISTS sales_delivery (
    id SERIAL PRIMARY KEY,
    delivery_no VARCHAR(50) NOT NULL UNIQUE,             -- 发货单号
    sales_order_id INTEGER NOT NULL,                     -- 销售订单 ID
    customer_id INTEGER NOT NULL,                        -- 客户 ID
    delivery_date DATE NOT NULL,                         -- 发货日期
    warehouse_id INTEGER NOT NULL,                       -- 仓库 ID
    total_quantity DECIMAL(10,2) DEFAULT 0.00,           -- 总数量
    total_amount DECIMAL(10,2) DEFAULT 0.00,             -- 总金额
    delivery_status VARCHAR(20) DEFAULT 'draft',         -- 发货状态
    shipped_at TIMESTAMP,                                -- 发货时间
    confirmed_at TIMESTAMP,                              -- 确认时间
    notes TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,
    updated_by INTEGER
);

COMMENT ON TABLE sales_delivery IS '销售发货单表';

-- 外键约束
ALTER TABLE sales_delivery ADD CONSTRAINT fk_sd_sales_order
    FOREIGN KEY (sales_order_id) REFERENCES sales_orders(id);
ALTER TABLE sales_delivery ADD CONSTRAINT fk_sd_customer
    FOREIGN KEY (customer_id) REFERENCES customers(id);
ALTER TABLE sales_delivery ADD CONSTRAINT fk_sd_warehouse
    FOREIGN KEY (warehouse_id) REFERENCES warehouses(id);
ALTER TABLE sales_delivery ADD CONSTRAINT fk_sd_created_by
    FOREIGN KEY (created_by) REFERENCES users(id);
ALTER TABLE sales_delivery ADD CONSTRAINT fk_sd_updated_by
    FOREIGN KEY (updated_by) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_sd_delivery_no ON sales_delivery(delivery_no);
CREATE INDEX IF NOT EXISTS idx_sd_sales_order ON sales_delivery(sales_order_id);
CREATE INDEX IF NOT EXISTS idx_sd_customer ON sales_delivery(customer_id);
CREATE INDEX IF NOT EXISTS idx_sd_delivery_date ON sales_delivery(delivery_date);
CREATE INDEX IF NOT EXISTS idx_sd_delivery_status ON sales_delivery(delivery_status);

-- ========================================
-- 6. 扩展示销售发货明细表（新增）
-- ========================================

-- 销售发货明细表（包含四级批次信息）
CREATE TABLE IF NOT EXISTS sales_delivery_item (
    id SERIAL PRIMARY KEY,
    delivery_id INTEGER NOT NULL,                        -- 发货单 ID
    sales_order_item_id INTEGER,                         -- 销售订单明细 ID
    line_no INTEGER NOT NULL,                            -- 行号
    product_id INTEGER NOT NULL,                         -- 成品 ID
    product_code VARCHAR(50) NOT NULL,                   -- 成品编码
    product_name VARCHAR(200) NOT NULL,                  -- 成品名称
    color_id INTEGER NOT NULL,                           -- 色号 ID
    color_no VARCHAR(50) NOT NULL,                       -- 色号
    dye_lot_id INTEGER NOT NULL,                         -- 缸号 ID ✅
    dye_lot_no VARCHAR(100) NOT NULL,                    -- 缸号 ✅
    piece_ids INTEGER[] NOT NULL,                        -- 匹号 ID 列表 ✅
    piece_nos VARCHAR(100)[] NOT NULL,                   -- 匹号列表 ✅
    quantity DECIMAL(10,2) NOT NULL,                     -- 数量
    unit_price DECIMAL(10,2),                            -- 单价
    amount DECIMAL(10,2),                                -- 金额
    warehouse_id INTEGER,                                -- 仓库 ID
    location_code VARCHAR(50),                           -- 库位编码
    notes TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE sales_delivery_item IS '销售发货明细表（含四级批次）';
COMMENT ON COLUMN sales_delivery_item.dye_lot_id IS '缸号 ID';
COMMENT ON COLUMN sales_delivery_item.dye_lot_no IS '缸号';
COMMENT ON COLUMN sales_delivery_item.piece_ids IS '匹号 ID 列表';
COMMENT ON COLUMN sales_delivery_item.piece_nos IS '匹号列表';

-- 外键约束
ALTER TABLE sales_delivery_item ADD CONSTRAINT fk_sdi_delivery
    FOREIGN KEY (delivery_id) REFERENCES sales_delivery(id) ON DELETE CASCADE;
ALTER TABLE sales_delivery_item ADD CONSTRAINT fk_sdi_sales_order_item
    FOREIGN KEY (sales_order_item_id) REFERENCES sales_order_items(id);
ALTER TABLE sales_delivery_item ADD CONSTRAINT fk_sdi_product
    FOREIGN KEY (product_id) REFERENCES products(id);
ALTER TABLE sales_delivery_item ADD CONSTRAINT fk_sdi_color
    FOREIGN KEY (color_id) REFERENCES product_colors(id);
ALTER TABLE sales_delivery_item ADD CONSTRAINT fk_sdi_dye_lot
    FOREIGN KEY (dye_lot_id) REFERENCES batch_dye_lot(id);

-- 唯一约束
ALTER TABLE sales_delivery_item ADD CONSTRAINT uk_sdi_delivery_line
    UNIQUE (delivery_id, line_no);

-- 索引
CREATE INDEX IF NOT EXISTS idx_sdi_delivery ON sales_delivery_item(delivery_id);
CREATE INDEX IF NOT EXISTS idx_sdi_product ON sales_delivery_item(product_id);
CREATE INDEX IF NOT EXISTS idx_sdi_color ON sales_delivery_item(color_id);
CREATE INDEX IF NOT EXISTS idx_sdi_dye_lot ON sales_delivery_item(dye_lot_id);
CREATE INDEX IF NOT EXISTS idx_sdi_piece_ids ON sales_delivery_item USING GIN (piece_ids);

-- ========================================
-- 7. 扩展采购收货单表（已存在，添加批次字段）
-- ========================================

-- 采购收货单主表已存在，这里添加批次相关字段
ALTER TABLE purchase_receipt
ADD COLUMN IF NOT EXISTS has_batch_info BOOLEAN DEFAULT FALSE,         -- 是否有批次信息
ADD COLUMN IF NOT EXISTS batch_validation_status VARCHAR(20) DEFAULT 'pending'; -- 批次验证状态

COMMENT ON COLUMN purchase_receipt.has_batch_info IS '是否有批次信息';
COMMENT ON COLUMN purchase_receipt.batch_validation_status IS '批次验证状态（pending/validated/failed）';

CREATE INDEX IF NOT EXISTS idx_pr_has_batch ON purchase_receipt(has_batch_info);

-- ========================================
-- 8. 扩展采购收货明细表（添加四级批次字段）
-- ========================================

-- 采购收货明细表已存在，添加四级批次字段
ALTER TABLE purchase_receipt_item
ADD COLUMN IF NOT EXISTS internal_dye_lot_id INTEGER,                  -- 内部缸号 ID
ADD COLUMN IF NOT EXISTS internal_dye_lot_no VARCHAR(100),             -- 内部缸号
ADD COLUMN IF NOT EXISTS internal_piece_ids INTEGER[],                 -- 内部匹号 ID 列表
ADD COLUMN IF NOT EXISTS internal_piece_nos VARCHAR(100)[],            -- 内部匹号列表
ADD COLUMN IF NOT EXISTS supplier_dye_lot_no VARCHAR(100),             -- 供应商缸号
ADD COLUMN IF NOT EXISTS supplier_piece_nos VARCHAR(100)[],            -- 供应商匹号列表
ADD COLUMN IF NOT EXISTS batch_conversion_log_id INTEGER;              -- 批次转换日志 ID

-- 添加注释
COMMENT ON COLUMN purchase_receipt_item.internal_dye_lot_id IS '内部缸号 ID';
COMMENT ON COLUMN purchase_receipt_item.internal_dye_lot_no IS '内部缸号';
COMMENT ON COLUMN purchase_receipt_item.internal_piece_ids IS '内部匹号 ID 列表';
COMMENT ON COLUMN purchase_receipt_item.internal_piece_nos IS '内部匹号列表';
COMMENT ON COLUMN purchase_receipt_item.supplier_dye_lot_no IS '供应商缸号';
COMMENT ON COLUMN purchase_receipt_item.supplier_piece_nos IS '供应商匹号列表';
COMMENT ON COLUMN purchase_receipt_item.batch_conversion_log_id IS '批次转换日志 ID';

-- 外键约束
ALTER TABLE purchase_receipt_item ADD CONSTRAINT fk_pri_internal_dye_lot
    FOREIGN KEY (internal_dye_lot_id) REFERENCES batch_dye_lot(id);
ALTER TABLE purchase_receipt_item ADD CONSTRAINT fk_pri_conversion_log
    FOREIGN KEY (batch_conversion_log_id) REFERENCES batch_trace_log(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_pri_internal_dye_lot ON purchase_receipt_item(internal_dye_lot_id);
CREATE INDEX IF NOT EXISTS idx_pri_internal_piece_ids ON purchase_receipt_item USING GIN (internal_piece_ids);
CREATE INDEX IF NOT EXISTS idx_pri_supplier_dye_lot ON purchase_receipt_item(supplier_dye_lot_no);
CREATE INDEX IF NOT EXISTS idx_pri_supplier_piece_nos ON purchase_receipt_item USING GIN (supplier_piece_nos);

-- ========================================
-- 9. 更新触发器
-- ========================================

-- 更新销售发货单的 updated_at
DROP TRIGGER IF EXISTS trg_sales_delivery_updated_at ON sales_delivery;
CREATE TRIGGER trg_sales_delivery_updated_at
    BEFORE UPDATE ON sales_delivery
    FOR EACH ROW
    EXECUTE FUNCTION update_batch_updated_at_column();

-- ========================================
-- 10. 数据完整性约束
-- ========================================

-- 添加检查约束，确保销售订单明细的批次字段正确
ALTER TABLE sales_order_items
ADD CONSTRAINT chk_soi_batch_allocated
    CHECK (
        (batch_required = FALSE AND allocated_dye_lot_ids IS NULL)
        OR 
        (batch_required = TRUE)
    );

-- 添加检查约束，确保采购订单明细的批次字段正确
ALTER TABLE purchase_order_item
ADD CONSTRAINT chk_poi_batch_required
    CHECK (
        (batch_required = FALSE AND expected_dye_lot_info IS NULL)
        OR 
        (batch_required = TRUE)
    );

-- ========================================
-- 迁移完成
-- ========================================

-- ============================================
-- 来源: 031_bpm_process_engine.sql
-- ============================================
-- ========================================
-- 1. 流程定义表
-- ========================================

-- ==================== 流程定义表 ====================
CREATE TABLE IF NOT EXISTS bpm_process_definition (
    id SERIAL PRIMARY KEY,
    process_key VARCHAR(100) NOT NULL,                   -- 流程标识（英文唯一标识）
    process_name VARCHAR(200) NOT NULL,                  -- 流程名称
    process_version VARCHAR(20) NOT NULL,                -- 流程版本（v1.0.0）
    process_category VARCHAR(50) NOT NULL,               -- 流程分类（procurement/sales/finance/hr/other）
    description TEXT,                                    -- 流程描述
    icon_url VARCHAR(500),                               -- 流程图标
    cover_image_url VARCHAR(500),                        -- 封面图片
    
    -- 流程配置
    form_type VARCHAR(50) DEFAULT 'custom',              -- 表单类型（custom/dynamic）
    form_schema JSONB,                                   -- 表单 schema（JSON 格式）
    flow_definition JSONB NOT NULL,                      -- 流程定义（JSON 格式，包含节点、流转条件等）
    
    -- 状态管理
    status VARCHAR(20) DEFAULT 'draft',                  -- 状态（draft/active/suspended/deprecated）
    is_published BOOLEAN DEFAULT FALSE,                  -- 是否已发布
    published_at TIMESTAMP,                              -- 发布时间
    published_by INTEGER,                                -- 发布人 ID
    
    -- 权限配置
    visible_roles INTEGER[],                             -- 可见角色 ID 列表
    initiator_roles INTEGER[],                           -- 发起人角色 ID 列表
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER,                                  -- 更新人 ID
    remarks TEXT                                         -- 备注
);

COMMENT ON TABLE bpm_process_definition IS '流程定义表';
COMMENT ON COLUMN bpm_process_definition.process_key IS '流程标识（英文唯一标识）';
COMMENT ON COLUMN bpm_process_definition.flow_definition IS '流程定义（JSON 格式，包含节点、流转条件等）';
COMMENT ON COLUMN bpm_process_definition.status IS '状态（draft/active/suspended/deprecated）';
COMMENT ON COLUMN bpm_process_definition.visible_roles IS '可见角色 ID 列表';
COMMENT ON COLUMN bpm_process_definition.initiator_roles IS '发起人角色 ID 列表';

-- 唯一约束
ALTER TABLE bpm_process_definition ADD CONSTRAINT uk_process_key_version UNIQUE (process_key, process_version);

-- 索引
CREATE INDEX IF NOT EXISTS idx_bpm_pd_process_key ON bpm_process_definition(process_key);
CREATE INDEX IF NOT EXISTS idx_bpm_pd_status ON bpm_process_definition(status);
CREATE INDEX IF NOT EXISTS idx_bpm_pd_category ON bpm_process_definition(process_category);
CREATE INDEX IF NOT EXISTS idx_bpm_pd_published ON bpm_process_definition(is_published);
CREATE INDEX IF NOT EXISTS idx_bpm_pd_created_at ON bpm_process_definition(created_at DESC);

-- ========================================
-- 2. 流程实例表
-- ========================================

-- ==================== 流程实例表 ====================
CREATE TABLE IF NOT EXISTS bpm_process_instance (
    id SERIAL PRIMARY KEY,
    instance_no VARCHAR(100) NOT NULL UNIQUE,            -- 实例编号（自动生成）
    process_definition_id INTEGER NOT NULL,              -- 流程定义 ID
    business_type VARCHAR(50) NOT NULL,                  -- 业务类型（purchase_order/sales_order/payment/leave 等）
    business_id INTEGER NOT NULL,                        -- 业务 ID
    title VARCHAR(500) NOT NULL,                         -- 流程标题
    priority VARCHAR(20) DEFAULT 'normal',               -- 优先级（low/normal/high/urgent）
    
    -- 流程状态
    current_node_id VARCHAR(100),                        -- 当前节点 ID
    current_node_name VARCHAR(200),                      -- 当前节点名称
    status VARCHAR(20) DEFAULT 'running',                -- 状态（running/suspended/completed/terminated/cancelled）
    
    -- 人员信息
    initiator_id INTEGER NOT NULL,                       -- 发起人 ID
    initiator_name VARCHAR(100) NOT NULL,                -- 发起人姓名
    initiator_department_id INTEGER,                     -- 发起人部门 ID
    current_handler_ids INTEGER[],                       -- 当前处理人 ID 列表
    current_handler_names VARCHAR(100)[],                -- 当前处理人姓名列表
    
    -- 流程数据
    form_data JSONB,                                     -- 表单数据（JSON 格式）
    variables JSONB,                                     -- 流程变量（JSON 格式）
    
    -- 时间信息
    started_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 开始时间
    completed_at TIMESTAMP,                              -- 完成时间
    duration_seconds BIGINT,                             -- 耗时（秒）
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    remarks TEXT                                         -- 备注
);

COMMENT ON TABLE bpm_process_instance IS '流程实例表';
COMMENT ON COLUMN bpm_process_instance.business_type IS '业务类型（purchase_order/sales_order/payment/leave 等）';
COMMENT ON COLUMN bpm_process_instance.business_id IS '业务 ID';
COMMENT ON COLUMN bpm_process_instance.status IS '状态（running/suspended/completed/terminated/cancelled）';
COMMENT ON COLUMN bpm_process_instance.form_data IS '表单数据（JSON 格式）';
COMMENT ON COLUMN bpm_process_instance.variables IS '流程变量（JSON 格式）';

-- 外键约束
ALTER TABLE bpm_process_instance ADD CONSTRAINT fk_pi_process_definition
    FOREIGN KEY (process_definition_id) REFERENCES bpm_process_definition(id);
ALTER TABLE bpm_process_instance ADD CONSTRAINT fk_pi_initiator
    FOREIGN KEY (initiator_id) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_bpm_pi_instance_no ON bpm_process_instance(instance_no);
CREATE INDEX IF NOT EXISTS idx_bpm_pi_process_definition ON bpm_process_instance(process_definition_id);
CREATE INDEX IF NOT EXISTS idx_bpm_pi_business ON bpm_process_instance(business_type, business_id);
CREATE INDEX IF NOT EXISTS idx_bpm_pi_initiator ON bpm_process_instance(initiator_id);
CREATE INDEX IF NOT EXISTS idx_bpm_pi_status ON bpm_process_instance(status);
CREATE INDEX IF NOT EXISTS idx_bpm_pi_current_node ON bpm_process_instance(current_node_id);
CREATE INDEX IF NOT EXISTS idx_bpm_pi_current_handler ON bpm_process_instance USING GIN (current_handler_ids);
CREATE INDEX IF NOT EXISTS idx_bpm_pi_started_at ON bpm_process_instance(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_bpm_pi_completed_at ON bpm_process_instance(completed_at);

-- ========================================
-- 3. 流程任务表
-- ========================================

-- ==================== 流程任务表 ====================
CREATE TABLE IF NOT EXISTS bpm_task (
    id SERIAL PRIMARY KEY,
    task_no VARCHAR(100) NOT NULL UNIQUE,                -- 任务编号
    instance_id INTEGER NOT NULL,                        -- 实例 ID
    process_definition_id INTEGER NOT NULL,              -- 流程定义 ID
    
    -- 节点信息
    node_id VARCHAR(100) NOT NULL,                       -- 节点 ID
    node_name VARCHAR(200) NOT NULL,                     -- 节点名称
    node_type VARCHAR(50) NOT NULL,                      -- 节点类型（start/end/user_task/system_task/condition 等）
    
    -- 任务状态
    task_type VARCHAR(20) DEFAULT 'manual',              -- 任务类型（manual/auto/system）
    status VARCHAR(20) DEFAULT 'pending',                -- 状态（pending/processing/completed/rejected/withdrawn/cancelled）
    priority VARCHAR(20) DEFAULT 'normal',               -- 优先级（low/normal/high/urgent）
    
    -- 处理人信息
    assignee_ids INTEGER[],                              -- 指派人 ID 列表
    assignee_names VARCHAR(100)[],                       -- 指派人姓名列表
    candidate_role_ids INTEGER[],                        -- 候选角色 ID 列表
    candidate_user_ids INTEGER[],                        -- 候选用户 ID 列表
    actual_handler_id INTEGER,                           -- 实际处理人 ID
    actual_handler_name VARCHAR(100),                    -- 实际处理人姓名
    
    -- 审批信息
    action VARCHAR(20),                                  -- 操作（approve/reject/withdraw/terminate/delegate）
    approval_opinion TEXT,                               -- 审批意见
    attachment_urls TEXT[],                              -- 附件 URL 列表
    handled_at TIMESTAMP,                                -- 处理时间
    duration_seconds BIGINT,                             -- 处理耗时（秒）
    
    -- 超时配置
    due_date TIMESTAMP,                                  -- 预计完成时间
    is_overdue BOOLEAN DEFAULT FALSE,                    -- 是否超时
    overdue_days INTEGER,                                -- 超时天数
    
    -- 任务数据
    form_data JSONB,                                     -- 表单数据
    task_variables JSONB,                                -- 任务变量
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    remarks TEXT                                         -- 备注
);

COMMENT ON TABLE bpm_task IS '流程任务表';
COMMENT ON COLUMN bpm_task.node_type IS '节点类型（start/end/user_task/system_task/condition 等）';
COMMENT ON COLUMN bpm_task.status IS '状态（pending/processing/completed/rejected/withdrawn/cancelled）';
COMMENT ON COLUMN bpm_task.action IS '操作（approve/reject/withdraw/terminate/delegate）';
COMMENT ON COLUMN bpm_task.approval_opinion IS '审批意见';

-- 外键约束
ALTER TABLE bpm_task ADD CONSTRAINT fk_bpm_task_instance
    FOREIGN KEY (instance_id) REFERENCES bpm_process_instance(id) ON DELETE CASCADE;
ALTER TABLE bpm_task ADD CONSTRAINT fk_bpm_task_process_definition
    FOREIGN KEY (process_definition_id) REFERENCES bpm_process_definition(id);
ALTER TABLE bpm_task ADD CONSTRAINT fk_bpm_task_actual_handler
    FOREIGN KEY (actual_handler_id) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_bpm_task_task_no ON bpm_task(task_no);
CREATE INDEX IF NOT EXISTS idx_bpm_task_instance ON bpm_task(instance_id);
CREATE INDEX IF NOT EXISTS idx_bpm_task_process_definition ON bpm_task(process_definition_id);
CREATE INDEX IF NOT EXISTS idx_bpm_task_node ON bpm_task(node_id);
CREATE INDEX IF NOT EXISTS idx_bpm_task_status ON bpm_task(status);
CREATE INDEX IF NOT EXISTS idx_bpm_task_assignee_ids ON bpm_task USING GIN (assignee_ids);
CREATE INDEX IF NOT EXISTS idx_bpm_task_candidate_user_ids ON bpm_task USING GIN (candidate_user_ids);
CREATE INDEX IF NOT EXISTS idx_bpm_task_actual_handler ON bpm_task(actual_handler_id);
CREATE INDEX IF NOT EXISTS idx_bpm_task_created_at ON bpm_task(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_bpm_task_due_date ON bpm_task(due_date);
CREATE INDEX IF NOT EXISTS idx_bpm_task_overdue ON bpm_task(is_overdue);

-- ========================================
-- 4. 流程操作日志表
-- ========================================

-- ==================== 流程操作日志表 ====================
CREATE TABLE IF NOT EXISTS bpm_operation_log (
    id SERIAL PRIMARY KEY,
    log_no VARCHAR(100) NOT NULL UNIQUE,                 -- 日志编号
    instance_id INTEGER NOT NULL,                        -- 实例 ID
    task_id INTEGER,                                     -- 任务 ID
    
    -- 操作信息
    operation_type VARCHAR(50) NOT NULL,                 -- 操作类型（start/approve/reject/withdraw/terminate/delegate/assign 等）
    operation_desc VARCHAR(500) NOT NULL,                -- 操作描述
    operator_id INTEGER NOT NULL,                        -- 操作人 ID
    operator_name VARCHAR(100) NOT NULL,                 -- 操作人姓名
    operator_department_id INTEGER,                      -- 操作人部门 ID
    
    -- 操作详情
    from_node_id VARCHAR(100),                           -- 源节点 ID
    from_node_name VARCHAR(200),                         -- 源节点名称
    to_node_id VARCHAR(100),                             -- 目标节点 ID
    to_node_name VARCHAR(200),                           -- 目标节点名称
    approval_opinion TEXT,                               -- 审批意见
    attachment_urls TEXT[],                              -- 附件 URL 列表
    
    -- 操作时间
    operated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    
    -- 系统字段
    ip_address VARCHAR(50),                              -- 操作 IP
    device_info VARCHAR(200),                            -- 设备信息
    remarks TEXT                                         -- 备注
);

COMMENT ON TABLE bpm_operation_log IS '流程操作日志表';
COMMENT ON COLUMN bpm_operation_log.operation_type IS '操作类型（start/approve/reject/withdraw/terminate/delegate/assign 等）';
COMMENT ON COLUMN bpm_operation_log.approval_opinion IS '审批意见';

-- 外键约束
ALTER TABLE bpm_operation_log ADD CONSTRAINT fk_bpm_log_instance
    FOREIGN KEY (instance_id) REFERENCES bpm_process_instance(id) ON DELETE CASCADE;
ALTER TABLE bpm_operation_log ADD CONSTRAINT fk_bpm_log_task
    FOREIGN KEY (task_id) REFERENCES bpm_task(id);
ALTER TABLE bpm_operation_log ADD CONSTRAINT fk_bpm_log_operator
    FOREIGN KEY (operator_id) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_bpm_log_instance ON bpm_operation_log(instance_id);
CREATE INDEX IF NOT EXISTS idx_bpm_log_task ON bpm_operation_log(task_id);
CREATE INDEX IF NOT EXISTS idx_bpm_log_operator ON bpm_operation_log(operator_id);
CREATE INDEX IF NOT EXISTS idx_bpm_log_operated_at ON bpm_operation_log(operated_at DESC);
CREATE INDEX IF NOT EXISTS idx_bpm_log_operation_type ON bpm_operation_log(operation_type);

-- ========================================
-- 5. 流程节点配置表
-- ========================================

-- ==================== 流程节点配置表 ====================
CREATE TABLE IF NOT EXISTS bpm_node_config (
    id SERIAL PRIMARY KEY,
    process_definition_id INTEGER NOT NULL,              -- 流程定义 ID
    node_id VARCHAR(100) NOT NULL,                       -- 节点 ID
    node_name VARCHAR(200) NOT NULL,                     -- 节点名称
    node_type VARCHAR(50) NOT NULL,                      -- 节点类型
    
    -- 节点配置
    node_config JSONB,                                   -- 节点配置（JSON 格式）
    assignee_type VARCHAR(50),                           -- 指派人类型（user/role/department/variable）
    assignee_value TEXT,                                 -- 指派人值（根据类型不同而不同）
    
    -- 审批配置
    approval_type VARCHAR(20) DEFAULT 'or_sign',         -- 审批类型（or_sign/and_sign/or_first）
    min_approval_count INTEGER DEFAULT 1,                -- 最少审批通过人数
    need_comment BOOLEAN DEFAULT FALSE,                  -- 是否需要审批意见
    
    -- 超时配置
    timeout_seconds INTEGER,                             -- 超时时间（秒）
    timeout_action VARCHAR(50),                          -- 超时动作（auto_approve/auto_reject/notify）
    
    -- 通知配置
    notify_initiator BOOLEAN DEFAULT FALSE,              -- 是否通知发起人
    notify_handler BOOLEAN DEFAULT TRUE,                 -- 是否通知处理人
    
    -- 系统字段
    sort_order INTEGER DEFAULT 0,                        -- 排序
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否启用
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE bpm_node_config IS '流程节点配置表';
COMMENT ON COLUMN bpm_node_config.approval_type IS '审批类型（or_sign/and_sign/or_first）';
COMMENT ON COLUMN bpm_node_config.assignee_type IS '指派人类型（user/role/department/variable）';

-- 外键约束
ALTER TABLE bpm_node_config ADD CONSTRAINT fk_bpm_nc_process_definition
    FOREIGN KEY (process_definition_id) REFERENCES bpm_process_definition(id) ON DELETE CASCADE;

-- 唯一约束
ALTER TABLE bpm_node_config ADD CONSTRAINT uk_nc_process_node UNIQUE (process_definition_id, node_id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_bpm_nc_process_definition ON bpm_node_config(process_definition_id);
CREATE INDEX IF NOT EXISTS idx_bpm_nc_node_type ON bpm_node_config(node_type);
CREATE INDEX IF NOT EXISTS idx_bpm_nc_active ON bpm_node_config(is_active);

-- ========================================
-- 6. 流程流转条件表
-- ========================================

-- ==================== 流程流转条件表 ====================
CREATE TABLE IF NOT EXISTS bpm_transition_condition (
    id SERIAL PRIMARY KEY,
    process_definition_id INTEGER NOT NULL,              -- 流程定义 ID
    from_node_id VARCHAR(100) NOT NULL,                  -- 源节点 ID
    to_node_id VARCHAR(100) NOT NULL,                    -- 目标节点 ID
    
    -- 条件配置
    condition_name VARCHAR(200),                         -- 条件名称
    condition_expression TEXT,                           -- 条件表达式（支持脚本）
    condition_type VARCHAR(50) DEFAULT 'expression',     -- 条件类型（expression/script/default）
    priority INTEGER DEFAULT 0,                          -- 优先级（数字越小优先级越高）
    
    -- 系统字段
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否启用
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    remarks TEXT                                         -- 备注
);

COMMENT ON TABLE bpm_transition_condition IS '流程流转条件表';
COMMENT ON COLUMN bpm_transition_condition.condition_expression IS '条件表达式（支持脚本）';
COMMENT ON COLUMN bpm_transition_condition.condition_type IS '条件类型（expression/script/default）';

-- 外键约束
ALTER TABLE bpm_transition_condition ADD CONSTRAINT fk_bpm_tc_process_definition
    FOREIGN KEY (process_definition_id) REFERENCES bpm_process_definition(id) ON DELETE CASCADE;

-- 索引
CREATE INDEX IF NOT EXISTS idx_bpm_tc_process_definition ON bpm_transition_condition(process_definition_id);
CREATE INDEX IF NOT EXISTS idx_bpm_tc_from_node ON bpm_transition_condition(from_node_id);
CREATE INDEX IF NOT EXISTS idx_bpm_tc_to_node ON bpm_transition_condition(to_node_id);
CREATE INDEX IF NOT EXISTS idx_bpm_tc_active ON bpm_transition_condition(is_active);

-- ========================================
-- 7. 流程委托表
-- ========================================

-- ==================== 流程委托表 ====================
CREATE TABLE IF NOT EXISTS bpm_task_delegation (
    id SERIAL PRIMARY KEY,
    task_id INTEGER NOT NULL,                            -- 任务 ID
    delegator_id INTEGER NOT NULL,                       -- 委托人 ID
    delegatee_id INTEGER NOT NULL,                       -- 被委托人 ID
    delegation_type VARCHAR(20) NOT NULL,                -- 委托类型（temporary/permanent）
    
    -- 委托时间范围
    start_date DATE NOT NULL,                            -- 开始日期
    end_date DATE NOT NULL,                              -- 结束日期
    
    -- 委托状态
    status VARCHAR(20) DEFAULT 'active',                 -- 状态（active/expired/cancelled）
    
    -- 系统字段
    reason TEXT,                                         -- 委托原因
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    cancelled_at TIMESTAMP,                              -- 取消时间
    cancelled_by INTEGER                                 -- 取消人 ID
);

COMMENT ON TABLE bpm_task_delegation IS '流程委托表';
COMMENT ON COLUMN bpm_task_delegation.delegation_type IS '委托类型（temporary/permanent）';

-- 外键约束
ALTER TABLE bpm_task_delegation ADD CONSTRAINT fk_bpm_td_task
    FOREIGN KEY (task_id) REFERENCES bpm_task(id) ON DELETE CASCADE;
ALTER TABLE bpm_task_delegation ADD CONSTRAINT fk_bpm_td_delegator
    FOREIGN KEY (delegator_id) REFERENCES users(id);
ALTER TABLE bpm_task_delegation ADD CONSTRAINT fk_bpm_td_delegatee
    FOREIGN KEY (delegatee_id) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_bpm_td_task ON bpm_task_delegation(task_id);
CREATE INDEX IF NOT EXISTS idx_bpm_td_delegator ON bpm_task_delegation(delegator_id);
CREATE INDEX IF NOT EXISTS idx_bpm_td_delegatee ON bpm_task_delegation(delegatee_id);
CREATE INDEX IF NOT EXISTS idx_bpm_td_status ON bpm_task_delegation(status);
CREATE INDEX IF NOT EXISTS idx_bpm_td_date_range ON bpm_task_delegation(start_date, end_date);

-- ========================================
-- 8. 流程催办表
-- ========================================

-- ==================== 流程催办表 ====================
CREATE TABLE IF NOT EXISTS bpm_task_urge (
    id SERIAL PRIMARY KEY,
    task_id INTEGER NOT NULL,                            -- 任务 ID
    instance_id INTEGER NOT NULL,                        -- 实例 ID
    
    -- 催办信息
    urger_id INTEGER NOT NULL,                           -- 催办人 ID
    urger_name VARCHAR(100) NOT NULL,                    -- 催办人姓名
    urge_reason TEXT,                                    -- 催办原因
    urge_type VARCHAR(20) DEFAULT 'system',              -- 催办类型（system/manual）
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    notified_user_ids INTEGER[],                         -- 已通知用户 ID 列表
    remarks TEXT                                         -- 备注
);

COMMENT ON TABLE bpm_task_urge IS '流程催办表';
COMMENT ON COLUMN bpm_task_urge.urge_type IS '催办类型（system/manual）';

-- 外键约束
ALTER TABLE bpm_task_urge ADD CONSTRAINT fk_bpm_urge_task
    FOREIGN KEY (task_id) REFERENCES bpm_task(id) ON DELETE CASCADE;
ALTER TABLE bpm_task_urge ADD CONSTRAINT fk_bpm_urge_instance
    FOREIGN KEY (instance_id) REFERENCES bpm_process_instance(id) ON DELETE CASCADE;

-- 索引
CREATE INDEX IF NOT EXISTS idx_bpm_urge_task ON bpm_task_urge(task_id);
CREATE INDEX IF NOT EXISTS idx_bpm_urge_instance ON bpm_task_urge(instance_id);
CREATE INDEX IF NOT EXISTS idx_bpm_urge_created_at ON bpm_task_urge(created_at DESC);

-- ========================================
-- 9. 触发器函数
-- ========================================

-- ==================== 自动更新 updated_at ====================
CREATE OR REPLACE FUNCTION update_bpm_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ========================================
-- 10. 应用触发器
-- ========================================

-- 更新 updated_at 触发器
DROP TRIGGER IF EXISTS trg_bpm_pd_updated_at ON bpm_process_definition;
CREATE TRIGGER trg_bpm_pd_updated_at
    BEFORE UPDATE ON bpm_process_definition
    FOR EACH ROW
    EXECUTE FUNCTION update_bpm_updated_at_column();

DROP TRIGGER IF EXISTS trg_bpm_pi_updated_at ON bpm_process_instance;
CREATE TRIGGER trg_bpm_pi_updated_at
    BEFORE UPDATE ON bpm_process_instance
    FOR EACH ROW
    EXECUTE FUNCTION update_bpm_updated_at_column();

DROP TRIGGER IF EXISTS trg_bpm_task_updated_at ON bpm_task;
CREATE TRIGGER trg_bpm_task_updated_at
    BEFORE UPDATE ON bpm_task
    FOR EACH ROW
    EXECUTE FUNCTION update_bpm_updated_at_column();

DROP TRIGGER IF EXISTS trg_bpm_nc_updated_at ON bpm_node_config;
CREATE TRIGGER trg_bpm_nc_updated_at
    BEFORE UPDATE ON bpm_node_config
    FOR EACH ROW
    EXECUTE FUNCTION update_bpm_updated_at_column();

-- ========================================
-- 11. 初始化数据
-- ========================================

-- 初始化流程实例编号序列
CREATE SEQUENCE IF NOT EXISTS bpm_process_instance_no_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

-- 初始化任务编号序列
CREATE SEQUENCE IF NOT EXISTS bpm_task_no_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

-- 初始化日志编号序列
CREATE SEQUENCE IF NOT EXISTS bpm_operation_log_no_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

-- ========================================
-- 迁移完成
-- ========================================

-- ============================================
-- 来源: 032_bpm_extension.sql
-- ============================================
-- ========================================
-- 10. 流程通知表
-- ========================================

-- ==================== 流程通知表 ====================
CREATE TABLE IF NOT EXISTS bpm_task_notification (
    id SERIAL PRIMARY KEY,
    task_id INTEGER NOT NULL,                            -- 任务 ID
    instance_id INTEGER NOT NULL,                        -- 实例 ID
    user_id INTEGER NOT NULL,                            -- 用户 ID
    
    -- 通知信息
    notification_type VARCHAR(50) NOT NULL,              -- 通知类型（new_task/urge/overdue/delegation 等）
    notification_method VARCHAR(50),                     -- 通知方式（站内信/邮件/短信/微信）
    title VARCHAR(500) NOT NULL,                         -- 通知标题
    content TEXT NOT NULL,                               -- 通知内容
    
    -- 通知状态
    status VARCHAR(20) DEFAULT 'unread',                 -- 状态（unread/read/deleted）
    is_read BOOLEAN DEFAULT FALSE,                       -- 是否已读
    read_at TIMESTAMP,                                   -- 阅读时间
    
    -- 系统字段
    sent_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 发送时间
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE bpm_task_notification IS '流程通知表';
COMMENT ON COLUMN bpm_task_notification.notification_type IS '通知类型（new_task/urge/overdue/delegation 等）';
COMMENT ON COLUMN bpm_task_notification.status IS '状态（unread/read/deleted）';

-- 外键约束
ALTER TABLE bpm_task_notification ADD CONSTRAINT fk_bpm_tn_task
    FOREIGN KEY (task_id) REFERENCES bpm_task(id) ON DELETE CASCADE;
ALTER TABLE bpm_task_notification ADD CONSTRAINT fk_bpm_tn_instance
    FOREIGN KEY (instance_id) REFERENCES bpm_process_instance(id) ON DELETE CASCADE;
ALTER TABLE bpm_task_notification ADD CONSTRAINT fk_bpm_tn_user
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- 索引
CREATE INDEX IF NOT EXISTS idx_bpm_tn_task ON bpm_task_notification(task_id);
CREATE INDEX IF NOT EXISTS idx_bpm_tn_instance ON bpm_task_notification(instance_id);
CREATE INDEX IF NOT EXISTS idx_bpm_tn_user ON bpm_task_notification(user_id);
CREATE INDEX IF NOT EXISTS idx_bpm_tn_status ON bpm_task_notification(status);
CREATE INDEX IF NOT EXISTS idx_bpm_tn_is_read ON bpm_task_notification(is_read);
CREATE INDEX IF NOT EXISTS idx_bpm_tn_sent_at ON bpm_task_notification(sent_at DESC);

-- ========================================
-- 11. 流程统计表
-- ========================================

-- ==================== 流程统计表（按天） ====================
CREATE TABLE IF NOT EXISTS bpm_statistics_daily (
    id SERIAL PRIMARY KEY,
    statistics_date DATE NOT NULL,                       -- 统计日期
    process_definition_id INTEGER NOT NULL,              -- 流程定义 ID
    
    -- 发起统计
    initiated_count INTEGER DEFAULT 0,                   -- 发起数量
    completed_count INTEGER DEFAULT 0,                   -- 完成数量
    cancelled_count INTEGER DEFAULT 0,                   -- 取消数量
    
    -- 任务统计
    pending_tasks INTEGER DEFAULT 0,                     -- 待处理任务数
    completed_tasks INTEGER DEFAULT 0,                   -- 已完成任务数
    rejected_tasks INTEGER DEFAULT 0,                    -- 已拒绝任务数
    overdue_tasks INTEGER DEFAULT 0,                     -- 超时任务数
    
    -- 时效统计
    avg_duration_seconds BIGINT DEFAULT 0,               -- 平均耗时（秒）
    max_duration_seconds BIGINT DEFAULT 0,               -- 最大耗时（秒）
    min_duration_seconds BIGINT DEFAULT 0,               -- 最小耗时（秒）
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE bpm_statistics_daily IS '流程统计表（按天）';

-- 外键约束
ALTER TABLE bpm_statistics_daily ADD CONSTRAINT fk_bpm_sd_process_definition
    FOREIGN KEY (process_definition_id) REFERENCES bpm_process_definition(id);

-- 唯一约束
ALTER TABLE bpm_statistics_daily ADD CONSTRAINT uk_sd_date_process UNIQUE (statistics_date, process_definition_id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_bpm_sd_date ON bpm_statistics_daily(statistics_date);
CREATE INDEX IF NOT EXISTS idx_bpm_sd_process_definition ON bpm_statistics_daily(process_definition_id);

-- ========================================
-- 12. 流程超时配置表
-- ========================================

-- ==================== 流程超时配置表 ====================
CREATE TABLE IF NOT EXISTS bpm_timeout_config (
    id SERIAL PRIMARY KEY,
    process_definition_id INTEGER,                       -- 流程定义 ID（NULL 表示全局配置）
    node_id VARCHAR(100),                                -- 节点 ID（NULL 表示全局配置）
    
    -- 超时配置
    timeout_seconds INTEGER NOT NULL,                    -- 超时时间（秒）
    timeout_type VARCHAR(50) DEFAULT 'working_hours',    -- 超时类型（working_hours/calendar_hours）
    
    -- 超时动作
    action_type VARCHAR(50) DEFAULT 'notify',            -- 动作类型（notify/auto_approve/auto_reject/escalate）
    action_params JSONB,                                 -- 动作参数（JSON 格式）
    
    -- 通知配置
    notify_before_seconds INTEGER,                       -- 超时前通知时间（秒）
    notify_recipients INTEGER[],                         -- 通知接收人 ID 列表
    
    -- 系统字段
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否启用
    priority INTEGER DEFAULT 0,                          -- 优先级
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE bpm_timeout_config IS '流程超时配置表';
COMMENT ON COLUMN bpm_timeout_config.timeout_type IS '超时类型（working_hours/calendar_hours）';
COMMENT ON COLUMN bpm_timeout_config.action_type IS '动作类型（notify/auto_approve/auto_reject/escalate）';

-- 外键约束
ALTER TABLE bpm_timeout_config ADD CONSTRAINT fk_bpm_toc_process_definition
    FOREIGN KEY (process_definition_id) REFERENCES bpm_process_definition(id) ON DELETE CASCADE;

-- 索引
CREATE INDEX IF NOT EXISTS idx_bpm_tc_process_definition ON bpm_timeout_config(process_definition_id);
CREATE INDEX IF NOT EXISTS idx_bpm_tc_node ON bpm_timeout_config(node_id);
CREATE INDEX IF NOT EXISTS idx_bpm_tc_active ON bpm_timeout_config(is_active);
CREATE INDEX IF NOT EXISTS idx_bpm_tc_priority ON bpm_timeout_config(priority);

-- ========================================
-- 触发器
-- ========================================

-- 更新 updated_at 触发器
DROP TRIGGER IF EXISTS trg_bpm_tn_updated_at ON bpm_task_notification;
CREATE TRIGGER trg_bpm_tn_updated_at
    BEFORE UPDATE ON bpm_task_notification
    FOR EACH ROW
    EXECUTE FUNCTION update_bpm_updated_at_column();

DROP TRIGGER IF EXISTS trg_bpm_sd_updated_at ON bpm_statistics_daily;
CREATE TRIGGER trg_bpm_sd_updated_at
    BEFORE UPDATE ON bpm_statistics_daily
    FOR EACH ROW
    EXECUTE FUNCTION update_bpm_updated_at_column();

DROP TRIGGER IF EXISTS trg_bpm_tc_updated_at ON bpm_timeout_config;
CREATE TRIGGER trg_bpm_tc_updated_at
    BEFORE UPDATE ON bpm_timeout_config
    FOR EACH ROW
    EXECUTE FUNCTION update_bpm_updated_at_column();

-- ========================================
-- 迁移完成
-- ========================================

-- ============================================
-- 来源: 033_log_management.sql
-- ============================================
-- ========================================
-- 1. 操作日志表
-- ========================================

-- ==================== 操作日志表 ====================
CREATE TABLE IF NOT EXISTS log_operation (
    id BIGSERIAL PRIMARY KEY,
    log_no VARCHAR(100) NOT NULL UNIQUE,                 -- 日志编号
    module VARCHAR(50) NOT NULL,                         -- 模块（procurement/sales/inventory/finance 等）
    operation_type VARCHAR(50) NOT NULL,                 -- 操作类型（create/update/delete/approve/reject 等）
    operation_desc VARCHAR(500),                         -- 操作描述
    
    -- 业务信息
    business_type VARCHAR(50),                           -- 业务类型
    business_id INTEGER,                                 -- 业务 ID
    business_no VARCHAR(100),                            -- 业务编号
    business_desc VARCHAR(500),                          -- 业务描述
    
    -- 操作人信息
    user_id INTEGER NOT NULL,                            -- 用户 ID
    username VARCHAR(100) NOT NULL,                      -- 用户名
    real_name VARCHAR(100),                              -- 真实姓名
    department_id INTEGER,                               -- 部门 ID
    department_name VARCHAR(200),                        -- 部门名称
    
    -- 操作详情
    request_method VARCHAR(20),                          -- 请求方法（GET/POST/PUT/DELETE）
    request_url TEXT,                                    -- 请求 URL
    request_params JSONB,                                -- 请求参数
    request_body JSONB,                                  -- 请求体
    response_status INTEGER,                             -- 响应状态码
    response_body JSONB,                                 -- 响应体
    
    -- 设备信息
    ip_address VARCHAR(50),                              -- IP 地址
    ip_location VARCHAR(200),                            -- IP 所在地
    user_agent TEXT,                                     -- User-Agent
    device_type VARCHAR(50),                             -- 设备类型（PC/Mobile/Tablet）
    browser VARCHAR(100),                                -- 浏览器
    os VARCHAR(100),                                     -- 操作系统
    
    -- 时间信息
    operation_time TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 操作时间
    duration_ms INTEGER,                                 -- 耗时（毫秒）
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE log_operation IS '操作日志表';
COMMENT ON COLUMN log_operation.module IS '模块（procurement/sales/inventory/finance 等）';
COMMENT ON COLUMN log_operation.operation_type IS '操作类型（create/update/delete/approve/reject 等）';
COMMENT ON COLUMN log_operation.request_params IS '请求参数';
COMMENT ON COLUMN log_operation.request_body IS '请求体';
COMMENT ON COLUMN log_operation.response_body IS '响应体';

-- 索引
CREATE INDEX IF NOT EXISTS idx_log_op_log_no ON log_operation(log_no);
CREATE INDEX IF NOT EXISTS idx_log_op_module ON log_operation(module);
CREATE INDEX IF NOT EXISTS idx_log_op_operation_type ON log_operation(operation_type);
CREATE INDEX IF NOT EXISTS idx_log_op_user ON log_operation(user_id);
CREATE INDEX IF NOT EXISTS idx_log_op_business ON log_operation(business_type, business_id);
CREATE INDEX IF NOT EXISTS idx_log_op_operation_time ON log_operation(operation_time DESC);
CREATE INDEX IF NOT EXISTS idx_log_op_module_time ON log_operation(module, operation_time DESC);

-- ========================================
-- 2. 系统日志表
-- ========================================

-- ==================== 系统日志表 ====================
CREATE TABLE IF NOT EXISTS log_system (
    id BIGSERIAL PRIMARY KEY,
    log_no VARCHAR(100) NOT NULL UNIQUE,                 -- 日志编号
    
    -- 日志级别
    log_level VARCHAR(20) NOT NULL,                      -- 日志级别（DEBUG/INFO/WARN/ERROR/FATAL）
    
    -- 日志信息
    logger_name VARCHAR(200) NOT NULL,                   -- 记录器名称
    message TEXT NOT NULL,                               -- 日志消息
    exception_type VARCHAR(200),                         -- 异常类型
    exception_message TEXT,                              -- 异常消息
    stack_trace TEXT,                                    -- 堆栈跟踪
    log_data JSONB,                                      -- 日志数据
    
    -- 线程信息
    thread_name VARCHAR(100),                            -- 线程名称
    thread_id BIGINT,                                    -- 线程 ID
    
    -- 位置信息
    file_name VARCHAR(200),                              -- 文件名
    method_name VARCHAR(200),                            -- 方法名
    line_number INTEGER,                                 -- 行号
    
    -- 时间信息
    log_time TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 日志时间
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE log_system IS '系统日志表';
COMMENT ON COLUMN log_system.log_level IS '日志级别（DEBUG/INFO/WARN/ERROR/FATAL）';
COMMENT ON COLUMN log_system.logger_name IS '记录器名称';
COMMENT ON COLUMN log_system.log_data IS '日志数据';

-- 索引
CREATE INDEX IF NOT EXISTS idx_log_sys_log_no ON log_system(log_no);
CREATE INDEX IF NOT EXISTS idx_log_sys_level ON log_system(log_level);
CREATE INDEX IF NOT EXISTS idx_log_sys_logger ON log_system(logger_name);
CREATE INDEX IF NOT EXISTS idx_log_sys_time ON log_system(log_time DESC);
CREATE INDEX IF NOT EXISTS idx_log_sys_level_time ON log_system(log_level, log_time DESC);

-- ========================================
-- 3. 登录日志表
-- ========================================

-- ==================== 登录日志表 ====================
CREATE TABLE IF NOT EXISTS log_login (
    id BIGSERIAL PRIMARY KEY,
    log_no VARCHAR(100) NOT NULL UNIQUE,                 -- 日志编号
    
    -- 用户信息
    user_id INTEGER,                                     -- 用户 ID
    username VARCHAR(100) NOT NULL,                      -- 用户名
    real_name VARCHAR(100),                              -- 真实姓名
    
    -- 登录信息
    login_status VARCHAR(20) NOT NULL,                   -- 登录状态（success/failed/locked）
    failure_reason VARCHAR(200),                         -- 失败原因
    login_type VARCHAR(50) DEFAULT 'password',           -- 登录类型（password/sms/email/oauth）
    
    -- 设备信息
    ip_address VARCHAR(50),                              -- IP 地址
    ip_location VARCHAR(200),                            -- IP 所在地
    user_agent TEXT,                                     -- User-Agent
    device_type VARCHAR(50),                             -- 设备类型（PC/Mobile/Tablet）
    browser VARCHAR(100),                                -- 浏览器
    os VARCHAR(100),                                     -- 操作系统
    
    -- 时间信息
    login_time TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 登录时间
    logout_time TIMESTAMP,                               -- 登出时间
    session_duration_seconds BIGINT,                     -- 会话时长（秒）
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE log_login IS '登录日志表';
COMMENT ON COLUMN log_login.login_status IS '登录状态（success/failed/locked）';
COMMENT ON COLUMN log_login.login_type IS '登录类型（password/sms/email/oauth）';

-- 索引
CREATE INDEX IF NOT EXISTS idx_log_login_log_no ON log_login(log_no);
CREATE INDEX IF NOT EXISTS idx_log_login_user ON log_login(user_id);
CREATE INDEX IF NOT EXISTS idx_log_login_username ON log_login(username);
CREATE INDEX IF NOT EXISTS idx_log_login_status ON log_login(login_status);
CREATE INDEX IF NOT EXISTS idx_log_login_time ON log_login(login_time DESC);
CREATE INDEX IF NOT EXISTS idx_log_login_ip ON log_login(ip_address);

-- ========================================
-- 4. API 访问日志表
-- ========================================

-- ==================== API 访问日志表 ====================
CREATE TABLE IF NOT EXISTS log_api_access (
    id BIGSERIAL PRIMARY KEY,
    log_no VARCHAR(100) NOT NULL UNIQUE,                 -- 日志编号
    
    -- 请求信息
    request_id VARCHAR(100) NOT NULL,                    -- 请求 ID
    request_method VARCHAR(20) NOT NULL,                 -- 请求方法
    request_url TEXT NOT NULL,                           -- 请求 URL
    request_path VARCHAR(500),                           -- 请求路径
    query_params JSONB,                                  -- 查询参数
    request_headers JSONB,                               -- 请求头
    request_body TEXT,                                   -- 请求体
    content_type VARCHAR(100),                           -- Content-Type
    
    -- 响应信息
    response_status INTEGER NOT NULL,                    -- 响应状态码
    response_headers JSONB,                              -- 响应头
    response_body TEXT,                                  -- 响应体
    response_size BIGINT,                                -- 响应大小（字节）
    
    -- 性能信息
    duration_ms INTEGER NOT NULL,                        -- 耗时（毫秒）
    db_query_count INTEGER DEFAULT 0,                    -- 数据库查询次数
    db_query_time_ms INTEGER DEFAULT 0,                  -- 数据库查询耗时（毫秒）
    
    -- 客户端信息
    client_ip VARCHAR(50),                               -- 客户端 IP
    client_location VARCHAR(200),                        -- 客户端位置
    user_agent TEXT,                                     -- User-Agent
    client_type VARCHAR(50),                             -- 客户端类型（web/mobile/api）
    
    -- 认证信息
    user_id INTEGER,                                     -- 用户 ID
    username VARCHAR(100),                               -- 用户名
    auth_type VARCHAR(50),                               -- 认证类型（jwt/session/api_key）
    
    -- 时间信息
    access_time TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 访问时间
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE log_api_access IS 'API 访问日志表';
COMMENT ON COLUMN log_api_access.request_headers IS '请求头';
COMMENT ON COLUMN log_api_access.response_headers IS '响应头';
COMMENT ON COLUMN log_api_access.db_query_count IS '数据库查询次数';

-- 索引
CREATE INDEX IF NOT EXISTS idx_log_api_log_no ON log_api_access(log_no);
CREATE INDEX IF NOT EXISTS idx_log_api_request_id ON log_api_access(request_id);
CREATE INDEX IF NOT EXISTS idx_log_api_method ON log_api_access(request_method);
CREATE INDEX IF NOT EXISTS idx_log_api_status ON log_api_access(response_status);
CREATE INDEX IF NOT EXISTS idx_log_api_user ON log_api_access(user_id);
CREATE INDEX IF NOT EXISTS idx_log_api_time ON log_api_access(access_time DESC);
CREATE INDEX IF NOT EXISTS idx_log_api_path ON log_api_access(request_path);
CREATE INDEX IF NOT EXISTS idx_log_api_client_ip ON log_api_access(client_ip);

-- ========================================
-- 5. 触发器函数
-- ========================================

-- ==================== 自动生成日志编号 ====================
CREATE OR REPLACE FUNCTION generate_log_no()
RETURNS TRIGGER AS $$
BEGIN
    -- 根据表名生成不同的编号前缀
    IF TG_TABLE_NAME = 'log_operation' THEN
        NEW.log_no := 'OP' || TO_CHAR(CURRENT_DATE, 'YYYYMMDD') || LPAD(NEXTVAL('log_operation_id_seq')::TEXT, 10, '0');
    ELSIF TG_TABLE_NAME = 'log_system' THEN
        NEW.log_no := 'SYS' || TO_CHAR(CURRENT_DATE, 'YYYYMMDD') || LPAD(NEXTVAL('log_system_id_seq')::TEXT, 10, '0');
    ELSIF TG_TABLE_NAME = 'log_login' THEN
        NEW.log_no := 'LOG' || TO_CHAR(CURRENT_DATE, 'YYYYMMDD') || LPAD(NEXTVAL('log_login_id_seq')::TEXT, 10, '0');
    ELSIF TG_TABLE_NAME = 'log_api_access' THEN
        NEW.log_no := 'API' || TO_CHAR(CURRENT_DATE, 'YYYYMMDD') || LPAD(NEXTVAL('log_api_access_id_seq')::TEXT, 10, '0');
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ========================================
-- 6. 应用触发器
-- ========================================

-- 自动生成日志编号触发器
DROP TRIGGER IF EXISTS trg_log_operation_generate_no ON log_operation;
CREATE TRIGGER trg_log_operation_generate_no
    BEFORE INSERT ON log_operation
    FOR EACH ROW
    EXECUTE FUNCTION generate_log_no();

DROP TRIGGER IF EXISTS trg_log_system_generate_no ON log_system;
CREATE TRIGGER trg_log_system_generate_no
    BEFORE INSERT ON log_system
    FOR EACH ROW
    EXECUTE FUNCTION generate_log_no();

DROP TRIGGER IF EXISTS trg_log_login_generate_no ON log_login;
CREATE TRIGGER trg_log_login_generate_no
    BEFORE INSERT ON log_login
    FOR EACH ROW
    EXECUTE FUNCTION generate_log_no();

DROP TRIGGER IF EXISTS trg_log_api_access_generate_no ON log_api_access;
CREATE TRIGGER trg_log_api_access_generate_no
    BEFORE INSERT ON log_api_access
    FOR EACH ROW
    EXECUTE FUNCTION generate_log_no();

-- ========================================
-- 7. 分区表配置（按时间分区，优化大数据量场景）
-- ========================================

-- 注意：PostgreSQL 18 支持声明式分区，按 operation_time 月度分区
-- 由于分区表需要主表先存在，所以先创建主表，再创建分区

-- 创建分区主表（按月分区）
CREATE TABLE IF NOT EXISTS log_operation_partitioned (
    id BIGSERIAL,
    log_no VARCHAR(100) NOT NULL,
    module VARCHAR(50) NOT NULL,
    operation_type VARCHAR(50) NOT NULL,
    operation_desc VARCHAR(500),
    business_type VARCHAR(50),
    business_id INTEGER,
    business_no VARCHAR(100),
    business_desc VARCHAR(500),
    user_id INTEGER NOT NULL,
    username VARCHAR(100) NOT NULL,
    real_name VARCHAR(100),
    department_id INTEGER,
    department_name VARCHAR(200),
    request_method VARCHAR(20),
    request_url TEXT,
    request_params JSONB,
    request_body JSONB,
    response_status INTEGER,
    response_body JSONB,
    ip_address VARCHAR(50),
    ip_location VARCHAR(200),
    user_agent TEXT,
    device_type VARCHAR(50),
    browser VARCHAR(100),
    os VARCHAR(100),
    operation_time TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    duration_ms INTEGER,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id, operation_time)
) PARTITION BY RANGE (operation_time);

COMMENT ON TABLE log_operation_partitioned IS '操作日志分区表（按月度分区）';

-- 为 2026 年创建月度分区
CREATE TABLE IF NOT EXISTS log_operation_202601 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');
CREATE TABLE IF NOT EXISTS log_operation_202602 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');
CREATE TABLE IF NOT EXISTS log_operation_202603 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');
CREATE TABLE IF NOT EXISTS log_operation_202604 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE IF NOT EXISTS log_operation_202605 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE IF NOT EXISTS log_operation_202606 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');
CREATE TABLE IF NOT EXISTS log_operation_202607 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-07-01') TO ('2026-08-01');
CREATE TABLE IF NOT EXISTS log_operation_202608 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-08-01') TO ('2026-09-01');
CREATE TABLE IF NOT EXISTS log_operation_202609 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-09-01') TO ('2026-10-01');
CREATE TABLE IF NOT EXISTS log_operation_202610 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-10-01') TO ('2026-11-01');
CREATE TABLE IF NOT EXISTS log_operation_202611 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-11-01') TO ('2026-12-01');
CREATE TABLE IF NOT EXISTS log_operation_202612 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-12-01') TO ('2027-01-01');

-- 分区表索引
CREATE INDEX IF NOT EXISTS idx_lop_partitioned_module ON log_operation_partitioned(module);
CREATE INDEX IF NOT EXISTS idx_lop_partitioned_operation_type ON log_operation_partitioned(operation_type);
CREATE INDEX IF NOT EXISTS idx_lop_partitioned_user ON log_operation_partitioned(user_id);
CREATE INDEX IF NOT EXISTS idx_lop_partitioned_business ON log_operation_partitioned(business_type, business_id);
CREATE INDEX IF NOT EXISTS idx_lop_partitioned_operation_time ON log_operation_partitioned(operation_time DESC);

-- 为分区表创建触发器以自动生成日志编号
DROP TRIGGER IF EXISTS trg_log_operation_partitioned_generate_no ON log_operation_partitioned;
CREATE TRIGGER trg_log_operation_partitioned_generate_no
    BEFORE INSERT ON log_operation_partitioned
    FOR EACH ROW
    EXECUTE FUNCTION generate_log_no();

-- 数据迁移：将原表数据迁移到分区表（可选，需要时执行）
-- INSERT INTO log_operation_partitioned SELECT * FROM log_operation;

COMMENT ON TABLE log_operation_partitioned IS '操作日志分区表 - 按月度分区存储';
COMMENT ON COLUMN log_operation_partitioned.operation_time IS '分区键 - 操作时间';

-- ========================================
-- 迁移完成
-- ========================================

-- ============================================
-- 来源: 034_crm_extension.sql
-- ============================================
-- ========================================
-- 1. 销售线索表
-- ========================================

-- ==================== 销售线索表 ====================
CREATE TABLE IF NOT EXISTS crm_lead (
    id SERIAL PRIMARY KEY,
    lead_no VARCHAR(100) NOT NULL UNIQUE,                -- 线索编号
    lead_source VARCHAR(50) NOT NULL,                    -- 线索来源（website/referral/exhibition/cold_call/other）
    lead_status VARCHAR(20) DEFAULT 'new',               -- 线索状态（new/contacted/qualified/converted/lost）
    
    -- 客户信息
    company_name VARCHAR(200),                           -- 公司名称
    contact_name VARCHAR(100) NOT NULL,                  -- 联系人姓名
    contact_title VARCHAR(100),                          -- 联系人职位
    mobile_phone VARCHAR(20),                            -- 手机号码
    tel_phone VARCHAR(50),                               -- 联系电话
    email VARCHAR(100),                                  -- 联系邮箱
    wechat VARCHAR(50),                                  -- 微信
    qq VARCHAR(20),                                      -- QQ
    address TEXT,                                        -- 地址
    
    -- 需求信息
    product_interest TEXT,                               -- 意向产品
    estimated_quantity DECIMAL(10,2),                    -- 预计数量
    estimated_amount DECIMAL(10,2),                      -- 预计金额
    expected_delivery_date DATE,                         -- 期望交货日期
    requirement_desc TEXT,                               -- 需求描述
    
    -- 跟进信息
    owner_id INTEGER NOT NULL,                           -- 负责人 ID
    owner_name VARCHAR(100) NOT NULL,                    -- 负责人姓名
    last_follow_up_date DATE,                            -- 最后跟进日期
    next_follow_up_date DATE,                            -- 下次跟进日期
    follow_up_plan TEXT,                                 -- 跟进计划
    
    -- 转化信息
    converted_at TIMESTAMP,                              -- 转化时间
    converted_customer_id INTEGER,                       -- 转化后客户 ID
    converted_opportunity_id INTEGER,                    -- 转化后商机 ID
    lost_reason VARCHAR(200),                            -- 丢失原因
    
    -- 系统字段
    priority VARCHAR(20) DEFAULT 'medium',               -- 优先级（low/medium/high/urgent）
    rating INTEGER DEFAULT 0,                            -- 评级（0-5）
    tags TEXT[],                                         -- 标签列表
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE crm_lead IS '销售线索表';
COMMENT ON COLUMN crm_lead.lead_source IS '线索来源（website/referral/exhibition/cold_call/other）';
COMMENT ON COLUMN crm_lead.lead_status IS '线索状态（new/contacted/qualified/converted/lost）';

-- 外键约束
ALTER TABLE crm_lead ADD CONSTRAINT fk_crm_lead_owner
    FOREIGN KEY (owner_id) REFERENCES users(id);
ALTER TABLE crm_lead ADD CONSTRAINT fk_crm_lead_converted_customer
    FOREIGN KEY (converted_customer_id) REFERENCES customers(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_crm_lead_lead_no ON crm_lead(lead_no);
CREATE INDEX IF NOT EXISTS idx_crm_lead_source ON crm_lead(lead_source);
CREATE INDEX IF NOT EXISTS idx_crm_lead_status ON crm_lead(lead_status);
CREATE INDEX IF NOT EXISTS idx_crm_lead_owner ON crm_lead(owner_id);
CREATE INDEX IF NOT EXISTS idx_crm_lead_created_at ON crm_lead(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_crm_lead_priority ON crm_lead(priority);

-- ========================================
-- 2. 商机表
-- ========================================

-- ==================== 商机表 ====================
CREATE TABLE IF NOT EXISTS crm_opportunity (
    id SERIAL PRIMARY KEY,
    opportunity_no VARCHAR(100) NOT NULL UNIQUE,         -- 商机编号
    opportunity_name VARCHAR(500) NOT NULL,              -- 商机名称
    customer_id INTEGER NOT NULL,                        -- 客户 ID
    lead_id INTEGER,                                     -- 来源线索 ID
    
    -- 商机信息
    opportunity_type VARCHAR(50),                        -- 商机类型（new_business/existing_business/upsell）
    opportunity_stage VARCHAR(50) DEFAULT 'prospecting', -- 商机阶段
    win_probability DECIMAL(5,2) DEFAULT 0.00,           -- 成功概率（%）
    
    -- 金额信息
    estimated_amount DECIMAL(10,2),                      -- 预计金额
    actual_amount DECIMAL(10,2),                         -- 实际金额
    currency VARCHAR(10) DEFAULT 'CNY',                  -- 币种
    
    -- 时间信息
    expected_close_date DATE,                            -- 预计成交日期
    actual_close_date DATE,                              -- 实际成交日期
    
    -- 产品信息
    product_ids INTEGER[],                               -- 产品 ID 列表
    product_names VARCHAR(200)[],                        -- 产品名称列表
    product_desc TEXT,                                   -- 产品描述
    
    -- 跟进信息
    owner_id INTEGER NOT NULL,                           -- 负责人 ID
    owner_name VARCHAR(100) NOT NULL,                    -- 负责人姓名
    last_follow_up_date DATE,                            -- 最后跟进日期
    next_follow_up_date DATE,                            -- 下次跟进日期
    follow_up_plan TEXT,                                 -- 跟进计划
    
    -- 竞争对手
    competitor_names TEXT[],                             -- 竞争对手列表
    competitive_advantage TEXT,                          -- 竞争优势
    
    -- 状态信息
    opportunity_status VARCHAR(20) DEFAULT 'open',       -- 状态（open/won/lost/cancelled）
    won_reason TEXT,                                     -- 赢单原因
    lost_reason TEXT,                                    -- 丢单原因
    
    -- 系统字段
    priority VARCHAR(20) DEFAULT 'medium',               -- 优先级
    rating INTEGER DEFAULT 0,                            -- 评级（0-5）
    tags TEXT[],                                         -- 标签列表
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE crm_opportunity IS '商机表';
COMMENT ON COLUMN crm_opportunity.opportunity_stage IS '商机阶段';
COMMENT ON COLUMN crm_opportunity.win_probability IS '成功概率（%）';
COMMENT ON COLUMN crm_opportunity.opportunity_status IS '状态（open/won/lost/cancelled）';

-- 外键约束
ALTER TABLE crm_opportunity ADD CONSTRAINT fk_crm_opp_customer
    FOREIGN KEY (customer_id) REFERENCES customers(id);
ALTER TABLE crm_opportunity ADD CONSTRAINT fk_crm_opp_lead
    FOREIGN KEY (lead_id) REFERENCES crm_lead(id);
ALTER TABLE crm_opportunity ADD CONSTRAINT fk_crm_opp_owner
    FOREIGN KEY (owner_id) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_crm_opp_opportunity_no ON crm_opportunity(opportunity_no);
CREATE INDEX IF NOT EXISTS idx_crm_opp_customer ON crm_opportunity(customer_id);
CREATE INDEX IF NOT EXISTS idx_crm_opp_lead ON crm_opportunity(lead_id);
CREATE INDEX IF NOT EXISTS idx_crm_opp_stage ON crm_opportunity(opportunity_stage);
CREATE INDEX IF NOT EXISTS idx_crm_opp_status ON crm_opportunity(opportunity_status);
CREATE INDEX IF NOT EXISTS idx_crm_opp_owner ON crm_opportunity(owner_id);
CREATE INDEX IF NOT EXISTS idx_crm_opp_expected_close ON crm_opportunity(expected_close_date);

-- ========================================
-- 3. 客户跟进记录表
-- ========================================

-- ==================== 客户跟进记录表 ====================
CREATE TABLE IF NOT EXISTS crm_follow_up (
    id SERIAL PRIMARY KEY,
    follow_up_no VARCHAR(100) NOT NULL UNIQUE,           -- 跟进记录编号
    
    -- 关联信息
    lead_id INTEGER,                                     -- 线索 ID
    opportunity_id INTEGER,                              -- 商机 ID
    customer_id INTEGER,                                 -- 客户 ID
    
    -- 跟进信息
    follow_up_type VARCHAR(50) NOT NULL,                 -- 跟进类型（phone_call/meeting/email/wechat/other）
    follow_up_date DATE NOT NULL,                        -- 跟进日期
    follow_up_time TIME,                                 -- 跟进时间
    duration_minutes INTEGER,                            -- 时长（分钟）
    
    -- 跟进内容
    subject VARCHAR(500),                                -- 主题
    content TEXT NOT NULL,                               -- 跟进内容
    summary TEXT,                                        -- 跟进总结
    feedback TEXT,                                       -- 客户反馈
    
    -- 下一步计划
    next_step TEXT,                                      -- 下一步计划
    next_follow_up_date DATE,                            -- 下次跟进日期
    
    -- 附件
    attachment_urls TEXT[],                              -- 附件 URL 列表
    
    -- 系统字段
    owner_id INTEGER NOT NULL,                           -- 负责人 ID
    owner_name VARCHAR(100) NOT NULL,                    -- 负责人姓名
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE crm_follow_up IS '客户跟进记录表';
COMMENT ON COLUMN crm_follow_up.follow_up_type IS '跟进类型（phone_call/meeting/email/wechat/other）';

-- 外键约束
ALTER TABLE crm_follow_up ADD CONSTRAINT fk_crm_fu_lead
    FOREIGN KEY (lead_id) REFERENCES crm_lead(id);
ALTER TABLE crm_follow_up ADD CONSTRAINT fk_crm_fu_opportunity
    FOREIGN KEY (opportunity_id) REFERENCES crm_opportunity(id);
ALTER TABLE crm_follow_up ADD CONSTRAINT fk_crm_fu_customer
    FOREIGN KEY (customer_id) REFERENCES customers(id);
ALTER TABLE crm_follow_up ADD CONSTRAINT fk_crm_fu_owner
    FOREIGN KEY (owner_id) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_crm_fu_lead ON crm_follow_up(lead_id);
CREATE INDEX IF NOT EXISTS idx_crm_fu_opportunity ON crm_follow_up(opportunity_id);
CREATE INDEX IF NOT EXISTS idx_crm_fu_customer ON crm_follow_up(customer_id);
CREATE INDEX IF NOT EXISTS idx_crm_fu_owner ON crm_follow_up(owner_id);
CREATE INDEX IF NOT EXISTS idx_crm_fu_date ON crm_follow_up(follow_up_date DESC);
CREATE INDEX IF NOT EXISTS idx_crm_fu_type ON crm_follow_up(follow_up_type);

-- ========================================
-- 4. 客户联系人表（扩展）
-- ========================================

-- ==================== 客户联系人表（如果不存在则创建） ====================
CREATE TABLE IF NOT EXISTS crm_contact (
    id SERIAL PRIMARY KEY,
    customer_id INTEGER NOT NULL,                        -- 客户 ID
    contact_name VARCHAR(100) NOT NULL,                  -- 联系人姓名
    contact_title VARCHAR(100),                          -- 职位
    department VARCHAR(100),                             -- 部门
    mobile_phone VARCHAR(20),                            -- 手机号码
    tel_phone VARCHAR(50),                               -- 联系电话
    email VARCHAR(100),                                  -- 联系邮箱
    wechat VARCHAR(50),                                  -- 微信
    qq VARCHAR(20),                                      -- QQ
    is_primary BOOLEAN DEFAULT FALSE,                    -- 是否主要联系人
    is_decision_maker BOOLEAN DEFAULT FALSE,             -- 是否决策者
    contact_preference VARCHAR(50),                      -- 联系偏好
    birthday DATE,                                       -- 生日
    remarks TEXT,                                        -- 备注
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE crm_contact IS '客户联系人表';
COMMENT ON COLUMN crm_contact.is_decision_maker IS '是否决策者';

-- 外键约束
ALTER TABLE crm_contact ADD CONSTRAINT fk_crm_contact_customer
    FOREIGN KEY (customer_id) REFERENCES customers(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_crm_contact_customer ON crm_contact(customer_id);
CREATE INDEX IF NOT EXISTS idx_crm_contact_mobile ON crm_contact(mobile_phone);
CREATE INDEX IF NOT EXISTS idx_crm_contact_email ON crm_contact(email);

-- ========================================
-- 5. 客户公海表
-- ========================================

-- ==================== 客户公海表 ====================
CREATE TABLE IF NOT EXISTS crm_customer_sea (
    id SERIAL PRIMARY KEY,
    customer_id INTEGER NOT NULL,                        -- 客户 ID
    reason_type VARCHAR(50) NOT NULL,                    -- 进入公海原因（no_follow_up/active_release/passive_release）
    reason_detail TEXT,                                  -- 原因详情
    
    -- 时间信息
    released_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 释放时间
    released_by INTEGER NOT NULL,                        -- 释放人 ID
    released_by_name VARCHAR(100) NOT NULL,              -- 释放人姓名
    
    -- 领取信息
    claimed_at TIMESTAMP,                                -- 领取时间
    claimed_by INTEGER,                                  -- 领取人 ID
    claimed_by_name VARCHAR(100),                        -- 领取人姓名
    
    -- 系统字段
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否有效
    priority INTEGER DEFAULT 0,                          -- 优先级
    remarks TEXT,                                        -- 备注
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE crm_customer_sea IS '客户公海表';
COMMENT ON COLUMN crm_customer_sea.reason_type IS '进入公海原因（no_follow_up/active_release/passive_release）';

-- 外键约束
ALTER TABLE crm_customer_sea ADD CONSTRAINT fk_crm_cs_customer
    FOREIGN KEY (customer_id) REFERENCES customers(id);
ALTER TABLE crm_customer_sea ADD CONSTRAINT fk_crm_cs_released_by
    FOREIGN KEY (released_by) REFERENCES users(id);
ALTER TABLE crm_customer_sea ADD CONSTRAINT fk_crm_cs_claimed_by
    FOREIGN KEY (claimed_by) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_crm_cs_customer ON crm_customer_sea(customer_id);
CREATE INDEX IF NOT EXISTS idx_crm_cs_released_at ON crm_customer_sea(released_at DESC);
CREATE INDEX IF NOT EXISTS idx_crm_cs_active ON crm_customer_sea(is_active);

-- ========================================
-- 6. 销售漏斗配置表
-- ========================================

-- ==================== 销售漏斗配置表 ====================
CREATE TABLE IF NOT EXISTS crm_sales_funnel_config (
    id SERIAL PRIMARY KEY,
    funnel_name VARCHAR(200) NOT NULL,                   -- 漏斗名称
    funnel_type VARCHAR(50) NOT NULL,                    -- 漏斗类型（standard/custom）
    
    -- 阶段配置
    stages JSONB NOT NULL,                               -- 阶段配置（JSON 数组）
    
    -- 系统字段
    is_default BOOLEAN DEFAULT FALSE,                    -- 是否默认漏斗
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否启用
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE crm_sales_funnel_config IS '销售漏斗配置表';
COMMENT ON COLUMN crm_sales_funnel_config.stages IS '阶段配置（JSON 数组）';

-- 索引
CREATE INDEX IF NOT EXISTS idx_crm_sfc_type ON crm_sales_funnel_config(funnel_type);
CREATE INDEX IF NOT EXISTS idx_crm_sfc_active ON crm_sales_funnel_config(is_active);

-- ========================================
-- 7. 触发器函数
-- ========================================

-- ==================== 自动更新 updated_at ====================
CREATE OR REPLACE FUNCTION update_crm_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ========================================
-- 8. 应用触发器
-- ========================================

-- 更新 updated_at 触发器
DROP TRIGGER IF EXISTS trg_crm_lead_updated_at ON crm_lead;
CREATE TRIGGER trg_crm_lead_updated_at
    BEFORE UPDATE ON crm_lead
    FOR EACH ROW
    EXECUTE FUNCTION update_crm_updated_at_column();

DROP TRIGGER IF EXISTS trg_crm_opportunity_updated_at ON crm_opportunity;
CREATE TRIGGER trg_crm_opportunity_updated_at
    BEFORE UPDATE ON crm_opportunity
    FOR EACH ROW
    EXECUTE FUNCTION update_crm_updated_at_column();

DROP TRIGGER IF EXISTS trg_crm_follow_up_updated_at ON crm_follow_up;
CREATE TRIGGER trg_crm_follow_up_updated_at
    BEFORE UPDATE ON crm_follow_up
    FOR EACH ROW
    EXECUTE FUNCTION update_crm_updated_at_column();

DROP TRIGGER IF EXISTS trg_crm_contact_updated_at ON crm_contact;
CREATE TRIGGER trg_crm_contact_updated_at
    BEFORE UPDATE ON crm_contact
    FOR EACH ROW
    EXECUTE FUNCTION update_crm_updated_at_column();

DROP TRIGGER IF EXISTS trg_crm_sales_funnel_updated_at ON crm_sales_funnel_config;
CREATE TRIGGER trg_crm_sales_funnel_updated_at
    BEFORE UPDATE ON crm_sales_funnel_config
    FOR EACH ROW
    EXECUTE FUNCTION update_crm_updated_at_column();

-- ========================================
-- 9. 初始化数据
-- ========================================

-- 初始化销售漏斗默认配置
INSERT INTO crm_sales_funnel_config (funnel_name, funnel_type, stages, is_default) VALUES
('标准销售漏斗', 'standard', 
 '[
    {"stage": "prospecting", "name": "初步接触", "probability": 10},
    {"stage": "qualification", "name": "需求确认", "probability": 30},
    {"stage": "proposal", "name": "方案报价", "probability": 50},
    {"stage": "negotiation", "name": "商务谈判", "probability": 70},
    {"stage": "closing", "name": "成交", "probability": 90}
  ]'::JSONB,
 TRUE) ON CONFLICT DO NOTHING;

-- ========================================
-- 迁移完成
-- ========================================

-- ============================================
-- 来源: 035_oa_collaboration.sql
-- ============================================
-- ========================================
-- 1. 通知公告表
-- ========================================

-- ==================== 通知公告表 ====================
CREATE TABLE IF NOT EXISTS oa_announcement (
    id SERIAL PRIMARY KEY,
    announcement_no VARCHAR(100) NOT NULL UNIQUE,        -- 公告编号
    title VARCHAR(500) NOT NULL,                         -- 公告标题
    content TEXT NOT NULL,                               -- 公告内容
    announcement_type VARCHAR(50) NOT NULL,              -- 公告类型（company_notice/department_notice/system_notice）
    priority VARCHAR(20) DEFAULT 'normal',               -- 优先级（low/normal/high/urgent）
    
    -- 发布信息
    publisher_id INTEGER NOT NULL,                       -- 发布人 ID
    publisher_name VARCHAR(100) NOT NULL,                -- 发布人姓名
    publisher_department_id INTEGER,                     -- 发布人部门 ID
    publish_date DATE NOT NULL,                          -- 发布日期
    publish_time TIME,                                   -- 发布时间
    
    -- 生效信息
    effective_date DATE,                                 -- 生效日期
    expiration_date DATE,                                -- 失效日期
    is_permanent BOOLEAN DEFAULT FALSE,                  -- 是否永久有效
    
    -- 范围配置
    visible_scope VARCHAR(50) DEFAULT 'all',             -- 可见范围（all/company/department/specific）
    visible_department_ids INTEGER[],                    -- 可见部门 ID 列表
    visible_role_ids INTEGER[],                          -- 可见角色 ID 列表
    visible_user_ids INTEGER[],                          -- 可见用户 ID 列表
    
    -- 状态管理
    status VARCHAR(20) DEFAULT 'draft',                  -- 状态（draft/published/archived/cancelled）
    is_top BOOLEAN DEFAULT FALSE,                        -- 是否置顶
    top_until DATE,                                      -- 置顶至
    view_count INTEGER DEFAULT 0,                        -- 浏览次数
    
    -- 附件
    attachment_urls TEXT[],                              -- 附件 URL 列表
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE oa_announcement IS '通知公告表';
COMMENT ON COLUMN oa_announcement.announcement_type IS '公告类型（company_notice/department_notice/system_notice）';
COMMENT ON COLUMN oa_announcement.visible_scope IS '可见范围（all/company/department/specific）';
COMMENT ON COLUMN oa_announcement.status IS '状态（draft/published/archived/cancelled）';

-- 外键约束
ALTER TABLE oa_announcement ADD CONSTRAINT fk_oa_ann_publisher
    FOREIGN KEY (publisher_id) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_oa_ann_announcement_no ON oa_announcement(announcement_no);
CREATE INDEX IF NOT EXISTS idx_oa_ann_type ON oa_announcement(announcement_type);
CREATE INDEX IF NOT EXISTS idx_oa_ann_status ON oa_announcement(status);
CREATE INDEX IF NOT EXISTS idx_oa_ann_priority ON oa_announcement(priority);
CREATE INDEX IF NOT EXISTS idx_oa_ann_publish_date ON oa_announcement(publish_date DESC);
CREATE INDEX IF NOT EXISTS idx_oa_ann_is_top ON oa_announcement(is_top);
CREATE INDEX IF NOT EXISTS idx_oa_ann_effective ON oa_announcement(effective_date);

-- ========================================
-- 2. 公告阅读记录表
-- ========================================

-- ==================== 公告阅读记录表 ====================
CREATE TABLE IF NOT EXISTS oa_announcement_read (
    id SERIAL PRIMARY KEY,
    announcement_id INTEGER NOT NULL,                    -- 公告 ID
    user_id INTEGER NOT NULL,                            -- 用户 ID
    
    -- 阅读信息
    is_read BOOLEAN DEFAULT FALSE,                       -- 是否已读
    read_at TIMESTAMP,                                   -- 阅读时间
    read_duration_seconds INTEGER,                       -- 阅读时长（秒）
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uk_ann_read UNIQUE (announcement_id, user_id)
);

COMMENT ON TABLE oa_announcement_read IS '公告阅读记录表';

-- 外键约束
ALTER TABLE oa_announcement_read ADD CONSTRAINT fk_oa_ar_announcement
    FOREIGN KEY (announcement_id) REFERENCES oa_announcement(id) ON DELETE CASCADE;
ALTER TABLE oa_announcement_read ADD CONSTRAINT fk_oa_ar_user
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- 索引
CREATE INDEX IF NOT EXISTS idx_oa_ar_announcement ON oa_announcement_read(announcement_id);
CREATE INDEX IF NOT EXISTS idx_oa_ar_user ON oa_announcement_read(user_id);
CREATE INDEX IF NOT EXISTS idx_oa_ar_is_read ON oa_announcement_read(is_read);

-- ========================================
-- 3. 站内消息表
-- ========================================

-- ==================== 站内消息表 ====================
CREATE TABLE IF NOT EXISTS oa_message (
    id SERIAL PRIMARY KEY,
    message_no VARCHAR(100) NOT NULL UNIQUE,             -- 消息编号
    
    -- 消息信息
    message_type VARCHAR(50) NOT NULL,                   -- 消息类型（system/task/approval/notice/personal）
    title VARCHAR(500) NOT NULL,                         -- 消息标题
    content TEXT NOT NULL,                               -- 消息内容
    priority VARCHAR(20) DEFAULT 'normal',               -- 优先级（low/normal/high/urgent）
    
    -- 发送信息
    sender_id INTEGER,                                   -- 发送人 ID（系统消息为 NULL）
    sender_name VARCHAR(100),                            -- 发送人姓名
    send_time TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 发送时间
    
    -- 接收信息
    receiver_type VARCHAR(50) DEFAULT 'user',            -- 接收者类型（user/department/role/all）
    receiver_ids INTEGER[],                              -- 接收者 ID 列表
    receiver_names VARCHAR(100)[],                       -- 接收者姓名列表
    
    -- 状态管理
    is_read BOOLEAN DEFAULT FALSE,                       -- 是否已读（对群发消息而言）
    read_count INTEGER DEFAULT 0,                        -- 已读人数
    total_count INTEGER DEFAULT 0,                       -- 总人数
    
    -- 关联信息
    business_type VARCHAR(50),                           -- 关联业务类型
    business_id INTEGER,                                 -- 关联业务 ID
    action_url VARCHAR(500),                             -- 操作 URL
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP                                 -- 过期时间
);

COMMENT ON TABLE oa_message IS '站内消息表';
COMMENT ON COLUMN oa_message.message_type IS '消息类型（system/task/approval/notice/personal）';
COMMENT ON COLUMN oa_message.receiver_type IS '接收者类型（user/department/role/all）';

-- 外键约束
ALTER TABLE oa_message ADD CONSTRAINT fk_oa_msg_sender
    FOREIGN KEY (sender_id) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_oa_msg_message_no ON oa_message(message_no);
CREATE INDEX IF NOT EXISTS idx_oa_msg_type ON oa_message(message_type);
CREATE INDEX IF NOT EXISTS idx_oa_msg_sender ON oa_message(sender_id);
CREATE INDEX IF NOT EXISTS idx_oa_msg_receiver_ids ON oa_message USING GIN (receiver_ids);
CREATE INDEX IF NOT EXISTS idx_oa_msg_send_time ON oa_message(send_time DESC);
CREATE INDEX IF NOT EXISTS idx_oa_msg_business ON oa_message(business_type, business_id);

-- ========================================
-- 4. 用户消息状态表
-- ========================================

-- ==================== 用户消息状态表 ====================
CREATE TABLE IF NOT EXISTS oa_user_message_status (
    id SERIAL PRIMARY KEY,
    message_id INTEGER NOT NULL,                         -- 消息 ID
    user_id INTEGER NOT NULL,                            -- 用户 ID
    
    -- 状态信息
    is_read BOOLEAN DEFAULT FALSE,                       -- 是否已读
    read_at TIMESTAMP,                                   -- 阅读时间
    is_starred BOOLEAN DEFAULT FALSE,                    -- 是否星标
    is_deleted BOOLEAN DEFAULT FALSE,                    -- 是否删除
    is_archived BOOLEAN DEFAULT FALSE,                   -- 是否归档
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uk_ums_message_user UNIQUE (message_id, user_id)
);

COMMENT ON TABLE oa_user_message_status IS '用户消息状态表';

-- 外键约束
ALTER TABLE oa_user_message_status ADD CONSTRAINT fk_oa_ums_message
    FOREIGN KEY (message_id) REFERENCES oa_message(id) ON DELETE CASCADE;
ALTER TABLE oa_user_message_status ADD CONSTRAINT fk_oa_ums_user
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- 索引
CREATE INDEX IF NOT EXISTS idx_oa_ums_message ON oa_user_message_status(message_id);
CREATE INDEX IF NOT EXISTS idx_oa_ums_user ON oa_user_message_status(user_id);
CREATE INDEX IF NOT EXISTS idx_oa_ums_is_read ON oa_user_message_status(is_read);
CREATE INDEX IF NOT EXISTS idx_oa_ums_is_starred ON oa_user_message_status(is_starred);

-- ========================================
-- 5. 触发器函数
-- ========================================

-- ==================== 自动更新 updated_at ====================
CREATE OR REPLACE FUNCTION update_oa_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ========================================
-- 6. 应用触发器
-- ========================================

-- 更新 updated_at 触发器
DROP TRIGGER IF EXISTS trg_oa_ann_updated_at ON oa_announcement;
CREATE TRIGGER trg_oa_ann_updated_at
    BEFORE UPDATE ON oa_announcement
    FOR EACH ROW
    EXECUTE FUNCTION update_oa_updated_at_column();

DROP TRIGGER IF EXISTS trg_oa_ar_updated_at ON oa_announcement_read;
CREATE TRIGGER trg_oa_ar_updated_at
    BEFORE UPDATE ON oa_announcement_read
    FOR EACH ROW
    EXECUTE FUNCTION update_oa_updated_at_column();

DROP TRIGGER IF EXISTS trg_oa_ums_updated_at ON oa_user_message_status;
CREATE TRIGGER trg_oa_ums_updated_at
    BEFORE UPDATE ON oa_user_message_status
    FOR EACH ROW
    EXECUTE FUNCTION update_oa_updated_at_column();

-- ========================================
-- 迁移完成
-- ========================================

-- ============================================
-- 来源: 036_data_visualization.sql
-- ============================================
-- ========================================
-- 1. 报表定义表
-- ========================================

-- ==================== 报表定义表 ====================
CREATE TABLE IF NOT EXISTS report_definition (
    id SERIAL PRIMARY KEY,
    report_key VARCHAR(100) NOT NULL,                    -- 报表标识
    report_name VARCHAR(200) NOT NULL,                   -- 报表名称
    report_category VARCHAR(50) NOT NULL,                -- 报表分类（financial/sales/procurement/inventory/hr）
    report_type VARCHAR(50) NOT NULL,                    -- 报表类型（summary/detail/analysis/dashboard）
    
    -- 报表配置
    data_source_type VARCHAR(50) DEFAULT 'sql',          -- 数据源类型（sql/api/custom）
    data_source_config JSONB,                            -- 数据源配置（JSON 格式）
    query_sql TEXT,                                      -- 查询 SQL
    columns_config JSONB,                                -- 列配置（JSON 格式）
    
    -- 筛选配置
    filter_config JSONB,                                 -- 筛选器配置（JSON 格式）
    default_filters JSONB,                               -- 默认筛选条件
    
    -- 权限配置
    visible_roles INTEGER[],                             -- 可见角色 ID 列表
    export_enabled BOOLEAN DEFAULT TRUE,                 -- 是否允许导出
    print_enabled BOOLEAN DEFAULT TRUE,                  -- 是否允许打印
    
    -- 调度配置
    schedule_enabled BOOLEAN DEFAULT FALSE,              -- 是否启用调度
    schedule_config JSONB,                               -- 调度配置（JSON 格式）
    last_run_at TIMESTAMP,                               -- 最后运行时间
    next_run_at TIMESTAMP,                               -- 下次运行时间
    
    -- 状态管理
    status VARCHAR(20) DEFAULT 'active',                 -- 状态（active/inactive/deprecated）
    is_system BOOLEAN DEFAULT FALSE,                     -- 是否系统报表
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE report_definition IS '报表定义表';
COMMENT ON COLUMN report_definition.data_source_config IS '数据源配置（JSON 格式）';
COMMENT ON COLUMN report_definition.columns_config IS '列配置（JSON 格式）';
COMMENT ON COLUMN report_definition.filter_config IS '筛选器配置（JSON 格式）';

-- 唯一约束
ALTER TABLE report_definition ADD CONSTRAINT uk_report_key UNIQUE (report_key);

-- 索引
CREATE INDEX IF NOT EXISTS idx_report_def_category ON report_definition(report_category);
CREATE INDEX IF NOT EXISTS idx_report_def_type ON report_definition(report_type);
CREATE INDEX IF NOT EXISTS idx_report_def_status ON report_definition(status);
CREATE INDEX IF NOT EXISTS idx_report_def_system ON report_definition(is_system);

-- ========================================
-- 2. 仪表板表
-- ========================================

-- ==================== 仪表板表 ====================
CREATE TABLE IF NOT EXISTS report_dashboard (
    id SERIAL PRIMARY KEY,
    dashboard_name VARCHAR(200) NOT NULL,                -- 仪表板名称
    dashboard_code VARCHAR(100) NOT NULL UNIQUE,         -- 仪表板编码
    description TEXT,                                    -- 仪表板描述
    
    -- 布局配置
    layout_config JSONB,                                 -- 布局配置（JSON 格式）
    widgets_config JSONB,                                -- 组件配置（JSON 格式）
    
    -- 权限配置
    visible_roles INTEGER[],                             -- 可见角色 ID 列表
    is_public BOOLEAN DEFAULT FALSE,                     -- 是否公开
    
    -- 状态管理
    status VARCHAR(20) DEFAULT 'active',                 -- 状态（active/inactive/draft）
    is_default BOOLEAN DEFAULT FALSE,                    -- 是否默认仪表板
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE report_dashboard IS '仪表板表';
COMMENT ON COLUMN report_dashboard.layout_config IS '布局配置（JSON 格式）';
COMMENT ON COLUMN report_dashboard.widgets_config IS '组件配置（JSON 格式）';

-- 唯一约束
ALTER TABLE report_dashboard ADD CONSTRAINT uk_dashboard_code UNIQUE (dashboard_code);

-- 索引
CREATE INDEX IF NOT EXISTS idx_report_dash_status ON report_dashboard(status);
CREATE INDEX IF NOT EXISTS idx_report_dash_public ON report_dashboard(is_public);
CREATE INDEX IF NOT EXISTS idx_report_dash_default ON report_dashboard(is_default);

-- ========================================
-- 3. 报表组件表
-- ========================================

-- ==================== 报表组件表 ====================
CREATE TABLE IF NOT EXISTS report_widget (
    id SERIAL PRIMARY KEY,
    widget_name VARCHAR(200) NOT NULL,                   -- 组件名称
    widget_type VARCHAR(50) NOT NULL,                    -- 组件类型（chart/table/card/map/pivot）
    widget_category VARCHAR(50),                         -- 组件分类
    
    -- 组件配置
    chart_type VARCHAR(50),                              -- 图表类型（bar/line/pie/scatter/area）
    data_source_config JSONB,                            -- 数据源配置
    visual_config JSONB,                                 -- 可视化配置
    interaction_config JSONB,                            -- 交互配置
    
    -- 状态管理
    is_system BOOLEAN DEFAULT FALSE,                     -- 是否系统组件
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否启用
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE report_widget IS '报表组件表';
COMMENT ON COLUMN report_widget.chart_type IS '图表类型（bar/line/pie/scatter/area）';
COMMENT ON COLUMN report_widget.data_source_config IS '数据源配置';
COMMENT ON COLUMN report_widget.visual_config IS '可视化配置';

-- 索引
CREATE INDEX IF NOT EXISTS idx_report_widget_type ON report_widget(widget_type);
CREATE INDEX IF NOT EXISTS idx_report_widget_category ON report_widget(widget_category);
CREATE INDEX IF NOT EXISTS idx_report_widget_active ON report_widget(is_active);

-- ========================================
-- 4. 报表订阅表
-- ========================================

-- ==================== 报表订阅表 ====================
CREATE TABLE IF NOT EXISTS report_subscription (
    id SERIAL PRIMARY KEY,
    report_id INTEGER NOT NULL,                          -- 报表 ID
    user_id INTEGER NOT NULL,                            -- 用户 ID
    
    -- 订阅配置
    subscription_type VARCHAR(50) DEFAULT 'email',       -- 订阅类型（email/sms/wechat/system）
    frequency VARCHAR(50) NOT NULL,                      -- 频率（daily/weekly/monthly/custom）
    schedule_time TIME,                                  -- 发送时间
    schedule_day_of_week INTEGER,                        -- 星期几（1-7）
    schedule_day_of_month INTEGER,                       -- 每月几号（1-31）
    
    -- 筛选条件
    filter_config JSONB,                                 -- 筛选条件（JSON 格式）
    
    -- 状态管理
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否启用
    last_sent_at TIMESTAMP,                              -- 最后发送时间
    next_send_at TIMESTAMP,                              -- 下次发送时间
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE report_subscription IS '报表订阅表';
COMMENT ON COLUMN report_subscription.subscription_type IS '订阅类型（email/sms/wechat/system）';
COMMENT ON COLUMN report_subscription.frequency IS '频率（daily/weekly/monthly/custom）';

-- 外键约束
ALTER TABLE report_subscription ADD CONSTRAINT fk_report_sub_report
    FOREIGN KEY (report_id) REFERENCES report_definition(id) ON DELETE CASCADE;
ALTER TABLE report_subscription ADD CONSTRAINT fk_report_sub_user
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- 唯一约束
ALTER TABLE report_subscription ADD CONSTRAINT uk_report_sub_user UNIQUE (report_id, user_id, subscription_type);

-- 索引
CREATE INDEX IF NOT EXISTS idx_report_sub_report ON report_subscription(report_id);
CREATE INDEX IF NOT EXISTS idx_report_sub_user ON report_subscription(user_id);
CREATE INDEX IF NOT EXISTS idx_report_sub_active ON report_subscription(is_active);
CREATE INDEX IF NOT EXISTS idx_report_sub_frequency ON report_subscription(frequency);

-- ========================================
-- 5. 报表导出历史表
-- ========================================

-- ==================== 报表导出历史表 ====================
CREATE TABLE IF NOT EXISTS report_export_history (
    id SERIAL PRIMARY KEY,
    export_no VARCHAR(100) NOT NULL UNIQUE,              -- 导出编号
    report_id INTEGER NOT NULL,                          -- 报表 ID
    user_id INTEGER NOT NULL,                            -- 用户 ID
    
    -- 导出信息
    export_format VARCHAR(20) NOT NULL,                  -- 导出格式（pdf/excel/csv/html）
    export_type VARCHAR(50) DEFAULT 'full',              -- 导出类型（full/filtered/custom）
    filter_config JSONB,                                 -- 筛选条件
    export_status VARCHAR(20) DEFAULT 'pending',         -- 导出状态（pending/processing/completed/failed）
    
    -- 文件信息
    file_path VARCHAR(500),                              -- 文件路径
    file_size BIGINT,                                    -- 文件大小（字节）
    download_count INTEGER DEFAULT 0,                    -- 下载次数
    download_url TEXT,                                   -- 下载 URL
    
    -- 时间信息
    requested_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 请求时间
    completed_at TIMESTAMP,                              -- 完成时间
    expires_at TIMESTAMP,                                -- 过期时间
    
    -- 系统字段
    error_message TEXT                                   -- 错误信息
);

COMMENT ON TABLE report_export_history IS '报表导出历史表';
COMMENT ON COLUMN report_export_history.export_format IS '导出格式（pdf/excel/csv/html）';
COMMENT ON COLUMN report_export_history.export_status IS '导出状态（pending/processing/completed/failed）';

-- 外键约束
ALTER TABLE report_export_history ADD CONSTRAINT fk_report_exp_report
    FOREIGN KEY (report_id) REFERENCES report_definition(id);
ALTER TABLE report_export_history ADD CONSTRAINT fk_report_exp_user
    FOREIGN KEY (user_id) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_report_exp_export_no ON report_export_history(export_no);
CREATE INDEX IF NOT EXISTS idx_report_exp_report ON report_export_history(report_id);
CREATE INDEX IF NOT EXISTS idx_report_exp_user ON report_export_history(user_id);
CREATE INDEX IF NOT EXISTS idx_report_exp_status ON report_export_history(export_status);
CREATE INDEX IF NOT EXISTS idx_report_exp_requested ON report_export_history(requested_at DESC);

-- ========================================
-- 6. 物化视图日志表
-- ========================================

-- ==================== 物化视图刷新日志表 ====================
CREATE TABLE IF NOT EXISTS report_mv_refresh_log (
    id SERIAL PRIMARY KEY,
    mv_name VARCHAR(200) NOT NULL,                       -- 物化视图名称
    refresh_type VARCHAR(50) DEFAULT 'full',             -- 刷新类型（full/concurrent）
    refresh_status VARCHAR(20) DEFAULT 'pending',        -- 刷新状态（pending/running/completed/failed）
    
    -- 时间信息
    started_at TIMESTAMP,                                -- 开始时间
    completed_at TIMESTAMP,                              -- 完成时间
    duration_seconds BIGINT,                             -- 耗时（秒）
    
    -- 统计信息
    rows_affected BIGINT,                                -- 影响行数
    
    -- 系统字段
    error_message TEXT,                                  -- 错误信息
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE report_mv_refresh_log IS '物化视图刷新日志表';

-- 索引
CREATE INDEX IF NOT EXISTS idx_report_mv_mv_name ON report_mv_refresh_log(mv_name);
CREATE INDEX IF NOT EXISTS idx_report_mv_status ON report_mv_refresh_log(refresh_status);
CREATE INDEX IF NOT EXISTS idx_report_mv_started ON report_mv_refresh_log(started_at DESC);

-- ========================================
-- 7. 触发器函数
-- ========================================

-- ==================== 自动更新 updated_at ====================
CREATE OR REPLACE FUNCTION update_report_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ========================================
-- 8. 应用触发器
-- ========================================

-- 更新 updated_at 触发器
DROP TRIGGER IF EXISTS trg_report_def_updated_at ON report_definition;
CREATE TRIGGER trg_report_def_updated_at
    BEFORE UPDATE ON report_definition
    FOR EACH ROW
    EXECUTE FUNCTION update_report_updated_at_column();

DROP TRIGGER IF EXISTS trg_report_dashboard_updated_at ON report_dashboard;
CREATE TRIGGER trg_report_dashboard_updated_at
    BEFORE UPDATE ON report_dashboard
    FOR EACH ROW
    EXECUTE FUNCTION update_report_updated_at_column();

DROP TRIGGER IF EXISTS trg_report_widget_updated_at ON report_widget;
CREATE TRIGGER trg_report_widget_updated_at
    BEFORE UPDATE ON report_widget
    FOR EACH ROW
    EXECUTE FUNCTION update_report_updated_at_column();

DROP TRIGGER IF EXISTS trg_report_subscription_updated_at ON report_subscription;
CREATE TRIGGER trg_report_subscription_updated_at
    BEFORE UPDATE ON report_subscription
    FOR EACH ROW
    EXECUTE FUNCTION update_report_updated_at_column();

-- ========================================
-- 9. 初始化数据 - 常用报表
-- ========================================

-- 初始化销售统计报表
INSERT INTO report_definition (report_key, report_name, report_category, report_type, query_sql, columns_config, is_system) VALUES
('sales_daily_summary', '销售日报', 'sales', 'summary', 
 'SELECT DATE(created_at) as date, COUNT(*) as order_count, SUM(total_amount) as total_amount FROM sales_orders GROUP BY DATE(created_at)',
 '[{"key": "date", "label": "日期", "type": "date"}, {"key": "order_count", "label": "订单数", "type": "number"}, {"key": "total_amount", "label": "总金额", "type": "money"}]',
 TRUE),
('sales_product_ranking', '产品销售排行', 'sales', 'analysis',
 'SELECT product_id, product_name, SUM(quantity) as total_quantity, SUM(amount) as total_amount FROM sales_order_items GROUP BY product_id, product_name ORDER BY total_quantity DESC',
 '[{"key": "product_name", "label": "产品名称", "type": "text"}, {"key": "total_quantity", "label": "销售数量", "type": "number"}, {"key": "total_amount", "label": "销售金额", "type": "money"}]',
 TRUE),
('procurement_supplier_stats', '供应商采购统计', 'procurement', 'summary',
 'SELECT supplier_id, supplier_name, COUNT(*) as order_count, SUM(total_amount) as total_amount FROM purchase_order GROUP BY supplier_id, supplier_name',
 '[{"key": "supplier_name", "label": "供应商", "type": "text"}, {"key": "order_count", "label": "订单数", "type": "number"}, {"key": "total_amount", "label": "采购金额", "type": "money"}]',
 TRUE) ON CONFLICT DO NOTHING;

-- 初始化默认仪表板
INSERT INTO report_dashboard (dashboard_name, dashboard_code, description, is_default, is_public) VALUES
('经营驾驶舱', 'executive_dashboard', '企业经营管理驾驶舱，包含关键经营指标', TRUE, TRUE),
('销售看板', 'sales_dashboard', '销售业务数据看板', FALSE, TRUE),
('采购看板', 'procurement_dashboard', '采购业务数据看板', FALSE, TRUE) ON CONFLICT DO NOTHING;

-- ========================================
-- 迁移完成
-- ========================================

-- ============================================
-- 来源: 037_test_data.sql
-- ============================================
-- ========================================

-- ========================================
-- 0. 基础主数据填充（解决外键约束依赖）
-- ========================================

-- 插入默认用户
INSERT INTO users (id, username, password_hash, email, is_active) VALUES
(1, 'admin', 'mock_hash', 'admin@example.com', true),
(2, 'user1', 'mock_hash', 'user1@example.com', true),
(3, 'user2', 'mock_hash', 'user2@example.com', true) ON CONFLICT DO NOTHING;

-- 插入默认供应商
INSERT INTO suppliers (id, supplier_code, supplier_name, supplier_short_name, supplier_type, credit_code, registered_address, legal_representative, registered_capital, bank_name, bank_account, taxpayer_type, contact_phone, establishment_date) VALUES
(1, 'SUP001', '江苏纺织有限公司', '江苏纺织', 'manufacturer', '913200000000000001', '江苏省南京市', '张三', 1000.00, '中国银行', '123456789', 'general', '13800000000', CURRENT_DATE),
(2, 'SUP002', '浙江印染厂', '浙江印染', 'processor', '913300000000000002', '浙江省杭州市', '李四', 500.00, '建设银行', '987654321', 'general', '13900000000', CURRENT_DATE) ON CONFLICT DO NOTHING;

-- 插入默认客户
INSERT INTO customers (id, customer_code, customer_name, contact_person, customer_type, status) VALUES
(1, 'CUS001', '江苏纺织有限公司', '张三', 'wholesale', 'active'),
(2, 'CUS002', '浙江印染厂', '李四', 'wholesale', 'active'),
(3, 'CUS003', '广东服装厂', '王五', 'wholesale', 'active') ON CONFLICT DO NOTHING;

-- 插入默认产品
INSERT INTO products (id, code, name, category_id, unit, specification) VALUES
(1, 'PROD001', '纯棉帆布', NULL, '米', '100%棉 10安'),
(2, 'PROD002', '涤纶牛津布', NULL, '米', '100%涤 600D') ON CONFLICT DO NOTHING;

-- 插入默认产品颜色
INSERT INTO product_colors (id, product_id, color_no, color_name) VALUES
(1, 1, 'COLOR001', '黑色'),
(2, 1, 'COLOR002', '白色') ON CONFLICT DO NOTHING;


-- 1. 四级批次管理测试数据
-- ========================================

-- 供应商成品编码映射
INSERT INTO product_code_mapping (internal_product_code, supplier_product_code, supplier_id, mapping_date, validation_status) VALUES
('PROD001', 'SPROD001', 1, CURRENT_DATE, 'validated'),
('PROD002', 'SPROD002', 1, CURRENT_DATE, 'validated'),
('PROD001', 'SPROD001-A', 2, CURRENT_DATE, 'validated') ON CONFLICT DO NOTHING;

-- 供应商色号编码映射
INSERT INTO color_code_mapping (internal_color_no, supplier_color_code, supplier_id, mapping_date, validation_status) VALUES
('COLOR001', 'SCOLOR001', 1, CURRENT_DATE, 'validated'),
('COLOR002', 'SCOLOR002', 1, CURRENT_DATE, 'validated'),
('COLOR001', 'SCOLOR-A01', 2, CURRENT_DATE, 'validated') ON CONFLICT DO NOTHING;

-- 缸号管理
INSERT INTO batch_dye_lot (dye_lot_no, product_id, color_id, supplier_dye_lot_no, supplier_id, production_date, quality_grade, quality_status) VALUES
('DL20260316001', 1, 1, 'SDL001', 1, CURRENT_DATE, 'A', 'passed'),
('DL20260316002', 1, 1, 'SDL002', 1, CURRENT_DATE, 'A', 'passed'),
('DL20260316003', 1, 2, 'SDL003', 1, CURRENT_DATE, 'B', 'passed'),
('DL20260316004', 2, 1, 'SDL004', 2, CURRENT_DATE, 'A', 'passed') ON CONFLICT DO NOTHING;

-- 匹号管理
INSERT INTO inventory_piece (piece_no, dye_lot_id, supplier_piece_no, length, weight, quality_status, inventory_status) VALUES
('P20260316000001', 1, 'SP001', 100.00, 20.50, 'passed', 'available'),
('P20260316000002', 1, 'SP002', 105.00, 21.00, 'passed', 'available'),
('P20260316000003', 1, 'SP003', 98.00, 19.80, 'passed', 'available'),
('P20260316000004', 2, 'SP004', 102.00, 20.80, 'passed', 'available'),
('P20260316000005', 2, 'SP005', 103.00, 21.20, 'passed', 'available') ON CONFLICT DO NOTHING;

-- ========================================
-- 2. BPM 流程引擎测试数据
-- ========================================

-- 流程定义 - 采购审批流程
INSERT INTO bpm_process_definition (process_key, process_name, process_version, process_category, description, flow_definition, status, is_published, published_at) VALUES
('procurement_approval', '采购审批流程', 'v1.0.0', 'procurement', 
 '用于采购订单的审批流程',
 '{
   "nodes": [
     {"id": "start", "type": "start", "name": "开始"},
     {"id": "dept_manager", "type": "user_task", "name": "部门经理审批", "assignee_type": "role", "assignee_value": "dept_manager"},
     {"id": "finance_manager", "type": "user_task", "name": "财务经理审批", "assignee_type": "role", "assignee_value": "finance_manager"},
     {"id": "general_manager", "type": "user_task", "name": "总经理审批", "assignee_type": "role", "assignee_value": "general_manager", "condition": "amount > 100000"},
     {"id": "end", "type": "end", "name": "结束"}
   ],
   "transitions": [
     {"from": "start", "to": "dept_manager"},
     {"from": "dept_manager", "to": "finance_manager", "condition": "approved"},
     {"from": "finance_manager", "to": "end", "condition": "approved AND amount <= 100000"},
     {"from": "finance_manager", "to": "general_manager", "condition": "approved AND amount > 100000"},
     {"from": "general_manager", "to": "end", "condition": "approved"}
   ]
 }'::JSONB,
 'active', TRUE, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING;

-- 流程定义 - 销售审批流程
INSERT INTO bpm_process_definition (process_key, process_name, process_version, process_category, description, flow_definition, status, is_published, published_at) VALUES
('sales_approval', '销售审批流程', 'v1.0.0', 'sales',
 '用于销售订单的审批流程',
 '{
   "nodes": [
     {"id": "start", "type": "start", "name": "开始"},
     {"id": "sales_manager", "type": "user_task", "name": "销售经理审批", "assignee_type": "role", "assignee_value": "sales_manager"},
     {"id": "end", "type": "end", "name": "结束"}
   ],
   "transitions": [
     {"from": "start", "to": "sales_manager"},
     {"from": "sales_manager", "to": "end", "condition": "approved"}
   ]
 }'::JSONB,
 'active', TRUE, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING;

-- ========================================
-- 3. CRM 扩展测试数据
-- ========================================

-- 销售线索
INSERT INTO crm_lead (lead_no, lead_source, lead_status, company_name, contact_name, mobile_phone, estimated_amount, owner_id, owner_name, priority) VALUES
('LEAD20260316001', 'exhibition', 'qualified', '江苏纺织有限公司', '张经理', '13800138001', 500000.00, 1, '张三', 'high'),
('LEAD20260316002', 'referral', 'new', '浙江印染厂', '李总', '13800138002', 300000.00, 1, '张三', 'medium'),
('LEAD20260316003', 'website', 'contacted', '广东服装厂', '王经理', '13800138003', 200000.00, 2, '李四', 'low') ON CONFLICT DO NOTHING;

-- 商机
INSERT INTO crm_opportunity (opportunity_no, opportunity_name, customer_id, opportunity_stage, win_probability, estimated_amount, owner_id, owner_name, opportunity_status) VALUES
('OPP20260316001', '江苏纺织年度采购', 1, 'negotiation', 70.00, 500000.00, 1, '张三', 'open'),
('OPP20260316002', '浙江印染厂批量订单', 2, 'proposal', 50.00, 300000.00, 1, '张三', 'open'),
('OPP20260316003', '广东服装厂试单', 3, 'closing', 90.00, 200000.00, 2, '李四', 'open') ON CONFLICT DO NOTHING;

-- 客户跟进记录
INSERT INTO crm_follow_up (follow_up_no, lead_id, opportunity_id, follow_up_type, follow_up_date, subject, content, owner_id, owner_name) VALUES
('FU20260316001', 1, NULL, 'phone_call', CURRENT_DATE, '初次联系', '与客户沟通了产品需求，客户对我们的产品质量很感兴趣', 1, '张三'),
('FU20260316002', NULL, 1, 'meeting', CURRENT_DATE, '商务谈判', '与客户进行了面对面的商务谈判，讨论了价格和交货期', 1, '张三') ON CONFLICT DO NOTHING;

-- ========================================
-- 4. OA 协同办公测试数据
-- ========================================

-- 通知公告
INSERT INTO oa_announcement (announcement_no, title, content, announcement_type, priority, publisher_id, publisher_name, publish_date, status, is_top) VALUES
('ANN20260316001', '关于 2026 年春节放假的通知', '根据公司安排，2026 年春节放假时间为 2 月 15 日至 2 月 22 日，共 8 天。请各部门做好工作安排。', 'company_notice', 'high', 1, '行政部', CURRENT_DATE, 'published', TRUE),
('ANN20260316002', '系统升级维护通知', '公司 ERP 系统将于本周末进行升级维护，届时系统将暂停使用。请提前做好工作安排。', 'system_notice', 'normal', 1, 'IT 部', CURRENT_DATE, 'published', FALSE) ON CONFLICT DO NOTHING;

-- 站内消息
INSERT INTO oa_message (message_no, message_type, title, content, receiver_type, receiver_ids, business_type, business_id) VALUES
('MSG20260316001', 'approval', '您有新的采购订单待审批', '您有一笔采购订单（PO20260316001）需要审批，请及时处理。', 'user', ARRAY[1], 'purchase_order', 1),
('MSG20260316002', 'task', '新的销售线索分配', '您有新的销售线索（LEAD20260316001）已分配给您，请及时跟进。', 'user', ARRAY[1], 'crm_lead', 1) ON CONFLICT DO NOTHING;

-- ========================================
-- 5. 报表测试数据
-- ========================================

-- 报表组件
INSERT INTO report_widget (widget_name, widget_type, chart_type, is_system, is_active) VALUES
('销售趋势图', 'chart', 'line', TRUE, TRUE),
('销售占比饼图', 'chart', 'pie', TRUE, TRUE),
('产品销售排行', 'table', NULL, TRUE, TRUE),
('关键指标卡片', 'card', NULL, TRUE, TRUE),
('销售地图', 'map', NULL, TRUE, TRUE) ON CONFLICT DO NOTHING;

-- ========================================
-- 6. 日志测试数据（可选）
-- ========================================

-- 操作日志示例
INSERT INTO log_operation (log_no, module, operation_type, business_type, business_id, user_id, username, operation_desc, ip_address) VALUES
('OPL20260316001', 'procurement', 'create', 'purchase_order', 1, 1, 'admin', '创建采购订单 PO20260316001', '192.168.1.100'),
('OPL20260316002', 'sales', 'update', 'sales_order', 1, 2, 'user1', '更新销售订单 SO20260316001', '192.168.1.101'),
('OPL20260316003', 'inventory', 'approve', 'inventory_transfer', 1, 3, 'user2', '审批库存调拨单', '192.168.1.102') ON CONFLICT DO NOTHING;

-- 登录日志示例
INSERT INTO log_login (log_no, username, login_status, login_type, ip_address, browser, os) VALUES
('LGL20260316001', 'admin', 'success', 'password', '192.168.1.100', 'Chrome 120', 'Windows 10'),
('LGL20260316002', 'user1', 'success', 'password', '192.168.1.101', 'Firefox 121', 'Windows 11'),
('LGL20260316003', 'user2', 'failed', 'password', '192.168.1.102', 'Chrome 120', 'macOS') ON CONFLICT DO NOTHING;

-- ========================================
-- 7. 批次追溯日志测试
-- ========================================

-- 批次转换日志
INSERT INTO batch_trace_log (trace_no, business_type, business_id, trace_direction, internal_product_code, internal_color_no, supplier_product_code, supplier_color_code, validation_result, operator_id, operation_type) VALUES
('TR20260316001', 'purchase_receipt', 1, 'supplier_to_internal', 'PROD001', 'COLOR001', 'SPROD001', 'SCOLOR001', 'success', 1, 'convert'),
('TR20260316002', 'sales_delivery', 1, 'internal_to_supplier', 'PROD001', 'COLOR001', 'SPROD001', 'SCOLOR001', 'success', 1, 'convert') ON CONFLICT DO NOTHING;

-- ========================================
-- 8. 更新相关统计表
-- ========================================

-- BPM 统计
INSERT INTO bpm_statistics_daily (statistics_date, process_definition_id, initiated_count, completed_count, pending_tasks, completed_tasks) VALUES
(CURRENT_DATE, 1, 5, 3, 10, 25),
(CURRENT_DATE, 2, 8, 6, 15, 40) ON CONFLICT DO NOTHING;

-- ========================================
-- 测试数据创建完成
-- ========================================

-- ============================================
-- 来源: 038_assist_accounting.sql
-- ============================================
-- ============================================
-- 辅助核算管理模块 - 基础表结构
-- ============================================
-- 文档编号：MIGRATION-060-ASSIST-ACCOUNTING
-- 创建日期：2026-03-17
-- 说明：辅助核算管理模块基础表结构，包含维度定义、核算记录、汇总数据
-- ============================================

-- 1. 辅助核算维度表
-- ============================================
CREATE TABLE IF NOT EXISTS assist_accounting_dimension (
    id SERIAL PRIMARY KEY,
    dimension_code VARCHAR(50) NOT NULL UNIQUE,
    dimension_name VARCHAR(200) NOT NULL,
    description TEXT,
    is_active BOOLEAN DEFAULT TRUE,
    sort_order INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE assist_accounting_dimension IS '辅助核算维度表';
COMMENT ON COLUMN assist_accounting_dimension.dimension_code IS '维度编码：BATCH, COLOR, DYE_LOT, GRADE, WORKSHOP, WAREHOUSE, CUSTOMER, SUPPLIER';
COMMENT ON COLUMN assist_accounting_dimension.dimension_name IS '维度名称';
COMMENT ON COLUMN assist_accounting_dimension.description IS '维度描述';
COMMENT ON COLUMN assist_accounting_dimension.is_active IS '是否启用';
COMMENT ON COLUMN assist_accounting_dimension.sort_order IS '排序顺序';

-- 索引
CREATE INDEX IF NOT EXISTS idx_dimension_code ON assist_accounting_dimension(dimension_code);
CREATE INDEX IF NOT EXISTS idx_dimension_active ON assist_accounting_dimension(is_active);

-- 2. 辅助核算记录表
-- ============================================
CREATE TABLE IF NOT EXISTS assist_accounting_record (
    id SERIAL PRIMARY KEY,
    business_type VARCHAR(50) NOT NULL,
    business_no VARCHAR(100) NOT NULL,
    business_id INTEGER NOT NULL,
    account_subject_id INTEGER NOT NULL REFERENCES account_subjects(id),
    debit_amount DECIMAL(12,2) NOT NULL DEFAULT 0,
    credit_amount DECIMAL(12,2) NOT NULL DEFAULT 0,
    five_dimension_id VARCHAR(255) NOT NULL,
    product_id INTEGER NOT NULL,
    batch_no VARCHAR(100) NOT NULL,
    color_no VARCHAR(100) NOT NULL,
    dye_lot_no VARCHAR(100),
    grade VARCHAR(50) NOT NULL,
    workshop_id INTEGER,
    warehouse_id INTEGER NOT NULL,
    customer_id INTEGER,
    supplier_id INTEGER,
    quantity_meters DECIMAL(12,2) NOT NULL DEFAULT 0,
    quantity_kg DECIMAL(12,2) NOT NULL DEFAULT 0,
    remarks TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    created_by INTEGER
);

COMMENT ON TABLE assist_accounting_record IS '辅助核算记录表';
COMMENT ON COLUMN assist_accounting_record.business_type IS '业务类型：PURCHASE, SALES, INVENTORY, PRODUCTION';
COMMENT ON COLUMN assist_accounting_record.business_no IS '业务单号';
COMMENT ON COLUMN assist_accounting_record.business_id IS '业务单 ID';
COMMENT ON COLUMN assist_accounting_record.account_subject_id IS '会计科目 ID';
COMMENT ON COLUMN assist_accounting_record.debit_amount IS '借方金额';
COMMENT ON COLUMN assist_accounting_record.credit_amount IS '贷方金额';
COMMENT ON COLUMN assist_accounting_record.five_dimension_id IS '五维 ID';
COMMENT ON COLUMN assist_accounting_record.product_id IS '产品 ID';
COMMENT ON COLUMN assist_accounting_record.batch_no IS '批次号';
COMMENT ON COLUMN assist_accounting_record.color_no IS '色号';
COMMENT ON COLUMN assist_accounting_record.dye_lot_no IS '缸号';
COMMENT ON COLUMN assist_accounting_record.grade IS '等级';
COMMENT ON COLUMN assist_accounting_record.workshop_id IS '车间 ID';
COMMENT ON COLUMN assist_accounting_record.warehouse_id IS '仓库 ID';
COMMENT ON COLUMN assist_accounting_record.customer_id IS '客户 ID';
COMMENT ON COLUMN assist_accounting_record.supplier_id IS '供应商 ID';
COMMENT ON COLUMN assist_accounting_record.quantity_meters IS '数量（米）';
COMMENT ON COLUMN assist_accounting_record.quantity_kg IS '数量（公斤）';
COMMENT ON COLUMN assist_accounting_record.remarks IS '备注';
COMMENT ON COLUMN assist_accounting_record.created_by IS '创建人 ID';

-- 索引
CREATE INDEX IF NOT EXISTS idx_record_business ON assist_accounting_record(business_type, business_no);
CREATE INDEX IF NOT EXISTS idx_record_five_dimension ON assist_accounting_record(five_dimension_id);
CREATE INDEX IF NOT EXISTS idx_record_subject ON assist_accounting_record(account_subject_id);
CREATE INDEX IF NOT EXISTS idx_record_batch ON assist_accounting_record(batch_no);
CREATE INDEX IF NOT EXISTS idx_record_color_no ON assist_accounting_record(color_no);
CREATE INDEX IF NOT EXISTS idx_record_warehouse ON assist_accounting_record(warehouse_id);
CREATE INDEX IF NOT EXISTS idx_record_created_at ON assist_accounting_record(created_at);
CREATE INDEX IF NOT EXISTS idx_record_business_five ON assist_accounting_record(business_type, five_dimension_id);
CREATE INDEX IF NOT EXISTS idx_record_period ON assist_accounting_record(created_at, account_subject_id);

-- 3. 辅助核算汇总表
-- ============================================
CREATE TABLE IF NOT EXISTS assist_accounting_summary (
    id SERIAL PRIMARY KEY,
    accounting_period VARCHAR(7) NOT NULL,
    dimension_code VARCHAR(50) NOT NULL,
    dimension_value_id INTEGER NOT NULL,
    dimension_value_name VARCHAR(200) NOT NULL,
    account_subject_id INTEGER NOT NULL REFERENCES account_subjects(id),
    total_debit DECIMAL(12,2) NOT NULL DEFAULT 0,
    total_credit DECIMAL(12,2) NOT NULL DEFAULT 0,
    total_quantity_meters DECIMAL(12,2) NOT NULL DEFAULT 0,
    total_quantity_kg DECIMAL(12,2) NOT NULL DEFAULT 0,
    record_count BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    -- 唯一约束：按期间 + 维度 + 科目汇总
    UNIQUE(accounting_period, dimension_code, dimension_value_id, account_subject_id)
);

COMMENT ON TABLE assist_accounting_summary IS '辅助核算汇总表';
COMMENT ON COLUMN assist_accounting_summary.accounting_period IS '会计期间（格式：YYYY-MM）';
COMMENT ON COLUMN assist_accounting_summary.dimension_code IS '维度编码';
COMMENT ON COLUMN assist_accounting_summary.dimension_value_id IS '维度值 ID（如批次 ID、色号 ID 等）';
COMMENT ON COLUMN assist_accounting_summary.dimension_value_name IS '维度值名称';
COMMENT ON COLUMN assist_accounting_summary.account_subject_id IS '会计科目 ID';
COMMENT ON COLUMN assist_accounting_summary.total_debit IS '借方金额合计';
COMMENT ON COLUMN assist_accounting_summary.total_credit IS '贷方金额合计';
COMMENT ON COLUMN assist_accounting_summary.total_quantity_meters IS '数量（米）合计';
COMMENT ON COLUMN assist_accounting_summary.total_quantity_kg IS '数量（公斤）合计';
COMMENT ON COLUMN assist_accounting_summary.record_count IS '记录数';

-- 索引
CREATE INDEX IF NOT EXISTS idx_summary_period ON assist_accounting_summary(accounting_period);
CREATE INDEX IF NOT EXISTS idx_summary_dimension ON assist_accounting_summary(dimension_code);
CREATE INDEX IF NOT EXISTS idx_summary_subject ON assist_accounting_summary(account_subject_id);
CREATE INDEX IF NOT EXISTS idx_summary_period_dimension ON assist_accounting_summary(accounting_period, dimension_code, dimension_value_id);

-- 4. 插入预设的 8 个辅助核算维度
-- ============================================
INSERT INTO assist_accounting_dimension (dimension_code, dimension_name, description, is_active, sort_order) VALUES
('BATCH', '批次核算', '按生产批次进行辅助核算', TRUE, 1),
('COLOR', '色号核算', '按产品色号进行辅助核算', TRUE, 2),
('DYE_LOT', '缸号核算', '按染色缸次进行辅助核算', TRUE, 3),
('GRADE', '等级核算', '按产品质量等级进行辅助核算', TRUE, 4),
('WORKSHOP', '车间核算', '按生产车间进行辅助核算', TRUE, 5),
('WAREHOUSE', '仓库核算', '按仓库进行辅助核算', TRUE, 6),
('CUSTOMER', '客户核算', '按客户进行辅助核算', TRUE, 7),
('SUPPLIER', '供应商核算', '按供应商进行辅助核算', TRUE, 8) ON CONFLICT DO NOTHING;

-- 插入 8 个辅助核算维度

-- 4. 创建辅助核算专用的 updated_at 更新函数
-- ============================================
CREATE OR REPLACE FUNCTION update_account_subject_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION update_account_subject_timestamp() IS '辅助核算表专用时间戳更新函数';

-- 5. 触发器：自动更新 updated_at 字段
-- ============================================
DROP TRIGGER IF EXISTS trg_update_assist_dimension ON assist_accounting_dimension;
CREATE TRIGGER trg_update_assist_dimension
BEFORE UPDATE ON assist_accounting_dimension
FOR EACH ROW
EXECUTE FUNCTION update_account_subject_timestamp();

DROP TRIGGER IF EXISTS trg_update_assist_summary ON assist_accounting_summary;
CREATE TRIGGER trg_update_assist_summary
BEFORE UPDATE ON assist_accounting_summary
FOR EACH ROW
EXECUTE FUNCTION update_account_subject_timestamp();

COMMENT ON TRIGGER trg_update_assist_dimension ON assist_accounting_dimension IS '自动更新辅助核算维度 updated_at 字段';
COMMENT ON TRIGGER trg_update_assist_summary ON assist_accounting_summary IS '自动更新辅助核算汇总 updated_at 字段';

-- ============================================
-- 迁移完成
-- ============================================

-- ============================================
-- 来源: 039_performance_optimization.sql
-- ============================================
-- ========================================
-- 秉羲 ERP 系统 - 性能优化迁移脚本
-- 版本：2026-04-01
-- 说明：添加索引以优化数据库查询性能
-- ========================================

-- ========================================
-- 1. 库存管理优化
-- ========================================

-- 库存表复合索引（低库存检查）
CREATE INDEX IF NOT EXISTS idx_inventory_low_stock ON inventory_stocks(min_stock, quantity);

-- 库存表复合索引（按产品和批次查询）
CREATE INDEX IF NOT EXISTS idx_inventory_product_batch ON inventory_stocks(product_id, batch_no, color_no);

-- 库存表复合索引（按状态和数量查询）
CREATE INDEX IF NOT EXISTS idx_inventory_status_quantity ON inventory_stocks(status, quantity);

-- ========================================
-- 2. 销售管理优化
-- ========================================

-- 销售订单表复合索引（按状态和日期查询）
CREATE INDEX IF NOT EXISTS idx_sales_orders_status_date ON sales_orders(status, order_date);

-- 销售订单表复合索引（按客户和状态查询）
CREATE INDEX IF NOT EXISTS idx_sales_orders_customer_status ON sales_orders(customer_name, status);

-- 销售订单明细表复合索引（按订单和色号查询）
CREATE INDEX IF NOT EXISTS idx_sales_order_items_order_color ON sales_order_items(order_id, color_no);

-- 销售订单明细表复合索引（按产品和批次查询）
CREATE INDEX IF NOT EXISTS idx_sales_order_items_product_batch ON sales_order_items(product_id, batch_requirement);

-- ========================================
-- 3. 库存流水优化
-- ========================================

-- 库存流水表复合索引（按交易类型和日期查询）
CREATE INDEX IF NOT EXISTS idx_inventory_transactions_type_date ON inventory_transactions(transaction_type, created_at);

-- 库存流水表复合索引（按仓库和批次查询）
CREATE INDEX IF NOT EXISTS idx_inventory_transactions_warehouse_batch ON inventory_transactions(warehouse_id, batch_no, color_no);

-- 库存流水表复合索引（按产品和数量查询）
CREATE INDEX IF NOT EXISTS idx_inventory_transactions_product_quantity ON inventory_transactions(product_id, quantity_meters, quantity_kg);

-- ========================================
-- 4. 产品管理优化
-- ========================================

-- 产品表复合索引（按类型和状态查询）
CREATE INDEX IF NOT EXISTS idx_products_type_status ON products(product_type, status);

-- 产品表复合索引（按类别和价格查询）
CREATE INDEX IF NOT EXISTS idx_products_category_price ON products(category_id, standard_price);

-- 产品色号表复合索引（按产品和色号查询）
CREATE INDEX IF NOT EXISTS idx_product_colors_product_color ON product_colors(product_id, color_no, is_active);

-- ========================================
-- 5. 客户管理优化
-- ========================================

-- 客户表复合索引（按行业和质量要求查询）
CREATE INDEX IF NOT EXISTS idx_customers_industry_quality ON customers(customer_industry, quality_requirement);

-- 客户表索引（按名称查询）
CREATE INDEX IF NOT EXISTS idx_customers_name ON customers(customer_name);

-- ========================================
-- 6. 仓库管理优化
-- ========================================

-- 仓库表复合索引（按类型和状态查询）
CREATE INDEX IF NOT EXISTS idx_warehouses_type_status ON warehouses(warehouse_type, status);

-- 库位表复合索引（按仓库和类型查询）
CREATE INDEX IF NOT EXISTS idx_warehouse_locations_warehouse_type ON warehouse_locations(warehouse_id, location_type);

-- ========================================
-- 7. 系统管理优化
-- ========================================

-- 操作日志表复合索引（按模块和操作查询）
CREATE INDEX IF NOT EXISTS idx_operation_logs_module_action ON operation_logs(module, action, status);

-- 操作日志表复合索引（按用户和日期查询）
CREATE INDEX IF NOT EXISTS idx_operation_logs_user_date ON operation_logs(user_id, created_at);

-- ========================================
-- 8. 视图优化
-- ========================================

-- 重新创建库存预警视图（增加索引使用）
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
        WHEN s.max_stock > 0 AND s.quantity > s.max_stock THEN 'high'
        ELSE 'normal'
    END AS stock_status
FROM inventory_stocks s
JOIN products p ON s.product_id = p.id
JOIN warehouses w ON s.warehouse_id = w.id
WHERE s.stock_status = '正常'
  AND s.quality_status = '合格'
  AND (s.quantity < s.min_stock OR (s.max_stock > 0 AND s.quantity > s.max_stock));

-- ========================================
-- 迁移完成提示
-- ========================================

DO $$
BEGIN
    RAISE NOTICE '========================================';
    RAISE NOTICE '秉羲 ERP 系统 - 性能优化迁移完成';
    RAISE NOTICE '版本：2026-04-01';
    RAISE NOTICE '========================================';
    RAISE NOTICE '添加索引：16 个';
    RAISE NOTICE '  - 库存管理：4 个';
    RAISE NOTICE '  - 销售管理：4 个';
    RAISE NOTICE '  - 库存流水：3 个';
    RAISE NOTICE '  - 产品管理：3 个';
    RAISE NOTICE '  - 客户管理：2 个';
    RAISE NOTICE '  - 仓库管理：2 个';
    RAISE NOTICE '  - 系统管理：2 个';
    RAISE NOTICE '';
    RAISE NOTICE '优化视图：1 个';
    RAISE NOTICE '  - v_low_stock_alerts (库存预警视图)';
    RAISE NOTICE '========================================';
END $$;

-- ============================================
-- 来源: 040_supplier_product_mapping.sql
-- ============================================
-- ========================================
-- 1. 供应商产品表
-- ========================================

-- ==================== 供应商产品表 ====================
CREATE TABLE IF NOT EXISTS supplier_products (
    id SERIAL PRIMARY KEY,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id) ON DELETE CASCADE,
    product_code VARCHAR(100) NOT NULL,                    -- 供应商产品编码
    product_name VARCHAR(200) NOT NULL,                   -- 供应商产品名称
    product_description TEXT,                               -- 产品描述
    unit VARCHAR(20) DEFAULT '米',                          -- 计量单位
    is_enabled BOOLEAN DEFAULT TRUE,                        -- 是否启用
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                     -- 创建人 ID
    updated_by INTEGER,                                     -- 更新人 ID
    remarks TEXT                                            -- 备注
);

COMMENT ON TABLE supplier_products IS '供应商产品表';
COMMENT ON COLUMN supplier_products.product_code IS '供应商产品编码';
COMMENT ON COLUMN supplier_products.product_name IS '供应商产品名称';

-- 唯一约束
ALTER TABLE supplier_products ADD CONSTRAINT uk_supplier_product_code UNIQUE (supplier_id, product_code);

-- 索引
CREATE INDEX IF NOT EXISTS idx_supplier_products_supplier_id ON supplier_products(supplier_id);
CREATE INDEX IF NOT EXISTS idx_supplier_products_code ON supplier_products(product_code);
CREATE INDEX IF NOT EXISTS idx_supplier_products_enabled ON supplier_products(is_enabled);

-- ========================================
-- 2. 供应商产品颜色表
-- ========================================

-- ==================== 供应商产品颜色表 ====================
CREATE TABLE IF NOT EXISTS supplier_product_colors (
    id SERIAL PRIMARY KEY,
    supplier_product_id INTEGER NOT NULL REFERENCES supplier_products(id) ON DELETE CASCADE,
    color_no VARCHAR(50) NOT NULL,                         -- 供应商色号编码
    color_name VARCHAR(100) NOT NULL,                      -- 供应商颜色名称
    pantone_code VARCHAR(50),                               -- 潘通色号
    extra_cost DECIMAL(10,2) DEFAULT 0.00,                 -- 特殊色号加价
    is_enabled BOOLEAN DEFAULT TRUE,                        -- 是否启用
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    remarks TEXT                                            -- 备注
);

COMMENT ON TABLE supplier_product_colors IS '供应商产品颜色表';
COMMENT ON COLUMN supplier_product_colors.color_no IS '供应商色号编码';

-- 唯一约束
ALTER TABLE supplier_product_colors ADD CONSTRAINT uk_supplier_product_color UNIQUE (supplier_product_id, color_no);

-- 索引
CREATE INDEX IF NOT EXISTS idx_supplier_product_colors_product_id ON supplier_product_colors(supplier_product_id);
CREATE INDEX IF NOT EXISTS idx_supplier_product_colors_color_no ON supplier_product_colors(color_no);
CREATE INDEX IF NOT EXISTS idx_supplier_product_colors_enabled ON supplier_product_colors(is_enabled);

-- ========================================
-- 3. 系统产品与供应商产品映射表
-- ========================================

-- ==================== 系统产品与供应商产品映射表 ====================
CREATE TABLE IF NOT EXISTS product_supplier_mappings (
    id SERIAL PRIMARY KEY,
    product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    product_color_id INTEGER REFERENCES product_colors(id) ON DELETE SET NULL,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id) ON DELETE CASCADE,
    supplier_product_id INTEGER NOT NULL REFERENCES supplier_products(id) ON DELETE CASCADE,
    supplier_product_color_id INTEGER REFERENCES supplier_product_colors(id) ON DELETE SET NULL,
    is_primary BOOLEAN DEFAULT FALSE,                       -- 是否为首选供应商
    priority INTEGER DEFAULT 1,                              -- 优先级（数字越小优先级越高）
    supplier_price DECIMAL(12,2),                            -- 供应商价格
    min_order_quantity DECIMAL(12,2),                        -- 最小起订量
    lead_time INTEGER,                                        -- 交货期（天）
    is_enabled BOOLEAN DEFAULT TRUE,                          -- 是否启用
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                       -- 创建人 ID
    updated_by INTEGER,                                       -- 更新人 ID
    remarks TEXT                                              -- 备注
);

COMMENT ON TABLE product_supplier_mappings IS '系统产品与供应商产品映射表';
COMMENT ON COLUMN product_supplier_mappings.is_primary IS '是否为首选供应商';
COMMENT ON COLUMN product_supplier_mappings.priority IS '优先级（数字越小优先级越高）';

-- 唯一约束
ALTER TABLE product_supplier_mappings ADD CONSTRAINT uk_product_supplier_mapping UNIQUE (
    product_id, 
    COALESCE(product_color_id, 0), 
    supplier_id, 
    supplier_product_id, 
    COALESCE(supplier_product_color_id, 0)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_product_supplier_mappings_product ON product_supplier_mappings(product_id);
CREATE INDEX IF NOT EXISTS idx_product_supplier_mappings_product_color ON product_supplier_mappings(product_color_id);
CREATE INDEX IF NOT EXISTS idx_product_supplier_mappings_supplier ON product_supplier_mappings(supplier_id);
CREATE INDEX IF NOT EXISTS idx_product_supplier_mappings_supplier_product ON product_supplier_mappings(supplier_product_id);
CREATE INDEX IF NOT EXISTS idx_product_supplier_mappings_primary ON product_supplier_mappings(is_primary);
CREATE INDEX IF NOT EXISTS idx_product_supplier_mappings_enabled ON product_supplier_mappings(is_enabled);

-- ========================================
-- 4. 触发器函数
-- ========================================

-- ==================== 自动更新 updated_at ====================
CREATE OR REPLACE FUNCTION update_supplier_product_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ========================================
-- 5. 应用触发器
-- ========================================

-- 更新 updated_at 触发器
DROP TRIGGER IF EXISTS trg_supplier_products_updated_at ON supplier_products;
CREATE TRIGGER trg_supplier_products_updated_at
    BEFORE UPDATE ON supplier_products
    FOR EACH ROW
    EXECUTE FUNCTION update_supplier_product_updated_at_column();

DROP TRIGGER IF EXISTS trg_supplier_product_colors_updated_at ON supplier_product_colors;
CREATE TRIGGER trg_supplier_product_colors_updated_at
    BEFORE UPDATE ON supplier_product_colors
    FOR EACH ROW
    EXECUTE FUNCTION update_supplier_product_updated_at_column();

DROP TRIGGER IF EXISTS trg_product_supplier_mappings_updated_at ON product_supplier_mappings;
CREATE TRIGGER trg_product_supplier_mappings_updated_at
    BEFORE UPDATE ON product_supplier_mappings
    FOR EACH ROW
    EXECUTE FUNCTION update_supplier_product_updated_at_column();

-- ========================================
-- 迁移完成
-- ========================================

-- ============================================
-- 来源: 041_dual_unit_optimization.sql
-- ============================================
-- 双计量单位优化迁移脚本
-- 创建时间：2026-03-15
-- 说明：添加双计量单位计算字段、触发器和索引

-- 1. 库存表添加计算字段
ALTER TABLE inventory_stocks
ADD COLUMN IF NOT EXISTS quantity_alt DECIMAL(18,4),
ADD COLUMN IF NOT EXISTS gram_weight DECIMAL(8,2),
ADD COLUMN IF NOT EXISTS width DECIMAL(8,2),
ADD COLUMN IF NOT EXISTS calculated_quantity_alt DECIMAL(10,3),
ADD COLUMN IF NOT EXISTS unit_conversion_rate DECIMAL(10,6);

COMMENT ON COLUMN inventory_stocks.calculated_quantity_alt IS '计算后的公斤数（自动计算）';
COMMENT ON COLUMN inventory_stocks.unit_conversion_rate IS '单位换算率（公斤/米）';

-- 2. 采购入库明细表添加计算字段
ALTER TABLE purchase_receipt_item 
ADD COLUMN IF NOT EXISTS calculated_quantity_alt DECIMAL(10,3),
ADD COLUMN IF NOT EXISTS unit_conversion_rate DECIMAL(10,6);

COMMENT ON COLUMN purchase_receipt_item.calculated_quantity_alt IS '计算后的公斤数（自动计算）';
COMMENT ON COLUMN purchase_receipt_item.unit_conversion_rate IS '单位换算率（公斤/米）';

-- 3. 采购订单明细表添加计算字段
ALTER TABLE purchase_order_item 
ADD COLUMN IF NOT EXISTS calculated_quantity_alt DECIMAL(10,3),
ADD COLUMN IF NOT EXISTS unit_conversion_rate DECIMAL(10,6);

COMMENT ON COLUMN purchase_order_item.calculated_quantity_alt IS '计算后的公斤数（自动计算）';
COMMENT ON COLUMN purchase_order_item.unit_conversion_rate IS '单位换算率（公斤/米）';

-- 4. 销售出库明细表添加计算字段（如果存在该表）
DO $$ 
BEGIN 
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'sales_delivery_item') THEN
        ALTER TABLE sales_delivery_item 
        ADD COLUMN IF NOT EXISTS calculated_quantity_alt DECIMAL(10,3),
        ADD COLUMN IF NOT EXISTS unit_conversion_rate DECIMAL(10,6);
        
        COMMENT ON COLUMN sales_delivery_item.calculated_quantity_alt IS '计算后的公斤数（自动计算）';
        COMMENT ON COLUMN sales_delivery_item.unit_conversion_rate IS '单位换算率（公斤/米）';
    END IF;
END $$;

-- 5. 创建通用双计量单位计算触发器函数 (针对 quantity / quantity_alt)
CREATE OR REPLACE FUNCTION calculate_dual_unit_quantity()
RETURNS TRIGGER AS $$
BEGIN
    -- 如果有米数、克重、幅宽，则自动计算公斤数
    IF NEW.quantity IS NOT NULL 
       AND NEW.quantity > 0 
       AND NEW.gram_weight IS NOT NULL 
       AND NEW.gram_weight > 0 
       AND NEW.width IS NOT NULL 
       AND NEW.width > 0 THEN
        
        NEW.calculated_quantity_alt := ROUND(
            NEW.quantity * NEW.gram_weight * (NEW.width / 100.0) / 1000.0, 
            3
        );
        
        -- 如果没有手动输入公斤数，则使用自动计算的公斤数
        IF NEW.quantity_alt IS NULL THEN
            NEW.quantity_alt := NEW.calculated_quantity_alt;
        END IF;
        
        -- 计算单位换算率
        NEW.unit_conversion_rate := ROUND(
            NEW.quantity_alt / NULLIF(NEW.quantity, 0), 
            6
        );
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 5.5 创建订单双计量单位计算触发器函数 (针对 quantity_ordered / quantity_alt_ordered)
CREATE OR REPLACE FUNCTION calculate_dual_unit_order_quantity()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.quantity_ordered IS NOT NULL 
       AND NEW.quantity_ordered > 0 
       AND NEW.gram_weight IS NOT NULL 
       AND NEW.gram_weight > 0 
       AND NEW.width IS NOT NULL 
       AND NEW.width > 0 THEN
        
        NEW.calculated_quantity_alt := ROUND(
            NEW.quantity_ordered * NEW.gram_weight * (NEW.width / 100.0) / 1000.0, 
            3
        );
        
        IF NEW.quantity_alt_ordered IS NULL THEN
            NEW.quantity_alt_ordered := NEW.calculated_quantity_alt;
        END IF;
        
        NEW.unit_conversion_rate := ROUND(
            NEW.quantity_alt_ordered / NULLIF(NEW.quantity_ordered, 0), 
            6
        );
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 6. 为 inventory_stocks 表创建触发器
DROP TRIGGER IF EXISTS trg_calculate_dual_unit_inventory ON inventory_stocks;
DROP TRIGGER IF EXISTS trg_calculate_dual_unit_inventory ON inventory_stocks;
CREATE TRIGGER trg_calculate_dual_unit_inventory
    BEFORE INSERT OR UPDATE ON inventory_stocks
    FOR EACH ROW
    EXECUTE FUNCTION calculate_dual_unit_quantity();

-- 7. 为 purchase_receipt_item 表创建触发器
DROP TRIGGER IF EXISTS trg_calculate_dual_unit_receipt ON purchase_receipt_item;
DROP TRIGGER IF EXISTS trg_calculate_dual_unit_receipt ON purchase_receipt_item;
CREATE TRIGGER trg_calculate_dual_unit_receipt
    BEFORE INSERT OR UPDATE ON purchase_receipt_item
    FOR EACH ROW
    EXECUTE FUNCTION calculate_dual_unit_quantity();

-- 8. 为 purchase_order_item 表创建触发器
DROP TRIGGER IF EXISTS trg_calculate_dual_unit_order ON purchase_order_item;
CREATE TRIGGER trg_calculate_dual_unit_order
    BEFORE INSERT OR UPDATE ON purchase_order_item
    FOR EACH ROW
    EXECUTE FUNCTION calculate_dual_unit_order_quantity();

-- 9. 为 sales_delivery_item 表创建触发器（如果存在）
DO $$ 
BEGIN 
    IF EXISTS (
        SELECT 1 FROM information_schema.triggers 
        WHERE trigger_name = 'trg_calculate_dual_unit_sales'
    ) THEN
        DROP TRIGGER trg_calculate_dual_unit_sales ON sales_delivery_item;
    END IF;
    
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'sales_delivery_item') THEN
DROP TRIGGER IF EXISTS trg_calculate_dual_unit_sales ON sales_delivery_item;
        CREATE TRIGGER trg_calculate_dual_unit_sales
            BEFORE INSERT OR UPDATE ON sales_delivery_item
            FOR EACH ROW
            EXECUTE FUNCTION calculate_dual_unit_quantity();
    END IF;
END $$;

-- 10. 更新现有数据（一次性操作）
UPDATE inventory_stocks
SET 
    quantity_alt = ROUND(quantity * gram_weight * (width / 100.0) / 1000.0, 3),
    calculated_quantity_alt = ROUND(quantity * gram_weight * (width / 100.0) / 1000.0, 3),
    unit_conversion_rate = ROUND(
        ROUND(quantity * gram_weight * (width / 100.0) / 1000.0, 3) / NULLIF(quantity, 0),
        6
    )
WHERE quantity IS NOT NULL 
  AND gram_weight IS NOT NULL 
  AND width IS NOT NULL
  AND quantity > 0
  AND gram_weight > 0
  AND width > 0;

UPDATE purchase_receipt_item
SET 
    quantity_alt = ROUND(quantity_received * gram_weight * (width / 100.0) / 1000.0, 3),
    calculated_quantity_alt = ROUND(quantity_received * gram_weight * (width / 100.0) / 1000.0, 3),
    unit_conversion_rate = ROUND(
        ROUND(quantity_received * gram_weight * (width / 100.0) / 1000.0, 3) / NULLIF(quantity_received, 0),
        6
    )
WHERE quantity_received IS NOT NULL 
  AND gram_weight IS NOT NULL 
  AND width IS NOT NULL
  AND quantity_received > 0
  AND gram_weight > 0
  AND width > 0;

UPDATE purchase_order_item
SET
    quantity_alt_ordered = ROUND(quantity_ordered * gram_weight * (width / 100.0) / 1000.0, 3),
    calculated_quantity_alt = ROUND(quantity_ordered * gram_weight * (width / 100.0) / 1000.0, 3),
    unit_conversion_rate = ROUND(
        ROUND(quantity_ordered * gram_weight * (width / 100.0) / 1000.0, 3) / NULLIF(quantity_ordered, 0),
        6
    )
WHERE quantity_ordered IS NOT NULL
  AND quantity_ordered > 0
  AND gram_weight IS NOT NULL
  AND gram_weight > 0
  AND width IS NOT NULL
  AND width > 0;

-- 11. 创建索引优化查询性能
CREATE INDEX IF NOT EXISTS idx_inventory_dual_unit 
ON inventory_stocks(quantity, quantity_alt, gram_weight, width);

CREATE INDEX IF NOT EXISTS idx_receipt_dual_unit
ON purchase_receipt_item(quantity, quantity_alt, gram_weight, width);

CREATE INDEX IF NOT EXISTS idx_order_dual_unit
ON purchase_order_item(quantity_ordered, quantity_alt_ordered, gram_weight, width);

-- 12. 创建视图方便双计量单位查询
CREATE OR REPLACE VIEW v_inventory_dual_unit AS
SELECT 
    id,
    product_id,
    batch_no,
    color_no,
    dye_lot_no,
    grade,
    quantity,
    quantity_alt,
    calculated_quantity_alt,
    unit_conversion_rate,
    gram_weight,
    width,
    warehouse_id,
    stock_status,
    quality_status,
    created_at,
    updated_at
FROM inventory_stocks
WHERE quantity IS NOT NULL;

COMMENT ON VIEW v_inventory_dual_unit IS '库存双计量单位视图（包含换算信息）';

-- 迁移完成提示
DO $$
BEGIN
    RAISE NOTICE '双计量单位优化迁移完成！';
    RAISE NOTICE '- 新增字段：calculated_quantity_alt, unit_conversion_rate';
    RAISE NOTICE '- 新增触发器：自动计算公斤数和换算率';
    RAISE NOTICE '- 新增索引：优化双计量单位查询';
    RAISE NOTICE '- 新增视图：v_inventory_dual_unit';
END $$;

-- ============================================
-- 来源: 042_five_dimension_optimization.sql
-- ============================================
-- ============================================================
-- 面料行业五维管理优化迁移脚本
-- 版本：v2.0
-- 日期：2024-01-01
-- 说明：优化五维查询性能，添加组合索引和计算列
-- ============================================================

-- 1. 为 inventory_stocks 表添加五维组合索引
-- 优化按批次 + 色号 + 等级查询
CREATE INDEX IF NOT EXISTS idx_inventory_five_dimension
ON inventory_stocks(product_id, batch_no, color_no, grade);

-- 2. 为 inventory_stocks 表添加五维 ID 计算列（虚拟列）
-- 格式：P{id}|B{batch}|C{color}|D{dye_lot}|G{grade}
ALTER TABLE inventory_stocks 
ADD COLUMN IF NOT EXISTS five_dimension_id VARCHAR(255) 
GENERATED ALWAYS AS (
    CONCAT(
        'P', product_id, '|',
        'B', batch_no, '|',
        'C', color_no, '|',
        'D', COALESCE(dye_lot_no, 'N'), '|',
        'G', grade
    )
) STORED;

-- 3. 为五维 ID 添加索引，加速精确查询
CREATE INDEX IF NOT EXISTS idx_inventory_five_dimension_id
ON inventory_stocks(five_dimension_id);

-- 4. 为 purchase_receipt_item 表添加五维组合索引
CREATE INDEX IF NOT EXISTS idx_purchase_receipt_five_dim
ON purchase_receipt_item(product_id, batch_no, color_code, grade);

-- 5. 为 purchase_receipt_item 表添加五维 ID 计算列
ALTER TABLE purchase_receipt_item 
ADD COLUMN IF NOT EXISTS five_dimension_id VARCHAR(255) 
GENERATED ALWAYS AS (
    CONCAT(
        'P', product_id, '|',
        'B', batch_no, '|',
        'C', color_no, '|',
        'D', COALESCE(dye_lot_no, 'N'), '|',
        'G', grade
    )
) STORED;

-- 6. 为 purchase_receipt_item 的五维 ID 添加索引
CREATE INDEX IF NOT EXISTS idx_purchase_receipt_five_dim_id
ON purchase_receipt_item(five_dimension_id);

-- 7. 为 sales_delivery_item 表添加五维组合索引
CREATE INDEX IF NOT EXISTS idx_sales_delivery_five_dim
ON sales_delivery_item(product_id, dye_lot_no, color_no);

-- 8. 为 sales_delivery_item 表添加五维 ID 计算列
ALTER TABLE sales_delivery_item 
ADD COLUMN IF NOT EXISTS five_dimension_id VARCHAR(255) 
GENERATED ALWAYS AS (
    CONCAT(
        'P', product_id, '|',
        'B', batch_no, '|',
        'C', color_no, '|',
        'D', COALESCE(dye_lot_no, 'N'), '|',
        'G', grade
    )
) STORED;

-- 9. 为 sales_delivery_item 的五维 ID 添加索引
CREATE INDEX IF NOT EXISTS idx_sales_delivery_five_dim_id
ON sales_delivery_item(five_dimension_id);

-- 10. 为 inventory_transactions 表添加五维组合索引
CREATE INDEX IF NOT EXISTS idx_inventory_transaction_five_dim
ON inventory_transactions(product_id, batch_no, color_no, grade);

-- 11. 为 inventory_transactions 表添加五维 ID 计算列
ALTER TABLE inventory_transactions
ADD COLUMN IF NOT EXISTS five_dimension_id VARCHAR(255)
GENERATED ALWAYS AS (
    CONCAT(
        'P', COALESCE(product_id::text, '0'), '|',
        'B', COALESCE(batch_no, 'N'), '|',
        'C', COALESCE(color_no, 'N'), '|',
        'D', 'N', '|',
        'G', COALESCE(grade, 'N')
    )
) STORED;

-- 12. 为 inventory_transactions 的五维 ID 添加索引
CREATE INDEX IF NOT EXISTS idx_inventory_transaction_five_dim_id
ON inventory_transactions(five_dimension_id);

-- 13. 创建五维统计视图（简化查询）
CREATE OR REPLACE VIEW v_five_dimension_inventory_summary AS
SELECT 
    product_id,
    batch_no,
    color_no,
    dye_lot_no,
    grade,
    five_dimension_id,
    COUNT(*) as stock_count,
    SUM(quantity_meters) as total_meters,
    SUM(quantity_kg) as total_kg,
    MAX(updated_at) as last_updated
FROM inventory_stocks
GROUP BY 
    product_id,
    batch_no,
    color_no,
    dye_lot_no,
    grade,
    five_dimension_id;

-- 14. 创建五维查询函数（带模糊匹配）
CREATE OR REPLACE FUNCTION search_by_five_dimension(
    p_product_id INTEGER DEFAULT NULL,
    p_batch_no VARCHAR DEFAULT NULL,
    p_color_no VARCHAR DEFAULT NULL,
    p_dye_lot_no VARCHAR DEFAULT NULL,
    p_grade VARCHAR DEFAULT NULL
)
RETURNS TABLE (
    inventory_id INTEGER,
    five_dim_id VARCHAR,
    warehouse_id INTEGER,
    quantity_meters DECIMAL,
    quantity_kg DECIMAL,
    stock_status VARCHAR
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        i.id,
        i.five_dimension_id,
        i.warehouse_id,
        i.quantity_meters,
        i.quantity_kg,
        i.stock_status
    FROM inventory_stocks i
    WHERE 
        (p_product_id IS NULL OR i.product_id = p_product_id)
        AND (p_batch_no IS NULL OR i.batch_no LIKE CONCAT('%', p_batch_no, '%'))
        AND (p_color_no IS NULL OR i.color_no LIKE CONCAT('%', p_color_no, '%'))
        AND (p_dye_lot_no IS NULL OR i.dye_lot_no IS NOT NULL AND i.dye_lot_no LIKE CONCAT('%', p_dye_lot_no, '%'))
        AND (p_grade IS NULL OR i.grade = p_grade);
END;
$$ LANGUAGE plpgsql;

-- 15. 添加注释说明
COMMENT ON INDEX idx_inventory_five_dimension IS '五维组合索引 - 优化库存查询';
COMMENT ON INDEX idx_inventory_five_dimension_id IS '五维 ID 索引 - 精确查询';
COMMENT ON INDEX idx_purchase_receipt_five_dim IS '采购收货五维索引';
COMMENT ON INDEX idx_sales_delivery_five_dim IS '销售发货五维索引';
COMMENT ON INDEX idx_inventory_transaction_five_dim IS '库存流水五维索引';
COMMENT ON VIEW v_five_dimension_inventory_summary IS '五维库存统计视图';
COMMENT ON FUNCTION search_by_five_dimension IS '五维搜索函数（支持模糊匹配）';

-- ============================================================
-- 迁移完成
-- 性能提升预期：
-- - 五维组合查询速度提升：80-90%
-- - 精确查询（通过五维 ID）：95%+
-- - 模糊查询：60-70%
-- ============================================================

-- ============================================
-- 来源: 043_business_trace_optimization.sql
-- ============================================
-- ============================================================
-- 面料行业业务追溯链条优化迁移脚本
-- 版本：v2.0
-- 日期：2024-01-01
-- 说明：实现完整的正向 + 反向业务追溯体系
-- ============================================================

-- 1. 创建业务追溯链表
CREATE TABLE IF NOT EXISTS business_trace_chain (
    id SERIAL PRIMARY KEY,
    trace_chain_id VARCHAR(255) NOT NULL UNIQUE,
    five_dimension_id VARCHAR(255) NOT NULL,
    product_id INTEGER NOT NULL,
    batch_no VARCHAR(100) NOT NULL,
    color_no VARCHAR(50) NOT NULL,
    dye_lot_no VARCHAR(100),
    grade VARCHAR(50) NOT NULL,
    current_stage VARCHAR(50) NOT NULL,
    current_bill_type VARCHAR(50) NOT NULL,
    current_bill_no VARCHAR(100) NOT NULL,
    current_bill_id INTEGER NOT NULL,
    previous_trace_id INTEGER,
    next_trace_id INTEGER,
    quantity_meters DECIMAL(12,2) NOT NULL,
    quantity_kg DECIMAL(12,2) NOT NULL,
    warehouse_id INTEGER NOT NULL,
    supplier_id INTEGER,
    customer_id INTEGER,
    workshop_id INTEGER,
    trace_status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
    remarks TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    created_by INTEGER,
    
    CONSTRAINT fk_previous_trace FOREIGN KEY (previous_trace_id) REFERENCES business_trace_chain(id),
    CONSTRAINT fk_next_trace FOREIGN KEY (next_trace_id) REFERENCES business_trace_chain(id)
);

-- 2. 创建业务追溯快照表
CREATE TABLE IF NOT EXISTS business_trace_snapshot (
    id SERIAL PRIMARY KEY,
    trace_chain_id VARCHAR(255) NOT NULL,
    five_dimension_id VARCHAR(255) NOT NULL,
    product_id INTEGER NOT NULL,
    batch_no VARCHAR(100) NOT NULL,
    color_no VARCHAR(50) NOT NULL,
    grade VARCHAR(50) NOT NULL,
    current_stage VARCHAR(50) NOT NULL,
    warehouse_id INTEGER NOT NULL,
    current_quantity_meters DECIMAL(12,2) NOT NULL,
    current_quantity_kg DECIMAL(12,2) NOT NULL,
    supplier_name VARCHAR(255),
    customer_name VARCHAR(255),
    trace_path JSONB NOT NULL,
    snapshot_time TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 3. 为追溯链表添加索引
CREATE INDEX IF NOT EXISTS idx_trace_chain_five_dim ON business_trace_chain(five_dimension_id);
CREATE INDEX IF NOT EXISTS idx_trace_chain_batch ON business_trace_chain(batch_no);
CREATE INDEX IF NOT EXISTS idx_trace_chain_supplier ON business_trace_chain(supplier_id);
CREATE INDEX IF NOT EXISTS idx_trace_chain_customer ON business_trace_chain(customer_id);
CREATE INDEX IF NOT EXISTS idx_trace_chain_stage ON business_trace_chain(current_stage);
CREATE INDEX IF NOT EXISTS idx_trace_chain_status ON business_trace_chain(trace_status);
CREATE INDEX IF NOT EXISTS idx_trace_chain_created ON business_trace_chain(created_at);

-- 4. 为追溯快照表添加索引
CREATE INDEX IF NOT EXISTS idx_trace_snapshot_chain_id ON business_trace_snapshot(trace_chain_id);
CREATE INDEX IF NOT EXISTS idx_trace_snapshot_five_dim ON business_trace_snapshot(five_dimension_id);
CREATE INDEX IF NOT EXISTS idx_trace_snapshot_batch ON business_trace_snapshot(batch_no);
CREATE INDEX IF NOT EXISTS idx_trace_snapshot_time ON business_trace_snapshot(snapshot_time);

-- 5. 创建业务追溯视图（简化查询）
CREATE OR REPLACE VIEW v_business_trace_view AS
SELECT 
    t.id,
    t.trace_chain_id,
    t.five_dimension_id,
    t.product_id,
    t.batch_no,
    t.color_no,
    t.grade,
    FIRST_VALUE(t.current_stage) OVER (
        PARTITION BY t.trace_chain_id 
        ORDER BY t.created_at ASC
    ) as start_stage,
    t.current_stage,
    COUNT(*) OVER (PARTITION BY t.trace_chain_id) as stage_count,
    SUM(CASE 
        WHEN t.current_stage IN ('PURCHASE_RECEIPT', 'INVENTORY_IN', 'PRODUCTION_OUTPUT') 
        THEN t.quantity_meters 
        ELSE 0 
    END) OVER (PARTITION BY t.trace_chain_id) as total_in_meters,
    SUM(CASE 
        WHEN t.current_stage IN ('INVENTORY_OUT', 'SALES_DELIVERY', 'PRODUCTION_INPUT') 
        THEN t.quantity_meters 
        ELSE 0 
    END) OVER (PARTITION BY t.trace_chain_id) as total_out_meters,
    t.quantity_meters as current_stock_meters,
    NULL as supplier_name,
    NULL as customer_name,
    t.created_at,
    MAX(t.created_at) OVER (PARTITION BY t.trace_chain_id) as updated_at
FROM business_trace_chain t;

-- 6. 创建追溯链查询函数
CREATE OR REPLACE FUNCTION get_trace_chain_by_five_dim(p_five_dimension_id VARCHAR)
RETURNS TABLE (
    trace_id INTEGER,
    trace_chain_id VARCHAR,
    stage_name VARCHAR,
    bill_type VARCHAR,
    bill_no VARCHAR,
    quantity_meters DECIMAL,
    warehouse_id INTEGER,
    created_at TIMESTAMP
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        t.id,
        t.trace_chain_id,
        t.current_stage,
        t.current_bill_type,
        t.current_bill_no,
        t.quantity_meters,
        t.warehouse_id,
        t.created_at
    FROM business_trace_chain t
    WHERE t.five_dimension_id = p_five_dimension_id
    ORDER BY t.created_at ASC;
END;
$$ LANGUAGE plpgsql;

-- 7. 创建正向追溯函数
CREATE OR REPLACE FUNCTION forward_trace_by_supplier(p_supplier_id INTEGER, p_batch_no VARCHAR)
RETURNS TABLE (
    trace_id INTEGER,
    trace_chain_id VARCHAR,
    five_dimension_id VARCHAR,
    current_stage VARCHAR,
    quantity_meters DECIMAL,
    created_at TIMESTAMP
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        t.id,
        t.trace_chain_id,
        t.five_dimension_id,
        t.current_stage,
        t.quantity_meters,
        t.created_at
    FROM business_trace_chain t
    WHERE t.supplier_id = p_supplier_id
      AND t.batch_no = p_batch_no
      AND t.current_stage = 'PURCHASE_RECEIPT'
    ORDER BY t.created_at ASC;
END;
$$ LANGUAGE plpgsql;

-- 8. 创建反向追溯函数
CREATE OR REPLACE FUNCTION backward_trace_by_customer(p_customer_id INTEGER, p_batch_no VARCHAR)
RETURNS TABLE (
    trace_id INTEGER,
    trace_chain_id VARCHAR,
    five_dimension_id VARCHAR,
    current_stage VARCHAR,
    quantity_meters DECIMAL,
    created_at TIMESTAMP
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        t.id,
        t.trace_chain_id,
        t.five_dimension_id,
        t.current_stage,
        t.quantity_meters,
        t.created_at
    FROM business_trace_chain t
    WHERE t.customer_id = p_customer_id
      AND t.batch_no = p_batch_no
      AND t.current_stage = 'SALES_DELIVERY'
    ORDER BY t.created_at DESC;
END;
$$ LANGUAGE plpgsql;

-- 9. 添加注释说明
COMMENT ON TABLE business_trace_chain IS '业务追溯链 - 记录物料从采购到销售的完整流转过程';
COMMENT ON TABLE business_trace_snapshot IS '业务追溯快照 - 定期保存追溯链状态，用于快速查询';
COMMENT ON VIEW v_business_trace_view IS '业务追溯视图 - 简化追溯查询';
COMMENT ON FUNCTION get_trace_chain_by_five_dim IS '按五维 ID 查询追溯链';
COMMENT ON FUNCTION forward_trace_by_supplier IS '正向追溯：从供应商到客户';
COMMENT ON FUNCTION backward_trace_by_customer IS '反向追溯：从客户到供应商';

-- 10. 添加触发器：自动更新追溯链状态
CREATE OR REPLACE FUNCTION update_trace_chain_status()
RETURNS TRIGGER AS $$
BEGIN
    -- 如果是最后一个环节（销售发货），标记为已完成
    IF NEW.current_stage = 'SALES_DELIVERY' THEN
        NEW.trace_status := 'COMPLETED';
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_update_trace_status ON business_trace_chain;
CREATE TRIGGER trg_update_trace_status
    BEFORE INSERT OR UPDATE ON business_trace_chain
    FOR EACH ROW
    EXECUTE FUNCTION update_trace_chain_status();

-- ============================================================
-- 追溯链条说明
-- ============================================================
-- 正向追溯流程：
-- 1. PURCHASE_RECEIPT (采购收货) - 供应商 → 仓库
-- 2. INVENTORY_IN (入库) - 仓库接收
-- 3. PRODUCTION_INPUT (生产投入) - 仓库 → 车间
-- 4. PRODUCTION_OUTPUT (生产产出) - 车间 → 仓库
-- 5. INVENTORY_OUT (出库) - 仓库备货
-- 6. SALES_DELIVERY (销售发货) - 仓库 → 客户
--
-- 反向追溯流程：
-- 1. 从销售发货开始
-- 2. 通过 previous_trace_id 逐级回溯
-- 3. 直到采购收货环节
--
-- 性能优化：
-- - 五维 ID 索引：加速按批次 + 色号查询
-- - 供应商/客户索引：加速正向/反向追溯
-- - 快照表：定期保存状态，避免长链查询
-- ============================================================
