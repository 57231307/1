-- ============================================
-- P1 级模块 - 固定资产管理
-- ============================================
-- 文档编号：MIGRATION-030-FIXED_ASSETS
-- 创建日期：2026-03-15
-- 说明：固定资产管理模块表结构
-- ============================================

-- 1. 固定资产卡片表
-- ============================================
CREATE TABLE IF NOT EXISTS fixed_assets (
    id SERIAL PRIMARY KEY,
    asset_no VARCHAR(50) NOT NULL UNIQUE,
    asset_name VARCHAR(200) NOT NULL,
    asset_category VARCHAR(50),
    
    -- 规格型号
    specification VARCHAR(200),
    model VARCHAR(100),
    
    -- 使用信息
    use_department_id INTEGER,
    use_location VARCHAR(200),
    responsible_person_id INTEGER,
    
    -- 价值信息
    original_value DECIMAL(14,2) NOT NULL,
    salvage_value DECIMAL(14,2),
    salvage_rate DECIMAL(8,4),
    depreciable_value DECIMAL(14,2),
    
    -- 折旧信息
    depreciation_method VARCHAR(20),
    useful_life INTEGER,
    monthly_depreciation DECIMAL(14,2),
    accumulated_depreciation DECIMAL(14,2) DEFAULT 0,
    net_value DECIMAL(14,2),
    
    -- 状态
    status VARCHAR(20) DEFAULT 'active',
    purchase_date DATE,
    in_service_date DATE,
    disposal_date DATE,
    
    -- 供应商信息
    supplier_id INTEGER,
    supplier_name VARCHAR(200),
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE fixed_assets IS '固定资产卡片表';
COMMENT ON COLUMN fixed_assets.asset_no IS '资产编号';
COMMENT ON COLUMN fixed_assets.original_value IS '原值';
COMMENT ON COLUMN fixed_assets.accumulated_depreciation IS '累计折旧';
COMMENT ON COLUMN fixed_assets.net_value IS '净值';

CREATE INDEX IF NOT EXISTS idx_fixed_assets_no ON fixed_assets(asset_no);
CREATE INDEX IF NOT EXISTS idx_fixed_assets_category ON fixed_assets(asset_category);
CREATE INDEX IF NOT EXISTS idx_fixed_assets_status ON fixed_assets(status);

-- 2. 折旧记录表
-- ============================================
CREATE TABLE IF NOT EXISTS depreciation_records (
    id SERIAL PRIMARY KEY,
    asset_id INTEGER NOT NULL REFERENCES fixed_assets(id),
    
    -- 会计期间
    period VARCHAR(7) NOT NULL,
    
    -- 折旧金额
    monthly_depreciation DECIMAL(14,2),
    accumulated_depreciation DECIMAL(14,2),
    
    -- 状态
    status VARCHAR(20) DEFAULT 'pending',
    posted_by INTEGER,
    posted_at TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE depreciation_records IS '折旧记录表';
COMMENT ON COLUMN depreciation_records.period IS '会计期间';
COMMENT ON COLUMN depreciation_records.monthly_depreciation IS '月折旧额';

CREATE INDEX IF NOT EXISTS idx_depreciation_records_asset ON depreciation_records(asset_id);
CREATE INDEX IF NOT EXISTS idx_depreciation_records_period ON depreciation_records(period);

-- 3. 资产处置表
-- ============================================
CREATE TABLE IF NOT EXISTS asset_disposals (
    id SERIAL PRIMARY KEY,
    disposal_no VARCHAR(50) NOT NULL UNIQUE,
    asset_id INTEGER NOT NULL REFERENCES fixed_assets(id),
    
    -- 处置信息
    disposal_type VARCHAR(20),
    disposal_date DATE,
    disposal_reason TEXT,
    
    -- 金额
    disposal_value DECIMAL(14,2),
    disposal_cost DECIMAL(14,2),
    net_gain_loss DECIMAL(14,2),
    
    -- 审批
    approved_by INTEGER,
    approved_at TIMESTAMPTZ,
    
    created_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE asset_disposals IS '资产处置表';
COMMENT ON COLUMN asset_disposals.disposal_type IS '处置类型';
COMMENT ON COLUMN asset_disposals.net_gain_loss IS '处置损益';

CREATE INDEX IF NOT EXISTS idx_asset_disposals_no ON asset_disposals(disposal_no);
CREATE INDEX IF NOT EXISTS idx_asset_disposals_asset ON asset_disposals(asset_id);

-- ============================================
-- 迁移完成
-- ============================================
