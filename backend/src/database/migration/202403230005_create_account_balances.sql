-- 账户余额表迁移脚本
-- 创建时间：2026-03-23
-- 描述：存储各科目的各期间余额信息

CREATE TABLE IF NOT EXISTS account_balances (
    id SERIAL PRIMARY KEY,
    subject_id INTEGER NOT NULL COMMENT '科目ID',
    period VARCHAR(7) NOT NULL COMMENT '会计期间 (YYYY-MM)',
    initial_balance_debit DECIMAL(18, 2) NOT NULL DEFAULT 0 COMMENT '期初余额(借方)',
    initial_balance_credit DECIMAL(18, 2) NOT NULL DEFAULT 0 COMMENT '期初余额(贷方)',
    current_period_debit DECIMAL(18, 2) NOT NULL DEFAULT 0 COMMENT '本期发生额(借方)',
    current_period_credit DECIMAL(18, 2) NOT NULL DEFAULT 0 COMMENT '本期发生额(贷方)',
    ending_balance_debit DECIMAL(18, 2) NOT NULL DEFAULT 0 COMMENT '期末余额(借方)',
    ending_balance_credit DECIMAL(18, 2) NOT NULL DEFAULT 0 COMMENT '期末余额(贷方)',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP COMMENT '更新时间'
);

-- 创建唯一索引（确保每个科目每个期间只有一条记录）
CREATE UNIQUE INDEX IF NOT EXISTS idx_account_balances_subject_period
    ON account_balances(subject_id, period);

-- 创建普通索引
CREATE INDEX IF NOT EXISTS idx_account_balances_subject ON account_balances(subject_id);
CREATE INDEX IF NOT EXISTS idx_account_balances_period ON account_balances(period);

-- 添加注释
COMMENT ON TABLE account_balances IS '账户余额表';
COMMENT ON COLUMN account_balances.id IS '记录ID';
COMMENT ON COLUMN account_balances.subject_id IS '科目ID';
COMMENT ON COLUMN account_balances.period IS '会计期间';
COMMENT ON COLUMN account_balances.initial_balance_debit IS '期初余额(借方)';
COMMENT ON COLUMN account_balances.initial_balance_credit IS '期初余额(贷方)';
COMMENT ON COLUMN account_balances.current_period_debit IS '本期发生额(借方)';
COMMENT ON COLUMN account_balances.current_period_credit IS '本期发生额(贷方)';
COMMENT ON COLUMN account_balances.ending_balance_debit IS '期末余额(借方)';
COMMENT ON COLUMN account_balances.ending_balance_credit IS '期末余额(贷方)';
COMMENT ON COLUMN account_balances.created_at IS '创建时间';
COMMENT ON COLUMN account_balances.updated_at IS '更新时间';