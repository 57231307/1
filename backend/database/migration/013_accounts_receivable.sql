-- ============================================
-- 应收账款模块 - 基础表结构
-- ============================================
-- 文档编号：MIGRATION-021-AR
-- 创建日期：2026-03-15
-- 说明：应收账款模块表结构，参考应付账款模块适配销售回款场景
-- ============================================

-- 1. 应收单表
-- ============================================
CREATE TABLE ar_invoices (
    id SERIAL PRIMARY KEY,
    invoice_no VARCHAR(50) NOT NULL UNIQUE,
    invoice_date DATE NOT NULL,
    due_date DATE NOT NULL,
    
    -- 客户信息
    customer_id INTEGER NOT NULL,
    customer_name VARCHAR(200),
    customer_code VARCHAR(50),
    
    -- 来源单据
    source_type VARCHAR(20),
    source_module VARCHAR(50),
    source_bill_id INTEGER,
    source_bill_no VARCHAR(50),
    
    -- 面料行业字段
    batch_no VARCHAR(50),
    color_no VARCHAR(50),
    dye_lot_no VARCHAR(50),
    sales_order_no VARCHAR(50),
    
    -- 金额
    invoice_amount DECIMAL(14,2) NOT NULL,
    received_amount DECIMAL(14,2) DEFAULT 0,
    unpaid_amount DECIMAL(14,2) NOT NULL,
    tax_amount DECIMAL(14,2),
    
    -- 双计量单位
    quantity_meters DECIMAL(14,2),
    quantity_kg DECIMAL(14,2),
    unit_price DECIMAL(12,2),
    
    -- 状态
    status VARCHAR(20) DEFAULT 'draft',
    approval_status VARCHAR(20) DEFAULT 'unapproved',
    
    -- 审核
    created_by INTEGER,
    reviewed_by INTEGER,
    reviewed_at TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE ar_invoices IS '应收单表';
COMMENT ON COLUMN ar_invoices.invoice_no IS '应收单编号';
COMMENT ON COLUMN ar_invoices.customer_id IS '客户 ID';
COMMENT ON COLUMN ar_invoices.invoice_amount IS '应收金额';
COMMENT ON COLUMN ar_invoices.received_amount IS '已收金额';
COMMENT ON COLUMN ar_invoices.unpaid_amount IS '未收金额';

CREATE INDEX IF NOT EXISTS idx_ar_invoices_no ON ar_invoices(invoice_no);
CREATE INDEX IF NOT EXISTS idx_ar_invoices_customer ON ar_invoices(customer_id);
CREATE INDEX IF NOT EXISTS idx_ar_invoices_date ON ar_invoices(invoice_date);
CREATE INDEX IF NOT EXISTS idx_ar_invoices_status ON ar_invoices(status);
CREATE INDEX IF NOT EXISTS idx_ar_invoices_batch ON ar_invoices(batch_no);
CREATE INDEX IF NOT EXISTS idx_ar_invoices_color_no ON ar_invoices(color_no);

-- 2. 收款申请单表
-- ============================================
CREATE TABLE ar_collection_requests (
    id SERIAL PRIMARY KEY,
    request_no VARCHAR(50) NOT NULL UNIQUE,
    request_date DATE NOT NULL,
    
    -- 客户信息
    customer_id INTEGER NOT NULL,
    customer_name VARCHAR(200),
    
    -- 收款信息
    collection_amount DECIMAL(14,2) NOT NULL,
    collection_type VARCHAR(20),
    expected_date DATE,
    
    -- 关联应收单
    invoice_ids INTEGER[],
    
    -- 审批流程
    approval_level INTEGER DEFAULT 1,
    current_approver_id INTEGER,
    
    -- 状态
    status VARCHAR(20) DEFAULT 'draft',
    approval_status VARCHAR(20) DEFAULT 'pending',
    
    -- 审核
    created_by INTEGER,
    submitted_by INTEGER,
    submitted_at TIMESTAMPTZ,
    approved_by INTEGER,
    approved_at TIMESTAMPTZ,
    rejected_by INTEGER,
    rejected_at TIMESTAMPTZ,
    rejection_reason TEXT,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE ar_collection_requests IS '收款申请单表';
COMMENT ON COLUMN ar_collection_requests.request_no IS '收款申请单编号';
COMMENT ON COLUMN ar_collection_requests.customer_id IS '客户 ID';
COMMENT ON COLUMN ar_collection_requests.collection_amount IS '收款金额';
COMMENT ON COLUMN ar_collection_requests.approval_status IS '审批状态';

CREATE INDEX IF NOT EXISTS idx_ar_collection_requests_no ON ar_collection_requests(request_no);
CREATE INDEX IF NOT EXISTS idx_ar_collection_requests_customer ON ar_collection_requests(customer_id);
CREATE INDEX IF NOT EXISTS idx_ar_collection_requests_status ON ar_collection_requests(status);

-- 3. 收款单表
-- ============================================
CREATE TABLE ar_collections (
    id SERIAL PRIMARY KEY,
    collection_no VARCHAR(50) NOT NULL UNIQUE,
    collection_date DATE NOT NULL,
    
    -- 客户信息
    customer_id INTEGER NOT NULL,
    customer_name VARCHAR(200),
    
    -- 收款信息
    collection_amount DECIMAL(14,2) NOT NULL,
    collection_method VARCHAR(20),
    bank_account VARCHAR(100),
    check_no VARCHAR(50),
    
    -- 关联收款申请
    request_id INTEGER,
    request_no VARCHAR(50),
    
    -- 状态
    status VARCHAR(20) DEFAULT 'draft',
    
    -- 确认
    confirmed_by INTEGER,
    confirmed_at TIMESTAMPTZ,
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE ar_collections IS '收款单表';
COMMENT ON COLUMN ar_collections.collection_no IS '收款单编号';
COMMENT ON COLUMN ar_collections.customer_id IS '客户 ID';
COMMENT ON COLUMN ar_collections.collection_amount IS '收款金额';
COMMENT ON COLUMN ar_collections.collection_method IS '收款方式';

CREATE INDEX IF NOT EXISTS idx_ar_collections_no ON ar_collections(collection_no);
CREATE INDEX IF NOT EXISTS idx_ar_collections_customer ON ar_collections(customer_id);
CREATE INDEX IF NOT EXISTS idx_ar_collections_date ON ar_collections(collection_date);
CREATE INDEX IF NOT EXISTS idx_ar_collections_status ON ar_collections(status);

-- 4. 核销记录表
-- ============================================
CREATE TABLE ar_verifications (
    id SERIAL PRIMARY KEY,
    verification_no VARCHAR(50) NOT NULL UNIQUE,
    verification_date DATE NOT NULL,
    verification_type VARCHAR(20),
    
    -- 客户信息
    customer_id INTEGER NOT NULL,
    customer_name VARCHAR(200),
    
    -- 核销信息
    verification_amount DECIMAL(14,2) NOT NULL,
    
    -- 关联单据
    invoice_ids INTEGER[],
    collection_ids INTEGER[],
    
    -- 核销明细
    invoice_amount DECIMAL(14,2),
    collection_amount DECIMAL(14,2),
    difference_amount DECIMAL(14,2),
    
    -- 状态
    status VARCHAR(20) DEFAULT 'draft',
    
    -- 取消
    cancelled_by INTEGER,
    cancelled_at TIMESTAMPTZ,
    cancel_reason TEXT,
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE ar_verifications IS '核销记录表';
COMMENT ON COLUMN ar_verifications.verification_no IS '核销单编号';
COMMENT ON COLUMN ar_verifications.customer_id IS '客户 ID';
COMMENT ON COLUMN ar_verifications.verification_amount IS '核销金额';

CREATE INDEX IF NOT EXISTS idx_ar_verifications_no ON ar_verifications(verification_no);
CREATE INDEX IF NOT EXISTS idx_ar_verifications_customer ON ar_verifications(customer_id);
CREATE INDEX IF NOT EXISTS idx_ar_verifications_date ON ar_verifications(verification_date);

-- 5. 对账单表
-- ============================================
CREATE TABLE ar_reconciliations (
    id SERIAL PRIMARY KEY,
    reconciliation_no VARCHAR(50) NOT NULL UNIQUE,
    reconciliation_date DATE NOT NULL,
    
    -- 会计期间
    period_start DATE,
    period_end DATE,
    
    -- 客户信息
    customer_id INTEGER NOT NULL,
    customer_name VARCHAR(200),
    
    -- 对账信息
    opening_balance DECIMAL(14,2),
    total_invoices DECIMAL(14,2),
    total_collections DECIMAL(14,2),
    closing_balance DECIMAL(14,2),
    
    -- 对账结果
    reconciliation_status VARCHAR(20) DEFAULT 'pending',
    confirmed_by_customer BOOLEAN DEFAULT false,
    dispute_reason TEXT,
    
    -- 确认
    confirmed_by INTEGER,
    confirmed_at TIMESTAMPTZ,
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE ar_reconciliations IS '对账单表';
COMMENT ON COLUMN ar_reconciliations.reconciliation_no IS '对账单编号';
COMMENT ON COLUMN ar_reconciliations.customer_id IS '客户 ID';
COMMENT ON COLUMN ar_reconciliations.opening_balance IS '期初余额';
COMMENT ON COLUMN ar_reconciliations.closing_balance IS '期末余额';

CREATE INDEX IF NOT EXISTS idx_ar_reconciliations_no ON ar_reconciliations(reconciliation_no);
CREATE INDEX IF NOT EXISTS idx_ar_reconciliations_customer ON ar_reconciliations(customer_id);
CREATE INDEX IF NOT EXISTS idx_ar_reconciliations_period ON ar_reconciliations(period_start, period_end);

-- 6. 收款计划表
-- ============================================
CREATE TABLE ar_collection_plans (
    id SERIAL PRIMARY KEY,
    plan_no VARCHAR(50) NOT NULL UNIQUE,
    
    -- 客户信息
    customer_id INTEGER NOT NULL,
    customer_name VARCHAR(200),
    
    -- 计划信息
    invoice_id INTEGER,
    invoice_no VARCHAR(50),
    
    plan_amount DECIMAL(14,2) NOT NULL,
    plan_date DATE NOT NULL,
    actual_amount DECIMAL(14,2),
    actual_date DATE,
    
    -- 状态
    status VARCHAR(20) DEFAULT 'pending',
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE ar_collection_plans IS '收款计划表';
COMMENT ON COLUMN ar_collection_plans.plan_no IS '收款计划编号';
COMMENT ON COLUMN ar_collection_plans.customer_id IS '客户 ID';
COMMENT ON COLUMN ar_collection_plans.plan_amount IS '计划收款金额';
COMMENT ON COLUMN ar_collection_plans.plan_date IS '计划收款日期';

CREATE INDEX IF NOT EXISTS idx_ar_collection_plans_customer ON ar_collection_plans(customer_id);
CREATE INDEX IF NOT EXISTS idx_ar_collection_plans_date ON ar_collection_plans(plan_date);

-- ============================================
-- 迁移完成
-- ============================================
