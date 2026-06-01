-- 应收对账模块增强迁移
-- 日期: 2026-05-09
-- 描述: 增强应收对账功能，添加明细表和账龄分析表

BEGIN;

-- 1. 增强应收对账单表
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ar_reconciliations') THEN
        -- 添加created_by字段
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'ar_reconciliations' AND column_name = 'created_by') THEN
            ALTER TABLE ar_reconciliations ADD COLUMN created_by INTEGER;
        END IF;
        -- 添加confirmed_by字段
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'ar_reconciliations' AND column_name = 'confirmed_by') THEN
            ALTER TABLE ar_reconciliations ADD COLUMN confirmed_by INTEGER;
        END IF;
        -- 添加confirmed_at字段
        IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'ar_reconciliations' AND column_name = 'confirmed_at') THEN
            ALTER TABLE ar_reconciliations ADD COLUMN confirmed_at TIMESTAMP;
        END IF;
    END IF;
END $$;

-- 添加外键约束
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ar_reconciliations') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'customers') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_ar_reconciliation_customer') THEN
            ALTER TABLE ar_reconciliations ADD CONSTRAINT fk_ar_reconciliation_customer FOREIGN KEY (customer_id) REFERENCES customers(id);
        END IF;
    END IF;
END $$;

-- 添加索引
CREATE INDEX IF NOT EXISTS idx_ar_reconciliation_customer ON ar_reconciliations(customer_id);
CREATE INDEX IF NOT EXISTS idx_ar_reconciliation_status ON ar_reconciliations(reconciliation_status);
CREATE INDEX IF NOT EXISTS idx_ar_reconciliation_date ON ar_reconciliations(period_start, period_end);

-- 2. 创建应收对账明细表
CREATE TABLE IF NOT EXISTS ar_reconciliation_items (
    id SERIAL PRIMARY KEY,
    reconciliation_id INTEGER NOT NULL,
    item_type VARCHAR(20) NOT NULL CHECK (item_type IN ('OPENING', 'INVOICE', 'RECEIPT', 'ADJUSTMENT', 'DISPUTE')),
    document_type VARCHAR(50),
    document_id INTEGER,
    document_no VARCHAR(100),
    document_date DATE,
    amount DECIMAL(15,2) NOT NULL DEFAULT 0,
    matched_amount DECIMAL(15,2) DEFAULT 0,
    match_status VARCHAR(20) NOT NULL DEFAULT 'UNMATCHED' CHECK (match_status IN ('UNMATCHED', 'MATCHED', 'PARTIAL')),
    matched_item_id INTEGER,
    remarks TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 添加外键约束
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ar_reconciliation_items') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ar_reconciliations') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_ar_item_reconciliation') THEN
            ALTER TABLE ar_reconciliation_items ADD CONSTRAINT fk_ar_item_reconciliation FOREIGN KEY (reconciliation_id) REFERENCES ar_reconciliations(id) ON DELETE CASCADE;
        END IF;
    END IF;
END $$;

-- 添加索引
CREATE INDEX IF NOT EXISTS idx_ar_item_reconciliation ON ar_reconciliation_items(reconciliation_id);
CREATE INDEX IF NOT EXISTS idx_ar_item_match_status ON ar_reconciliation_items(match_status);
CREATE INDEX IF NOT EXISTS idx_ar_item_document ON ar_reconciliation_items(document_type, document_id);

-- 3. 创建应收账龄分析表
CREATE TABLE IF NOT EXISTS ar_aging_analysis (
    id SERIAL PRIMARY KEY,
    customer_id INTEGER NOT NULL,
    analysis_date DATE NOT NULL,
    current_amount DECIMAL(15,2) NOT NULL DEFAULT 0,
    days_1_30 DECIMAL(15,2) NOT NULL DEFAULT 0,
    days_31_60 DECIMAL(15,2) NOT NULL DEFAULT 0,
    days_61_90 DECIMAL(15,2) NOT NULL DEFAULT 0,
    days_over_90 DECIMAL(15,2) NOT NULL DEFAULT 0,
    total_amount DECIMAL(15,2) NOT NULL DEFAULT 0,
    salesperson_id INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 添加外键约束
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ar_aging_analysis') 
       AND EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'customers') THEN
        IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_ar_aging_customer') THEN
            ALTER TABLE ar_aging_analysis ADD CONSTRAINT fk_ar_aging_customer FOREIGN KEY (customer_id) REFERENCES customers(id);
        END IF;
    END IF;
END $$;

-- 添加索引
CREATE INDEX IF NOT EXISTS idx_ar_aging_customer ON ar_aging_analysis(customer_id);
CREATE INDEX IF NOT EXISTS idx_ar_aging_date ON ar_aging_analysis(analysis_date);

-- 添加唯一约束（每个客户每天只能有一条分析记录）
CREATE UNIQUE INDEX IF NOT EXISTS idx_ar_aging_unique 
    ON ar_aging_analysis(customer_id, analysis_date);

COMMIT;
