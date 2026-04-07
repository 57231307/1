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
