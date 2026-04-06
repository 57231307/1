-- P2 级模块：采购价格管理
-- 创建时间：2026-03-15
-- 功能：采购价格管理、价格审批、价格趋势分析

-- 采购价格表
CREATE TABLE purchase_prices (
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
CREATE TABLE purchase_price_approvals (
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
CREATE TABLE purchase_price_history (
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
