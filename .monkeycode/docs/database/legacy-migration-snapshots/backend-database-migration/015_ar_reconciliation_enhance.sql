-- 应收对账模块增强迁移
-- 日期: 2026-05-09
-- 描述: 增强应收对账功能，添加明细表和账龄分析表

BEGIN;

-- 1. 增强应收对账单表
ALTER TABLE ar_reconciliations
    ADD COLUMN IF NOT EXISTS created_by INTEGER REFERENCES users(id),
    ADD COLUMN IF NOT EXISTS confirmed_by INTEGER REFERENCES users(id),
    ADD COLUMN IF NOT EXISTS confirmed_at TIMESTAMP;

-- 添加外键约束
ALTER TABLE ar_reconciliations
    ADD CONSTRAINT fk_ar_reconciliation_customer
    FOREIGN KEY (customer_id) REFERENCES customers(id);

-- 添加索引
CREATE INDEX IF NOT EXISTS idx_ar_reconciliation_customer ON ar_reconciliations(customer_id);
CREATE INDEX IF NOT EXISTS idx_ar_reconciliation_status ON ar_reconciliations(status);
CREATE INDEX IF NOT EXISTS idx_ar_reconciliation_date ON ar_reconciliations(start_date, end_date);

-- 2. 创建应收对账明细表
CREATE TABLE IF NOT EXISTS ar_reconciliation_items (
    id SERIAL PRIMARY KEY,
    reconciliation_id INTEGER NOT NULL REFERENCES ar_reconciliations(id) ON DELETE CASCADE,
    item_type VARCHAR(20) NOT NULL CHECK (item_type IN ('OPENING', 'INVOICE', 'RECEIPT', 'ADJUSTMENT', 'DISPUTE')),
    document_type VARCHAR(50),
    document_id INTEGER,
    document_no VARCHAR(100),
    document_date DATE,
    amount DECIMAL(15,2) NOT NULL DEFAULT 0,
    matched_amount DECIMAL(15,2) DEFAULT 0,
    match_status VARCHAR(20) NOT NULL DEFAULT 'UNMATCHED' CHECK (match_status IN ('UNMATCHED', 'MATCHED', 'PARTIAL')),
    matched_item_id INTEGER REFERENCES ar_reconciliation_items(id),
    remarks TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 添加索引
CREATE INDEX IF NOT EXISTS idx_ar_item_reconciliation ON ar_reconciliation_items(reconciliation_id);
CREATE INDEX IF NOT EXISTS idx_ar_item_match_status ON ar_reconciliation_items(match_status);
CREATE INDEX IF NOT EXISTS idx_ar_item_document ON ar_reconciliation_items(document_type, document_id);

-- 3. 创建应收账龄分析表
CREATE TABLE IF NOT EXISTS ar_aging_analysis (
    id SERIAL PRIMARY KEY,
    customer_id INTEGER NOT NULL REFERENCES customers(id),
    analysis_date DATE NOT NULL,
    current_amount DECIMAL(15,2) NOT NULL DEFAULT 0,
    days_1_30 DECIMAL(15,2) NOT NULL DEFAULT 0,
    days_31_60 DECIMAL(15,2) NOT NULL DEFAULT 0,
    days_61_90 DECIMAL(15,2) NOT NULL DEFAULT 0,
    days_over_90 DECIMAL(15,2) NOT NULL DEFAULT 0,
    total_amount DECIMAL(15,2) NOT NULL DEFAULT 0,
    salesperson_id INTEGER REFERENCES users(id),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 添加索引
CREATE INDEX IF NOT EXISTS idx_ar_aging_customer ON ar_aging_analysis(customer_id);
CREATE INDEX IF NOT EXISTS idx_ar_aging_date ON ar_aging_analysis(analysis_date);
CREATE INDEX IF NOT EXISTS idx_ar_aging_salesperson ON ar_aging_analysis(salesperson_id);

-- 添加唯一约束（每个客户每天只能有一条分析记录）
CREATE UNIQUE INDEX IF NOT EXISTS idx_ar_aging_unique 
    ON ar_aging_analysis(customer_id, analysis_date);

COMMIT;
