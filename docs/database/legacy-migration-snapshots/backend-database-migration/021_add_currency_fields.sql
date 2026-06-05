-- 021_add_currency_fields.sql
-- 为多个业务表添加多币种支持字段

-- 1. 为 sales_orders 表添加 currency_code, exchange_rate 字段
ALTER TABLE sales_orders
ADD COLUMN IF NOT EXISTS currency_code VARCHAR(10) DEFAULT 'CNY' NOT NULL;

ALTER TABLE sales_orders
ADD COLUMN IF NOT EXISTS exchange_rate DECIMAL(18, 6) DEFAULT 1.000000 NOT NULL;

COMMENT ON COLUMN sales_orders.currency_code IS '币种代码，关联 currencies 表';
COMMENT ON COLUMN sales_orders.exchange_rate IS '汇率，相对于本位币';

-- 2. 为 purchase_orders 表添加 currency_code, exchange_rate 字段
-- 注意：purchase_orders 表已有 currency 和 exchange_rate 字段，这里添加 currency_code 保持一致性
ALTER TABLE purchase_orders
ADD COLUMN IF NOT EXISTS currency_code VARCHAR(10) DEFAULT 'CNY' NOT NULL;

COMMENT ON COLUMN purchase_orders.currency_code IS '币种代码，关联 currencies 表（与 currency 字段保持一致）';

-- 3. 为 ar_invoices 表添加 currency_code, exchange_rate, base_amount 字段
ALTER TABLE ar_invoices
ADD COLUMN IF NOT EXISTS currency_code VARCHAR(10) DEFAULT 'CNY' NOT NULL;

ALTER TABLE ar_invoices
ADD COLUMN IF NOT EXISTS exchange_rate DECIMAL(18, 6) DEFAULT 1.000000 NOT NULL;

ALTER TABLE ar_invoices
ADD COLUMN IF NOT EXISTS base_amount DECIMAL(18, 2) DEFAULT 0.00 NOT NULL;

COMMENT ON COLUMN ar_invoices.currency_code IS '币种代码，关联 currencies 表';
COMMENT ON COLUMN ar_invoices.exchange_rate IS '汇率，相对于本位币';
COMMENT ON COLUMN ar_invoices.base_amount IS '本位币金额 = invoice_amount * exchange_rate';

-- 4. 为 ap_invoices 表添加 currency_code, exchange_rate, base_amount 字段
-- 注意：ap_invoice 表已有 currency 和 exchange_rate 字段，这里添加 currency_code 和 base_amount 保持一致性
ALTER TABLE ap_invoice
ADD COLUMN IF NOT EXISTS currency_code VARCHAR(10) DEFAULT 'CNY' NOT NULL;

ALTER TABLE ap_invoice
ADD COLUMN IF NOT EXISTS base_amount DECIMAL(18, 2) DEFAULT 0.00 NOT NULL;

COMMENT ON COLUMN ap_invoice.currency_code IS '币种代码，关联 currencies 表（与 currency 字段保持一致）';
COMMENT ON COLUMN ap_invoice.base_amount IS '本位币金额 = amount * exchange_rate';

-- 5. 创建汇率历史表（如果不存在）
CREATE TABLE IF NOT EXISTS exchange_rate_history (
    id SERIAL PRIMARY KEY,
    from_currency VARCHAR(10) NOT NULL,
    to_currency VARCHAR(10) NOT NULL,
    rate DECIMAL(18, 6) NOT NULL,
    effective_date DATE NOT NULL,
    end_date DATE,
    source VARCHAR(50) DEFAULT 'manual',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

COMMENT ON TABLE exchange_rate_history IS '汇率历史记录表';
COMMENT ON COLUMN exchange_rate_history.source IS '汇率来源：manual=手动录入，api=外部API，system=系统自动';

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_exchange_rate_history_currencies
ON exchange_rate_history(from_currency, to_currency, effective_date DESC);

CREATE INDEX IF NOT EXISTS idx_sales_orders_currency_code
ON sales_orders(currency_code);

CREATE INDEX IF NOT EXISTS idx_purchase_orders_currency_code
ON purchase_orders(currency_code);

CREATE INDEX IF NOT EXISTS idx_ar_invoices_currency_code
ON ar_invoices(currency_code);

CREATE INDEX IF NOT EXISTS idx_ap_invoices_currency_code
ON ap_invoice(currency_code);
