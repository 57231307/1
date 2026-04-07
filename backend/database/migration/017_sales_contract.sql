-- ============================================
-- P1 级模块 - 销售合同管理
-- ============================================
-- 文档编号：MIGRATION-032-SALES_CONTRACT
-- 创建日期**: 2026-03-15
-- 说明：销售合同管理模块表结构
-- ============================================

-- 1. 销售合同表
-- ============================================
CREATE TABLE sales_contracts (
    id SERIAL PRIMARY KEY,
    contract_no VARCHAR(50) NOT NULL UNIQUE,
    contract_name VARCHAR(200) NOT NULL,
    
    -- 合同类型
    contract_type VARCHAR(20),
    
    -- 客户信息
    customer_id INTEGER NOT NULL,
    customer_name VARCHAR(200),
    
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

COMMENT ON TABLE sales_contracts IS '销售合同表';
COMMENT ON COLUMN sales_contracts.contract_no IS '合同编号';
COMMENT ON COLUMN sales_contracts.total_amount IS '合同总金额';
COMMENT ON COLUMN sales_contracts.status IS '合同状态';

CREATE INDEX IF NOT EXISTS idx_sales_contracts_no ON sales_contracts(contract_no);
CREATE INDEX IF NOT EXISTS idx_sales_contracts_customer ON sales_contracts(customer_id);
CREATE INDEX IF NOT EXISTS idx_sales_contracts_status ON sales_contracts(status);

-- ============================================
-- 迁移完成
-- ============================================
