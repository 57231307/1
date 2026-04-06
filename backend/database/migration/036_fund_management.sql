-- P1 级模块：资金管理
-- 创建时间：2026-03-15
-- 功能：资金计划、资金调拨、资金监控

-- 资金账户表
CREATE TABLE fund_accounts (
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
CREATE TABLE fund_plans (
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
CREATE TABLE fund_transfers (
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
CREATE TABLE fund_transactions (
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
CREATE TABLE fund_monitoring (
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
