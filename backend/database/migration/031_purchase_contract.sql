-- ============================================
-- P1 级模块 - 采购合同管理
-- ============================================
-- 文档编号：MIGRATION-031-PURCHASE_CONTRACT
-- 创建日期**: 2026-03-15
-- 说明：采购合同管理模块表结构
-- ============================================

-- 1. 采购合同表
-- ============================================
CREATE TABLE purchase_contracts (
    id SERIAL PRIMARY KEY,
    contract_no VARCHAR(50) NOT NULL UNIQUE,
    contract_name VARCHAR(200) NOT NULL,
    
    -- 合同类型
    contract_type VARCHAR(20),
    
    -- 供应商信息
    supplier_id INTEGER NOT NULL,
    supplier_name VARCHAR(200),
    
    -- 金额信息
    total_amount DECIMAL(14,2),
    signed_date DATE,
    effective_date DATE,
    expiry_date DATE,
    
    -- 付款条款
    payment_terms TEXT,
    payment_method VARCHAR(50),
    
    -- 交货条款
    delivery_date DATE,
    delivery_location VARCHAR(200),
    
    -- 状态
    status VARCHAR(20) DEFAULT 'draft',
    
    -- 关联订单
    related_order_ids INTEGER[],
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE purchase_contracts IS '采购合同表';
COMMENT ON COLUMN purchase_contracts.contract_no IS '合同编号';
COMMENT ON COLUMN purchase_contracts.total_amount IS '合同总金额';
COMMENT ON COLUMN purchase_contracts.status IS '合同状态';

CREATE INDEX IF NOT EXISTS idx_purchase_contracts_no ON purchase_contracts(contract_no);
CREATE INDEX IF NOT EXISTS idx_purchase_contracts_supplier ON purchase_contracts(supplier_id);
CREATE INDEX IF NOT EXISTS idx_purchase_contracts_status ON purchase_contracts(status);

-- 2. 合同执行表
-- ============================================
CREATE TABLE contract_executions (
    id SERIAL PRIMARY KEY,
    contract_id INTEGER NOT NULL REFERENCES purchase_contracts(id),
    
    -- 执行信息
    execution_type VARCHAR(20),
    execution_date DATE,
    
    -- 关联单据
    related_bill_type VARCHAR(20),
    related_bill_id INTEGER,
    related_bill_no VARCHAR(50),
    
    -- 金额
    execution_amount DECIMAL(14,2),
    
    -- 状态
    status VARCHAR(20) DEFAULT 'pending',
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE contract_executions IS '合同执行表';
COMMENT ON COLUMN contract_executions.execution_type IS '执行类型';
COMMENT ON COLUMN contract_executions.execution_amount IS '执行金额';

CREATE INDEX IF NOT EXISTS idx_contract_executions_contract ON contract_executions(contract_id);

-- ============================================
-- 迁移完成
-- ============================================
