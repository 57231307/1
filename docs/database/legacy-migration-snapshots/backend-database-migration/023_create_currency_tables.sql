-- 创建 currencies 表
CREATE TABLE IF NOT EXISTS currencies (
    id SERIAL PRIMARY KEY,
    code VARCHAR(10) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    symbol VARCHAR(10),
    decimal_places INTEGER DEFAULT 2,
    is_base BOOLEAN DEFAULT false,
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 创建 exchange_rates 表
CREATE TABLE IF NOT EXISTS exchange_rates (
    id SERIAL PRIMARY KEY,
    from_currency VARCHAR(10) NOT NULL,
    to_currency VARCHAR(10) NOT NULL,
    rate DECIMAL(18,6) NOT NULL,
    effective_date DATE NOT NULL,
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_currencies_code ON currencies(code);
CREATE INDEX IF NOT EXISTS idx_currencies_status ON currencies(status);
CREATE INDEX IF NOT EXISTS idx_exchange_rates_from ON exchange_rates(from_currency);
CREATE INDEX IF NOT EXISTS idx_exchange_rates_to ON exchange_rates(to_currency);
CREATE INDEX IF NOT EXISTS idx_exchange_rates_date ON exchange_rates(effective_date);

-- 插入默认币种
INSERT INTO currencies (code, name, symbol, decimal_places, is_base, status) VALUES
('CNY', '人民币', '¥', 2, true, 'active'),
('USD', '美元', '$', 2, false, 'active'),
('EUR', '欧元', '€', 2, false, 'active'),
('GBP', '英镑', '£', 2, false, 'active'),
('JPY', '日元', '¥', 0, false, 'active')
ON CONFLICT (code) DO NOTHING;

-- 插入默认汇率
INSERT INTO exchange_rates (from_currency, to_currency, rate, effective_date, status) VALUES
('CNY', 'USD', 0.1389, CURRENT_DATE, 'active'),
('USD', 'CNY', 7.2000, CURRENT_DATE, 'active'),
('CNY', 'EUR', 0.1282, CURRENT_DATE, 'active'),
('EUR', 'CNY', 7.8000, CURRENT_DATE, 'active')
ON CONFLICT DO NOTHING;

-- 添加注释
COMMENT ON TABLE currencies IS '币种表';
COMMENT ON TABLE exchange_rates IS '汇率表';
