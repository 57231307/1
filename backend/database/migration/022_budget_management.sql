-- P1 级模块：预算管理
-- 创建时间：2026-03-15
-- 功能：预算编制、预算执行、预算控制

-- 预算科目表
CREATE TABLE IF NOT EXISTS budget_items (
    id SERIAL PRIMARY KEY,
    item_code VARCHAR(50) NOT NULL UNIQUE,
    item_name VARCHAR(100) NOT NULL,
    parent_id INTEGER REFERENCES budget_items(id),
    item_type VARCHAR(20) NOT NULL,
    level INTEGER DEFAULT 1,
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 预算方案表
CREATE TABLE IF NOT EXISTS budget_plans (
    id SERIAL PRIMARY KEY,
    plan_no VARCHAR(50) NOT NULL UNIQUE,
    plan_name VARCHAR(200) NOT NULL,
    budget_year INTEGER NOT NULL,
    budget_type VARCHAR(20) NOT NULL,
    department_id INTEGER REFERENCES departments(id),
    total_amount DECIMAL(18,2) NOT NULL,
    status VARCHAR(20) DEFAULT 'draft',
    prepared_by INTEGER,
    approved_by INTEGER,
    approved_at TIMESTAMP,
    remark TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 预算明细表
CREATE TABLE IF NOT EXISTS budget_plan_details (
    id SERIAL PRIMARY KEY,
    plan_id INTEGER NOT NULL REFERENCES budget_plans(id),
    budget_item_id INTEGER REFERENCES budget_items(id),
    period VARCHAR(7) NOT NULL,
    budget_amount DECIMAL(18,2) NOT NULL,
    actual_amount DECIMAL(18,2) DEFAULT 0,
    variance_amount DECIMAL(18,2),
    variance_rate DECIMAL(5,2),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 预算控制表
CREATE TABLE IF NOT EXISTS budget_controls (
    id SERIAL PRIMARY KEY,
    plan_id INTEGER NOT NULL REFERENCES budget_plans(id),
    budget_item_id INTEGER REFERENCES budget_items(id),
    control_type VARCHAR(20) NOT NULL,
    control_limit DECIMAL(18,2),
    warning_threshold DECIMAL(5,2) DEFAULT 80,
    control_status VARCHAR(20) DEFAULT 'normal',
    related_type VARCHAR(50),
    related_id INTEGER,
    amount DECIMAL(18,2),
    controlled_by INTEGER,
    controlled_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    remark TEXT
);

-- 预算调整表
CREATE TABLE IF NOT EXISTS budget_adjustments (
    id SERIAL PRIMARY KEY,
    adjustment_no VARCHAR(50) NOT NULL UNIQUE,
    plan_id INTEGER NOT NULL REFERENCES budget_plans(id),
    budget_item_id INTEGER REFERENCES budget_items(id),
    original_amount DECIMAL(18,2) NOT NULL,
    adjusted_amount DECIMAL(18,2) NOT NULL,
    change_amount DECIMAL(18,2),
    change_rate DECIMAL(5,2),
    reason TEXT NOT NULL,
    applied_by INTEGER,
    approved_by INTEGER,
    approved_at TIMESTAMP,
    status VARCHAR(20) DEFAULT 'pending',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_budget_items_type ON budget_items(item_type);
CREATE INDEX IF NOT EXISTS idx_budget_items_parent ON budget_items(parent_id);
CREATE INDEX IF NOT EXISTS idx_budget_plans_year ON budget_plans(budget_year);
CREATE INDEX IF NOT EXISTS idx_budget_plan_details_period ON budget_plan_details(period);
CREATE INDEX IF NOT EXISTS idx_budget_controls_plan ON budget_controls(plan_id);
CREATE INDEX IF NOT EXISTS idx_budget_adjustments_plan ON budget_adjustments(plan_id);

-- 添加中文注释
COMMENT ON TABLE budget_items IS '预算科目表';
COMMENT ON COLUMN budget_items.item_type IS '科目类型（收入/支出/资产/负债）';
COMMENT ON COLUMN budget_items.level IS '科目级别';

COMMENT ON TABLE budget_plans IS '预算方案表';
COMMENT ON COLUMN budget_plans.budget_type IS '预算类型（经营/资本/财务）';
COMMENT ON COLUMN budget_plans.total_amount IS '预算总额';

COMMENT ON TABLE budget_plan_details IS '预算明细表';
COMMENT ON COLUMN budget_plan_details.period IS '期间（YYYY-MM）';
COMMENT ON COLUMN budget_plan_details.budget_amount IS '预算金额';
COMMENT ON COLUMN budget_plan_details.actual_amount IS '实际金额';

COMMENT ON TABLE budget_controls IS '预算控制表';
COMMENT ON COLUMN budget_controls.control_type IS '控制类型（警告/冻结）';
COMMENT ON COLUMN budget_controls.warning_threshold IS '预警阈值（百分比）';

COMMENT ON TABLE budget_adjustments IS '预算调整表';
COMMENT ON COLUMN budget_adjustments.change_amount IS '变动金额';
COMMENT ON COLUMN budget_adjustments.reason IS '调整原因';
