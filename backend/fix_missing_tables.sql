-- 修复缺失的表和列
-- 日期: 2026-05-22

-- 1. 修复 sales_orders 表 - 添加 required_date 列
ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS required_date TIMESTAMP WITH TIME ZONE;
UPDATE sales_orders SET required_date = delivery_date WHERE required_date IS NULL;

-- 2. 创建 dye_recipe 表
CREATE TABLE IF NOT EXISTS dye_recipe (
    id SERIAL PRIMARY KEY,
    recipe_no VARCHAR(50) NOT NULL UNIQUE,
    color_code VARCHAR(50) NOT NULL,
    color_name VARCHAR(100) NOT NULL,
    fabric_type VARCHAR(100),
    dye_type VARCHAR(100),
    chemical_formula TEXT,
    temperature DECIMAL(10,2),
    time_minutes INTEGER,
    ph_value DECIMAL(5,2),
    liquor_ratio DECIMAL(10,2),
    auxiliaries JSONB,
    status VARCHAR(20) NOT NULL DEFAULT 'draft',
    version INTEGER DEFAULT 1,
    parent_recipe_id INTEGER REFERENCES dye_recipe(id),
    approved_by INTEGER REFERENCES users(id),
    approved_at TIMESTAMP WITH TIME ZONE,
    remarks TEXT,
    created_by INTEGER REFERENCES users(id),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 3. 修复 purchase_orders 表 - 添加 warehouse_id 列
ALTER TABLE purchase_orders ADD COLUMN IF NOT EXISTS warehouse_id INTEGER REFERENCES warehouses(id);

-- 4. 创建 ap_payment 表
CREATE TABLE IF NOT EXISTS ap_payment (
    id SERIAL PRIMARY KEY,
    payment_no VARCHAR(50) NOT NULL UNIQUE,
    payment_date DATE NOT NULL,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id),
    request_id INTEGER,
    payment_method VARCHAR(20) NOT NULL,
    payment_amount DECIMAL(18,2) NOT NULL,
    payment_status VARCHAR(20) NOT NULL DEFAULT 'REGISTERED',
    currency VARCHAR(10) NOT NULL DEFAULT 'CNY',
    exchange_rate DECIMAL(18,6) NOT NULL DEFAULT 1.000000,
    payment_amount_foreign DECIMAL(18,2),
    bank_name VARCHAR(200),
    bank_account VARCHAR(50),
    transaction_no VARCHAR(100),
    notes TEXT,
    attachment_urls TEXT[],
    created_by INTEGER NOT NULL REFERENCES users(id),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_by INTEGER REFERENCES users(id),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    confirmed_by INTEGER REFERENCES users(id),
    confirmed_at TIMESTAMP WITH TIME ZONE
);

-- 5. 创建 ap_verification 表
CREATE TABLE IF NOT EXISTS ap_verification (
    id SERIAL PRIMARY KEY,
    verification_no VARCHAR(50) NOT NULL UNIQUE,
    verification_date DATE NOT NULL,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id),
    verification_type VARCHAR(20) NOT NULL,
    total_amount DECIMAL(18,2) NOT NULL,
    verification_status VARCHAR(20) NOT NULL DEFAULT 'COMPLETED',
    notes TEXT,
    created_by INTEGER NOT NULL REFERENCES users(id),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    cancelled_by INTEGER REFERENCES users(id),
    cancelled_at TIMESTAMP WITH TIME ZONE,
    cancelled_reason TEXT
);

-- 6. 创建 ap_verification_item 表
CREATE TABLE IF NOT EXISTS ap_verification_item (
    id SERIAL PRIMARY KEY,
    verification_id INTEGER NOT NULL REFERENCES ap_verification(id),
    invoice_id INTEGER NOT NULL REFERENCES ap_invoices(id),
    payment_id INTEGER NOT NULL REFERENCES ap_payment(id),
    amount DECIMAL(18,2) NOT NULL,
    notes TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 7. 创建 ap_reconciliation 表
CREATE TABLE IF NOT EXISTS ap_reconciliation (
    id SERIAL PRIMARY KEY,
    reconciliation_no VARCHAR(50) NOT NULL UNIQUE,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id),
    start_date DATE NOT NULL,
    end_date DATE NOT NULL,
    opening_balance DECIMAL(18,2) NOT NULL DEFAULT 0.00,
    total_invoice DECIMAL(18,2) NOT NULL DEFAULT 0.00,
    total_payment DECIMAL(18,2) NOT NULL DEFAULT 0.00,
    closing_balance DECIMAL(18,2) NOT NULL DEFAULT 0.00,
    reconciliation_status VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    notes TEXT,
    created_by INTEGER NOT NULL REFERENCES users(id),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    confirmed_by INTEGER REFERENCES users(id),
    confirmed_at TIMESTAMP WITH TIME ZONE,
    disputed_by INTEGER REFERENCES users(id),
    disputed_at TIMESTAMP WITH TIME ZONE,
    disputed_reason TEXT
);

