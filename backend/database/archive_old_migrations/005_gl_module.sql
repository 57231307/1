-- ========================================
-- 总账管理模块（面料行业版）
-- 版本：2026-03-15
-- 说明：财务系统核心基础模块
-- ========================================

-- ========================================
-- 1. 会计科目表
-- ========================================
CREATE TABLE IF NOT EXISTS account_subjects (
    id SERIAL PRIMARY KEY,
    code VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(200) NOT NULL,
    level INTEGER NOT NULL,
    parent_id INTEGER REFERENCES account_subjects(id),
    full_code VARCHAR(200),
    
    -- 余额属性
    balance_direction VARCHAR(10),
    initial_balance_debit DECIMAL(14,2) DEFAULT 0,
    initial_balance_credit DECIMAL(14,2) DEFAULT 0,
    current_period_debit DECIMAL(14,2) DEFAULT 0,
    current_period_credit DECIMAL(14,2) DEFAULT 0,
    ending_balance_debit DECIMAL(14,2) DEFAULT 0,
    ending_balance_credit DECIMAL(14,2) DEFAULT 0,
    
    -- 辅助核算
    assist_customer BOOLEAN DEFAULT false,
    assist_supplier BOOLEAN DEFAULT false,
    assist_department BOOLEAN DEFAULT false,
    assist_employee BOOLEAN DEFAULT false,
    assist_project BOOLEAN DEFAULT false,
    assist_batch BOOLEAN DEFAULT false,           -- 面料行业：批次核算
    assist_color_no BOOLEAN DEFAULT false,        -- 面料行业：色号核算
    assist_dye_lot BOOLEAN DEFAULT false,         -- 面料行业：缸号核算
    assist_grade BOOLEAN DEFAULT false,           -- 面料行业：等级核算
    assist_workshop BOOLEAN DEFAULT false,        -- 面料行业：车间核算
    
    -- 双计量单位
    enable_dual_unit BOOLEAN DEFAULT false,       -- 面料行业：双计量单位
    primary_unit VARCHAR(20) DEFAULT '米',        -- 主单位
    secondary_unit VARCHAR(20) DEFAULT '公斤',    -- 辅单位
    
    -- 控制属性
    is_cash_account BOOLEAN DEFAULT false,
    is_bank_account BOOLEAN DEFAULT false,
    allow_manual_entry BOOLEAN DEFAULT true,
    require_summary BOOLEAN DEFAULT false,
    
    -- 状态
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE account_subjects IS '会计科目表（面料行业版）';
COMMENT ON COLUMN account_subjects.code IS '科目编码';
COMMENT ON COLUMN account_subjects.name IS '科目名称';
COMMENT ON COLUMN account_subjects.level IS '科目级别（1-6）';
COMMENT ON COLUMN account_subjects.balance_direction IS '余额方向：借/贷/无';
COMMENT ON COLUMN account_subjects.assist_batch IS '是否启用批次辅助核算';
COMMENT ON COLUMN account_subjects.assist_color_no IS '是否启用色号辅助核算';
COMMENT ON COLUMN account_subjects.enable_dual_unit IS '是否启用双计量单位';

CREATE INDEX IF NOT EXISTS idx_account_subjects_code ON account_subjects(code);
CREATE INDEX IF NOT EXISTS idx_account_subjects_parent ON account_subjects(parent_id);
CREATE INDEX IF NOT EXISTS idx_account_subjects_level ON account_subjects(level);

-- ========================================
-- 2. 凭证表
-- ========================================
CREATE TABLE IF NOT EXISTS vouchers (
    id SERIAL PRIMARY KEY,
    voucher_no VARCHAR(50) NOT NULL UNIQUE,
    voucher_type VARCHAR(20) NOT NULL,
    voucher_date DATE NOT NULL,
    
    -- 凭证来源
    source_type VARCHAR(20),
    source_module VARCHAR(50),
    source_bill_id INTEGER,
    source_bill_no VARCHAR(50),
    
    -- 面料行业字段
    batch_no VARCHAR(50),                 -- 批次号
    color_no VARCHAR(50),                 -- 色号
    dye_lot_no VARCHAR(50),               -- 缸号
    workshop VARCHAR(100),                -- 车间
    production_order_no VARCHAR(50),      -- 生产订单号
    
    -- 双计量单位
    quantity_meters DECIMAL(14,2),        -- 数量（米）
    quantity_kg DECIMAL(14,2),            -- 数量（公斤）
    gram_weight DECIMAL(10,2),            -- 克重
    
    -- 状态
    attachment_count INTEGER DEFAULT 0,
    status VARCHAR(20) DEFAULT 'draft',
    
    -- 审核
    created_by INTEGER,
    reviewed_by INTEGER,
    reviewed_at TIMESTAMPTZ,
    posted_by INTEGER,
    posted_at TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE vouchers IS '凭证表（面料行业版）';
COMMENT ON COLUMN vouchers.voucher_no IS '凭证字号';
COMMENT ON COLUMN vouchers.voucher_type IS '凭证类型：记/收/付/转';
COMMENT ON COLUMN vouchers.batch_no IS '批次号';
COMMENT ON COLUMN vouchers.color_no IS '色号';
COMMENT ON COLUMN vouchers.quantity_meters IS '数量（米）';
COMMENT ON COLUMN vouchers.quantity_kg IS '数量（公斤）';

CREATE INDEX IF NOT EXISTS idx_vouchers_no ON vouchers(voucher_no);
CREATE INDEX IF NOT EXISTS idx_vouchers_date ON vouchers(voucher_date);
CREATE INDEX IF NOT EXISTS idx_vouchers_status ON vouchers(status);
CREATE INDEX IF NOT EXISTS idx_vouchers_batch ON vouchers(batch_no, color_no);

-- ========================================
-- 3. 凭证分录表
-- ========================================
CREATE TABLE IF NOT EXISTS voucher_items (
    id SERIAL PRIMARY KEY,
    voucher_id INTEGER NOT NULL REFERENCES vouchers(id) ON DELETE CASCADE,
    line_no INTEGER NOT NULL,
    
    -- 科目
    subject_code VARCHAR(50) NOT NULL,
    subject_name VARCHAR(200) NOT NULL,
    
    -- 金额
    debit DECIMAL(14,2) DEFAULT 0,
    credit DECIMAL(14,2) DEFAULT 0,
    
    -- 摘要
    summary TEXT,
    
    -- 辅助核算
    assist_customer_id INTEGER,
    assist_supplier_id INTEGER,
    assist_department_id INTEGER,
    assist_employee_id INTEGER,
    assist_project_id INTEGER,
    assist_batch_id INTEGER,              -- 面料行业：批次
    assist_color_no_id INTEGER,           -- 面料行业：色号
    assist_dye_lot_id INTEGER,            -- 面料行业：缸号
    assist_grade VARCHAR(20),             -- 面料行业：等级
    assist_workshop_id INTEGER,           -- 面料行业：车间
    
    -- 双计量单位
    quantity_meters DECIMAL(14,2),        -- 数量（米）
    quantity_kg DECIMAL(14,2),            -- 数量（公斤）
    unit_price DECIMAL(12,2),             -- 单价
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE voucher_items IS '凭证分录表（面料行业版）';
COMMENT ON COLUMN voucher_items.assist_batch_id IS '批次辅助核算 ID';
COMMENT ON COLUMN voucher_items.assist_color_no_id IS '色号辅助核算 ID';
COMMENT ON COLUMN voucher_items.quantity_meters IS '数量（米）';

CREATE INDEX IF NOT EXISTS idx_voucher_items_voucher ON voucher_items(voucher_id);
CREATE INDEX IF NOT EXISTS idx_voucher_items_subject ON voucher_items(subject_code);
CREATE INDEX IF NOT EXISTS idx_voucher_items_batch ON voucher_items(assist_batch_id, assist_color_no_id);

-- ========================================
-- 4. 会计期间表
-- ========================================
CREATE TABLE IF NOT EXISTS accounting_periods (
    id SERIAL PRIMARY KEY,
    year INTEGER NOT NULL,
    period INTEGER NOT NULL,
    period_name VARCHAR(50) NOT NULL,
    start_date DATE NOT NULL,
    end_date DATE NOT NULL,
    status VARCHAR(20) DEFAULT 'OPEN',
    closed_at TIMESTAMPTZ,
    closed_by INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE accounting_periods IS '会计期间表';
CREATE UNIQUE INDEX IF NOT EXISTS idx_accounting_periods_year_period ON accounting_periods(year, period);

-- ========================================
-- 5. 初始化会计科目（面料行业）
-- ========================================

-- 插入一级科目
INSERT INTO account_subjects (code, name, level, balance_direction, status) VALUES
('1001', '库存现金', 1, '借', 'active'),
('1002', '银行存款', 1, '借', 'active'),
('1122', '应收账款', 1, '借', 'active'),
('1405', '库存商品', 1, '借', 'active'),
('2202', '应付账款', 1, '贷', 'active'),
('2221', '应交税费', 1, '贷', 'active'),
('5001', '生产成本', 1, '借', 'active'),
('6001', '主营业务收入', 1, '贷', 'active'),
('6401', '主营业务成本', 1, '借', 'active') ON CONFLICT DO NOTHING;

-- 插入二级科目（示例）
INSERT INTO account_subjects (code, name, level, parent_id, balance_direction, status)
SELECT 
    '1002.01', '工商银行', 2, id, '借', 'active'
FROM account_subjects WHERE code = '1002' ON CONFLICT DO NOTHING;

INSERT INTO account_subjects (code, name, level, parent_id, balance_direction, status)
SELECT 
    '1405.01', '坯布', 2, id, '借', 'active'
FROM account_subjects WHERE code = '1405' ON CONFLICT DO NOTHING;

INSERT INTO account_subjects (code, name, level, parent_id, balance_direction, status, assist_batch, assist_color_no, enable_dual_unit)
SELECT 
    '1405.02', '成品布', 2, id, '借', 'active', true, true, true
FROM account_subjects WHERE code = '1405' ON CONFLICT DO NOTHING;

-- ========================================
-- 6. 创建视图
-- ========================================

-- 科目余额视图
CREATE OR REPLACE VIEW v_account_balance AS
SELECT 
    id,
    code,
    name,
    level,
    balance_direction,
    initial_balance_debit,
    initial_balance_credit,
    current_period_debit,
    current_period_credit,
    ending_balance_debit,
    ending_balance_credit
FROM account_subjects
WHERE status = 'active';

COMMENT ON VIEW v_account_balance IS '科目余额视图';

-- ========================================
-- 迁移完成提示
-- ========================================

DO $$
BEGIN
    RAISE NOTICE '========================================';
    RAISE NOTICE '总账管理模块（面料行业版）迁移完成';
    RAISE NOTICE '版本：2026-03-15';
    RAISE NOTICE '========================================';
    RAISE NOTICE '新增表：4 个';
    RAISE NOTICE '  - account_subjects (会计科目表)';
    RAISE NOTICE '  - vouchers (凭证表)';
    RAISE NOTICE '  - voucher_items (凭证分录表)';
    RAISE NOTICE '  - accounting_periods (会计期间表)';
    RAISE NOTICE '';
    RAISE NOTICE '创建视图：1 个';
    RAISE NOTICE '  - v_account_balance (科目余额视图)';
    RAISE NOTICE '========================================';
END $$;
