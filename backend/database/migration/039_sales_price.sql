-- P2 级模块：销售价格管理
-- 创建时间：2026-03-15
-- 功能：销售价格管理、价格审批、价格策略

-- 销售价格表
CREATE TABLE sales_prices (
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
CREATE TABLE sales_price_approvals (
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
CREATE TABLE sales_price_history (
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
CREATE TABLE price_strategies (
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