-- 8. 创建 fund_accounts 表
CREATE TABLE IF NOT EXISTS fund_accounts (
    id SERIAL PRIMARY KEY,
    account_no VARCHAR(50) NOT NULL UNIQUE,
    account_name VARCHAR(200) NOT NULL,
    account_type VARCHAR(20) NOT NULL,
    bank_name VARCHAR(200),
    bank_account VARCHAR(50),
    balance DECIMAL(18,2) NOT NULL DEFAULT 0.00,
    currency VARCHAR(10) NOT NULL DEFAULT 'CNY',
    remarks TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_by INTEGER NOT NULL REFERENCES users(id),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 9. 创建 ar_reconciliations 表
CREATE TABLE IF NOT EXISTS ar_reconciliations (
    id SERIAL PRIMARY KEY,
    reconciliation_no VARCHAR(50) NOT NULL UNIQUE,
    reconciliation_date DATE NOT NULL,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    customer_id INTEGER NOT NULL REFERENCES customers(id),
    customer_name VARCHAR(200),
    opening_balance DECIMAL(18,2) NOT NULL DEFAULT 0.00,
    total_invoices DECIMAL(18,2) NOT NULL DEFAULT 0.00,
    total_collections DECIMAL(18,2) NOT NULL DEFAULT 0.00,
    closing_balance DECIMAL(18,2) NOT NULL DEFAULT 0.00,
    reconciliation_status VARCHAR(20) DEFAULT 'PENDING',
    confirmed_by_customer BOOLEAN DEFAULT false,
    dispute_reason TEXT,
    confirmed_by INTEGER REFERENCES users(id),
    confirmed_at TIMESTAMP WITH TIME ZONE,
    created_by INTEGER REFERENCES users(id),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 10. 创建 ar_reconciliation_items 表
CREATE TABLE IF NOT EXISTS ar_reconciliation_items (
    id SERIAL PRIMARY KEY,
    reconciliation_id INTEGER NOT NULL REFERENCES ar_reconciliations(id),
    invoice_id INTEGER NOT NULL REFERENCES ar_invoices(id),
    amount DECIMAL(18,2) NOT NULL,
    notes TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 11. 创建 ap_payment_request 表
CREATE TABLE IF NOT EXISTS ap_payment_request (
    id SERIAL PRIMARY KEY,
    request_no VARCHAR(50) NOT NULL UNIQUE,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id),
    request_date DATE NOT NULL,
    total_amount DECIMAL(18,2) NOT NULL,
    request_status VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    notes TEXT,
    created_by INTEGER NOT NULL REFERENCES users(id),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    approved_by INTEGER REFERENCES users(id),
    approved_at TIMESTAMP WITH TIME ZONE,
    rejected_by INTEGER REFERENCES users(id),
    rejected_at TIMESTAMP WITH TIME ZONE,
    rejection_reason TEXT
);

-- 12. 创建 ap_payment_request_item 表
CREATE TABLE IF NOT EXISTS ap_payment_request_item (
    id SERIAL PRIMARY KEY,
    request_id INTEGER NOT NULL REFERENCES ap_payment_request(id),
    invoice_id INTEGER NOT NULL REFERENCES ap_invoices(id),
    amount DECIMAL(18,2) NOT NULL,
    notes TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 添加外键约束
ALTER TABLE ap_payment ADD CONSTRAINT fk_ap_payment_request FOREIGN KEY (request_id) REFERENCES ap_payment_request(id);
ALTER TABLE ap_verification_item ADD CONSTRAINT fk_ap_verification_item_invoice FOREIGN KEY (invoice_id) REFERENCES ap_invoices(id);
ALTER TABLE ap_verification_item ADD CONSTRAINT fk_ap_verification_item_payment FOREIGN KEY (payment_id) REFERENCES ap_payment(id);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_ap_payment_supplier ON ap_payment(supplier_id);
CREATE INDEX IF NOT EXISTS idx_ap_payment_status ON ap_payment(payment_status);
CREATE INDEX IF NOT EXISTS idx_ap_payment_date ON ap_payment(payment_date);
CREATE INDEX IF NOT EXISTS idx_ap_verification_supplier ON ap_verification(supplier_id);
CREATE INDEX IF NOT EXISTS idx_ap_verification_status ON ap_verification(verification_status);
CREATE INDEX IF NOT EXISTS idx_ap_reconciliation_supplier ON ap_reconciliation(supplier_id);
CREATE INDEX IF NOT EXISTS idx_ap_reconciliation_status ON ap_reconciliation(reconciliation_status);
CREATE INDEX IF NOT EXISTS idx_fund_accounts_type ON fund_accounts(account_type);
CREATE INDEX IF NOT EXISTS idx_fund_accounts_active ON fund_accounts(is_active);
CREATE INDEX IF NOT EXISTS idx_ar_reconciliations_customer ON ar_reconciliations(customer_id);
CREATE INDEX IF NOT EXISTS idx_ar_reconciliations_status ON ar_reconciliations(reconciliation_status);
CREATE INDEX IF NOT EXISTS idx_dye_recipe_status ON dye_recipe(status);
CREATE INDEX IF NOT EXISTS idx_dye_recipe_color ON dye_recipe(color_code);
