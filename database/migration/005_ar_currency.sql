-- 迁移脚本: 005_ar_currency.sql
-- 描述: 应收对账与多币种支持模块
-- 日期: 2026-05-09
-- 依赖: 004_mrp_production.sql

BEGIN;

-- ========================================================
-- AR-001: 应收对账表状态字段增强
-- ========================================================
-- 注意：ar_reconciliations 表已在 001_consolidated_schema.sql 中创建
-- 此处添加缺失的 status 字段（如果不存在）
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'ar_reconciliations' AND column_name = 'status'
    ) THEN
        ALTER TABLE ar_reconciliations ADD COLUMN status VARCHAR(20) DEFAULT 'DRAFT';
        COMMENT ON COLUMN ar_reconciliations.status IS '状态: DRAFT=草稿, SENT=已发送, CONFIRMED=已确认, DISPUTED=有争议, CLOSED=已关闭';
        CREATE INDEX idx_ar_reconciliations_status ON ar_reconciliations(status);
    END IF;
END $$;

-- ========================================================
-- CUR-001: 币种表
-- ========================================================
CREATE TABLE IF NOT EXISTS currencies (
    id SERIAL PRIMARY KEY,
    code VARCHAR(10) UNIQUE NOT NULL,
    name VARCHAR(50) NOT NULL,
    symbol VARCHAR(10),
    is_base BOOLEAN DEFAULT false,
    precision INTEGER DEFAULT 2,
    is_active BOOLEAN DEFAULT true,
    is_deleted BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE currencies IS '币种表';

CREATE INDEX idx_currencies_code ON currencies(code);
CREATE INDEX idx_currencies_active ON currencies(is_active) WHERE is_active = true;

-- 插入默认币种
INSERT INTO currencies (code, name, symbol, is_base, precision, is_active) VALUES
('CNY', '人民币', '¥', true, 2, true),
('USD', '美元', '$', false, 2, true),
('EUR', '欧元', '€', false, 2, true)
ON CONFLICT (code) DO NOTHING;

-- ========================================================
-- CUR-002: 汇率表
-- ========================================================
CREATE TABLE IF NOT EXISTS exchange_rates (
    id SERIAL PRIMARY KEY,
    from_currency VARCHAR(10) NOT NULL,
    to_currency VARCHAR(10) NOT NULL,
    rate DECIMAL(15,6) NOT NULL,
    effective_date DATE NOT NULL,
    source VARCHAR(50),
    is_deleted BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE exchange_rates IS '汇率表';

CREATE INDEX idx_exchange_rates_currencies ON exchange_rates(from_currency, to_currency);
CREATE INDEX idx_exchange_rates_effective_date ON exchange_rates(effective_date);
CREATE UNIQUE INDEX idx_exchange_rates_unique ON exchange_rates(from_currency, to_currency, effective_date) WHERE is_deleted = false;

COMMIT;
