-- ============================================
-- P1 级模块 - 客户信用管理
-- ============================================
-- 文档编号：MIGRATION-033-CUSTOMER_CREDIT
-- 创建日期**: 2026-03-15
-- 说明：客户信用管理模块表结构
-- ============================================

-- 1. 客户信用评级表
-- ============================================
CREATE TABLE IF NOT EXISTS customer_credit_ratings (
    id SERIAL PRIMARY KEY,
    customer_id INTEGER NOT NULL UNIQUE,
    customer_name VARCHAR(200),
    
    -- 信用等级
    credit_level VARCHAR(10),
    credit_score INTEGER,
    
    -- 信用额度
    credit_limit DECIMAL(14,2),
    used_credit DECIMAL(14,2) DEFAULT 0,
    available_credit DECIMAL(14,2),
    
    -- 信用期限
    credit_days INTEGER,
    
    -- 评估信息
    last_assessment_date DATE,
    next_assessment_date DATE,
    
    -- 状态
    status VARCHAR(20) DEFAULT 'active',
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE customer_credit_ratings IS '客户信用评级表';
COMMENT ON COLUMN customer_credit_ratings.credit_level IS '信用等级';
COMMENT ON COLUMN customer_credit_ratings.credit_limit IS '信用额度';
COMMENT ON COLUMN customer_credit_ratings.available_credit IS '可用额度';

CREATE INDEX IF NOT EXISTS idx_customer_credit_ratings_customer ON customer_credit_ratings(customer_id);
CREATE INDEX IF NOT EXISTS idx_customer_credit_ratings_level ON customer_credit_ratings(credit_level);

-- 2. 信用变更记录表
-- ============================================
CREATE TABLE IF NOT EXISTS customer_credit_changes (
    id SERIAL PRIMARY KEY,
    customer_id INTEGER NOT NULL,
    
    -- 变更内容
    change_type VARCHAR(20),
    old_value TEXT,
    new_value TEXT,
    
    -- 变更原因
    reason TEXT,
    
    -- 审批
    approved_by INTEGER,
    approved_at TIMESTAMPTZ,
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE customer_credit_changes IS '客户信用变更记录表';
COMMENT ON COLUMN customer_credit_changes.change_type IS '变更类型';

CREATE INDEX IF NOT EXISTS idx_customer_credit_changes_customer ON customer_credit_changes(customer_id);

-- ============================================
-- 迁移完成
-- ============================================
