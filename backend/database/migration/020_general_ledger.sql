-- ============================================
-- 总账管理模块 - 基础表结构（阶段 1）
-- ============================================
-- 文档编号：MIGRATION-020-GL-PHASE1
-- 创建日期：2026-03-15
-- 说明：总账管理模块基础表结构，包含会计科目、凭证、凭证分录、科目余额表
-- ============================================

-- 1. 会计科目表（基础版）
-- ============================================
CREATE TABLE account_subjects (
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
    
    -- 辅助核算（阶段 1 基础版，阶段 2 扩展面料行业字段）
    assist_customer BOOLEAN DEFAULT false,
    assist_supplier BOOLEAN DEFAULT false,
    assist_department BOOLEAN DEFAULT false,
    assist_employee BOOLEAN DEFAULT false,
    assist_project BOOLEAN DEFAULT false,
    assist_batch BOOLEAN DEFAULT false,
    assist_color_no BOOLEAN DEFAULT false,
    assist_dye_lot BOOLEAN DEFAULT false,
    assist_grade BOOLEAN DEFAULT false,
    assist_workshop BOOLEAN DEFAULT false,
    
    -- 双计量单位（阶段 2 使用）
    enable_dual_unit BOOLEAN DEFAULT false,
    primary_unit VARCHAR(20) DEFAULT '米',
    secondary_unit VARCHAR(20) DEFAULT '公斤',
    
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

COMMENT ON TABLE account_subjects IS '会计科目表';
COMMENT ON COLUMN account_subjects.code IS '科目编码';
COMMENT ON COLUMN account_subjects.name IS '科目名称';
COMMENT ON COLUMN account_subjects.level IS '科目级次';
COMMENT ON COLUMN account_subjects.parent_id IS '父科目 ID';
COMMENT ON COLUMN account_subjects.full_code IS '完整编码';
COMMENT ON COLUMN account_subjects.balance_direction IS '余额方向（借/贷）';
COMMENT ON COLUMN account_subjects.initial_balance_debit IS '期初借方余额';
COMMENT ON COLUMN account_subjects.initial_balance_credit IS '期初贷方余额';
COMMENT ON COLUMN account_subjects.current_period_debit IS '本期借方发生额';
COMMENT ON COLUMN account_subjects.current_period_credit IS '本期贷方发生额';
COMMENT ON COLUMN account_subjects.ending_balance_debit IS '期末借方余额';
COMMENT ON COLUMN account_subjects.ending_balance_credit IS '期末贷方余额';
COMMENT ON COLUMN account_subjects.assist_customer IS '客户辅助核算';
COMMENT ON COLUMN account_subjects.assist_supplier IS '供应商辅助核算';
COMMENT ON COLUMN account_subjects.assist_batch IS '批次辅助核算';
COMMENT ON COLUMN account_subjects.assist_color_no IS '色号辅助核算';
COMMENT ON COLUMN account_subjects.enable_dual_unit IS '启用双计量单位';

-- 索引
CREATE INDEX idx_account_subjects_code ON account_subjects(code);
CREATE INDEX idx_account_subjects_parent ON account_subjects(parent_id);
CREATE INDEX idx_account_subjects_level ON account_subjects(level);
CREATE INDEX idx_account_subjects_status ON account_subjects(status);

-- 2. 凭证表（基础版）
-- ============================================
CREATE TABLE vouchers (
    id SERIAL PRIMARY KEY,
    voucher_no VARCHAR(50) NOT NULL UNIQUE,
    voucher_type VARCHAR(20) NOT NULL,
    voucher_date DATE NOT NULL,
    
    -- 凭证来源
    source_type VARCHAR(20),
    source_module VARCHAR(50),
    source_bill_id INTEGER,
    source_bill_no VARCHAR(50),
    
    -- 面料行业字段（阶段 2 使用）
    batch_no VARCHAR(50),
    color_no VARCHAR(50),
    dye_lot_no VARCHAR(50),
    workshop VARCHAR(100),
    production_order_no VARCHAR(50),
    
    -- 双计量单位（阶段 2 使用）
    quantity_meters DECIMAL(14,2),
    quantity_kg DECIMAL(14,2),
    gram_weight DECIMAL(10,2),
    
    -- 状态
    status VARCHAR(20) DEFAULT 'draft',
    attachment_count INTEGER DEFAULT 0,
    
    -- 审核
    created_by INTEGER,
    reviewed_by INTEGER,
    reviewed_at TIMESTAMPTZ,
    posted_by INTEGER,
    posted_at TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE vouchers IS '凭证表';
COMMENT ON COLUMN vouchers.voucher_no IS '凭证编号';
COMMENT ON COLUMN vouchers.voucher_type IS '凭证类型（记/收/付/转）';
COMMENT ON COLUMN vouchers.voucher_date IS '凭证日期';
COMMENT ON COLUMN vouchers.source_type IS '来源类型';
COMMENT ON COLUMN vouchers.source_module IS '来源模块';
COMMENT ON COLUMN vouchers.source_bill_id IS '来源单据 ID';
COMMENT ON COLUMN vouchers.source_bill_no IS '来源单据编号';
COMMENT ON COLUMN vouchers.status IS '状态（draft/submitted/reviewed/posted）';
COMMENT ON COLUMN vouchers.created_by IS '制单人 ID';
COMMENT ON COLUMN vouchers.reviewed_by IS '审核人 ID';
COMMENT ON COLUMN vouchers.posted_by IS '过账人 ID';

-- 索引
CREATE INDEX idx_vouchers_no ON vouchers(voucher_no);
CREATE INDEX idx_vouchers_date ON vouchers(voucher_date);
CREATE INDEX idx_vouchers_type ON vouchers(voucher_type);
CREATE INDEX idx_vouchers_status ON vouchers(status);
CREATE INDEX idx_vouchers_created_by ON vouchers(created_by);

-- 3. 凭证分录表
-- ============================================
CREATE TABLE voucher_items (
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
    
    -- 辅助核算（阶段 1 基础版）
    assist_customer_id INTEGER,
    assist_supplier_id INTEGER,
    assist_department_id INTEGER,
    assist_employee_id INTEGER,
    assist_project_id INTEGER,
    assist_batch_id INTEGER,
    assist_color_no_id INTEGER,
    assist_dye_lot_id INTEGER,
    assist_grade VARCHAR(20),
    assist_workshop_id INTEGER,
    
    -- 双计量单位（阶段 2 使用）
    quantity_meters DECIMAL(14,2),
    quantity_kg DECIMAL(14,2),
    unit_price DECIMAL(12,2),
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE voucher_items IS '凭证分录表';
COMMENT ON COLUMN voucher_items.voucher_id IS '凭证 ID';
COMMENT ON COLUMN voucher_items.line_no IS '分录行号';
COMMENT ON COLUMN voucher_items.subject_code IS '科目编码';
COMMENT ON COLUMN voucher_items.subject_name IS '科目名称';
COMMENT ON COLUMN voucher_items.debit IS '借方金额';
COMMENT ON COLUMN voucher_items.credit IS '贷方金额';
COMMENT ON COLUMN voucher_items.summary IS '摘要';
COMMENT ON COLUMN voucher_items.assist_batch_id IS '批次辅助核算 ID';
COMMENT ON COLUMN voucher_items.assist_color_no_id IS '色号辅助核算 ID';

-- 索引
CREATE INDEX idx_voucher_items_voucher ON voucher_items(voucher_id);
CREATE INDEX idx_voucher_items_subject ON voucher_items(subject_code);
CREATE INDEX idx_voucher_items_line_no ON voucher_items(voucher_id, line_no);

-- 4. 科目余额表（基础版）
-- ============================================
CREATE TABLE account_balances (
    id SERIAL PRIMARY KEY,
    subject_id INTEGER NOT NULL REFERENCES account_subjects(id),
    period VARCHAR(7) NOT NULL,
    
    -- 辅助核算维度（阶段 1 基础版，支持组合）
    assist_customer_id INTEGER,
    assist_supplier_id INTEGER,
    assist_department_id INTEGER,
    assist_employee_id INTEGER,
    assist_project_id INTEGER,
    assist_batch_id INTEGER,
    assist_color_no_id INTEGER,
    assist_dye_lot_id INTEGER,
    assist_grade VARCHAR(20),
    assist_workshop_id INTEGER,
    
    -- 双计量单位（阶段 2 使用）
    quantity_meters DECIMAL(14,2),
    quantity_kg DECIMAL(14,2),
    
    -- 余额
    initial_balance_debit DECIMAL(14,2) DEFAULT 0,
    initial_balance_credit DECIMAL(14,2) DEFAULT 0,
    current_period_debit DECIMAL(14,2) DEFAULT 0,
    current_period_credit DECIMAL(14,2) DEFAULT 0,
    ending_balance_debit DECIMAL(14,2) DEFAULT 0,
    ending_balance_credit DECIMAL(14,2) DEFAULT 0,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    -- 唯一约束：支持按辅助核算维度组合
    UNIQUE(subject_id, period, assist_customer_id, assist_supplier_id, 
           assist_department_id, assist_employee_id, assist_project_id,
           assist_batch_id, assist_color_no_id)
);

COMMENT ON TABLE account_balances IS '科目余额表';
COMMENT ON COLUMN account_balances.subject_id IS '科目 ID';
COMMENT ON COLUMN account_balances.period IS '会计期间（YYYY-MM）';
COMMENT ON COLUMN account_balances.initial_balance_debit IS '期初借方余额';
COMMENT ON COLUMN account_balances.initial_balance_credit IS '期初贷方余额';
COMMENT ON COLUMN account_balances.current_period_debit IS '本期借方发生额';
COMMENT ON COLUMN account_balances.current_period_credit IS '本期贷方发生额';
COMMENT ON COLUMN account_balances.ending_balance_debit IS '期末借方余额';
COMMENT ON COLUMN account_balances.ending_balance_credit IS '期末贷方余额';

-- 索引
CREATE INDEX idx_account_balances_period ON account_balances(period);
CREATE INDEX idx_account_balances_subject ON account_balances(subject_id);
CREATE INDEX idx_account_balances_subject_period ON account_balances(subject_id, period);
CREATE INDEX idx_account_balances_batch ON account_balances(assist_batch_id);
CREATE INDEX idx_account_balances_color_no ON account_balances(assist_color_no_id);

-- 5. 插入预设科目（基础科目）
-- ============================================
INSERT INTO account_subjects (code, name, level, parent_id, full_code, balance_direction, status) VALUES
-- 资产类
('1001', '库存现金', 1, NULL, '1001', '借', 'active'),
('1002', '银行存款', 1, NULL, '1002', '借', 'active'),
('1122', '应收账款', 1, NULL, '1122', '借', 'active'),
('1405', '库存商品', 1, NULL, '1405', '借', 'active'),
('1405.01', '库存商品 - 坯布', 2, (SELECT id FROM account_subjects WHERE code = '1405'), '1405.01', '借', 'active'),
('1405.02', '库存商品 - 成品布', 2, (SELECT id FROM account_subjects WHERE code = '1405'), '1405.02', '借', 'active'),
('1405.03', '库存商品 - 辅料', 2, (SELECT id FROM account_subjects WHERE code = '1405'), '1405.03', '借', 'active'),
('1601', '固定资产', 1, NULL, '1601', '借', 'active'),

-- 负债类
('2001', '短期借款', 1, NULL, '2001', '贷', 'active'),
('2202', '应付账款', 1, NULL, '2202', '贷', 'active'),
('2202.01', '应付账款 - 坯布供应商', 2, (SELECT id FROM account_subjects WHERE code = '2202'), '2202.01', '贷', 'active'),
('2202.02', '应付账款 - 辅料供应商', 2, (SELECT id FROM account_subjects WHERE code = '2202'), '2202.02', '贷', 'active'),
('2202.03', '应付账款 - 印染厂', 2, (SELECT id FROM account_subjects WHERE code = '2202'), '2202.03', '贷', 'active'),
('2221', '应交税费', 1, NULL, '2221', '贷', 'active'),
('2221.01.01', '应交税费 - 应交增值税（进项税额）', 3, (SELECT id FROM account_subjects WHERE code = '2221'), '2221.01.01', '借', 'active'),
('2221.01.02', '应交税费 - 应交增值税（销项税额）', 3, (SELECT id FROM account_subjects WHERE code = '2221'), '2221.01.02', '贷', 'active'),

-- 所有者权益类
('3001', '实收资本', 1, NULL, '3001', '贷', 'active'),
('3101', '盈余公积', 1, NULL, '3101', '贷', 'active'),
('3201', '本年利润', 1, NULL, '3201', '贷', 'active'),

-- 成本类
('5001', '生产成本', 1, NULL, '5001', '借', 'active'),
('5001.01', '生产成本 - 直接材料', 2, (SELECT id FROM account_subjects WHERE code = '5001'), '5001.01', '借', 'active'),
('5001.01.001', '生产成本 - 直接材料 - 坯布成本', 3, (SELECT id FROM account_subjects WHERE code = '5001.01'), '5001.01.001', '借', 'active'),
('5001.01.002', '生产成本 - 直接材料 - 染料成本', 3, (SELECT id FROM account_subjects WHERE code = '5001.01'), '5001.01.002', '借', 'active'),
('5001.02', '生产成本 - 直接人工', 2, (SELECT id FROM account_subjects WHERE code = '5001'), '5001.02', '借', 'active'),
('5001.03', '生产成本 - 制造费用', 2, (SELECT id FROM account_subjects WHERE code = '5001'), '5001.03', '借', 'active'),
('5001.04', '生产成本 - 委托加工费', 2, (SELECT id FROM account_subjects WHERE code = '5001'), '5001.04', '借', 'active'),
('5001.04.001', '生产成本 - 委托加工费 - 染费', 3, (SELECT id FROM account_subjects WHERE code = '5001.04'), '5001.04.001', '借', 'active'),

-- 损益类
('6001', '主营业务收入', 1, NULL, '6001', '贷', 'active'),
('6001.01', '主营业务收入 - 国内销售', 2, (SELECT id FROM account_subjects WHERE code = '6001'), '6001.01', '贷', 'active'),
('6001.01.001', '主营业务收入 - 国内销售 - 坯布', 3, (SELECT id FROM account_subjects WHERE code = '6001.01'), '6001.01.001', '贷', 'active'),
('6001.01.002', '主营业务收入 - 国内销售 - 成品布', 3, (SELECT id FROM account_subjects WHERE code = '6001.01'), '6001.01.002', '贷', 'active'),
('6401', '主营业务成本', 1, NULL, '6401', '借', 'active'),
('6401.01', '主营业务成本 - 坯布销售成本', 2, (SELECT id FROM account_subjects WHERE code = '6401'), '6401.01', '借', 'active'),
('6401.02', '主营业务成本 - 成品布销售成本', 2, (SELECT id FROM account_subjects WHERE code = '6401'), '6401.02', '借', 'active'),
('6601', '销售费用', 1, NULL, '6601', '借', 'active'),
('6602', '管理费用', 1, NULL, '6602', '借', 'active'),
('6603', '财务费用', 1, NULL, '6603', '借', 'active');

COMMENT ON INSERT: '插入基础会计科目（面料行业预设科目）';

-- 6. 触发器：自动更新科目余额
-- ============================================
CREATE OR REPLACE FUNCTION update_account_subject_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_update_account_subjects
BEFORE UPDATE ON account_subjects
FOR EACH ROW
EXECUTE FUNCTION update_account_subject_timestamp();

CREATE TRIGGER trg_update_vouchers
BEFORE UPDATE ON vouchers
FOR EACH ROW
EXECUTE FUNCTION update_account_subject_timestamp();

CREATE TRIGGER trg_update_account_balances
BEFORE UPDATE ON account_balances
FOR EACH ROW
EXECUTE FUNCTION update_account_subject_timestamp();

COMMENT ON FUNCTION update_account_subject_timestamp() IS '自动更新 updated_at 字段';

-- 7. 凭证编号生成规则（按月连续编号）
-- ============================================
CREATE OR REPLACE FUNCTION generate_voucher_no(
    p_voucher_type VARCHAR,
    p_voucher_date DATE
) RETURNS VARCHAR AS $$
DECLARE
    v_year_month VARCHAR(7);
    v_prefix VARCHAR(10);
    v_seq INTEGER;
    v_voucher_no VARCHAR(50);
BEGIN
    -- 获取年月
    v_year_month := TO_CHAR(p_voucher_date, 'YYYY-MM');
    
    -- 凭证类型前缀
    CASE p_voucher_type
        WHEN '记' THEN v_prefix := 'JZ';
        WHEN '收' THEN v_prefix := 'SK';
        WHEN '付' THEN v_prefix := 'FK';
        WHEN '转' THEN v_prefix := 'ZZ';
        ELSE v_prefix := 'JZ';
    END CASE;
    
    -- 获取当月最大序号
    SELECT COALESCE(MAX(
        CAST(SUBSTRING(voucher_no FROM LENGTH(v_prefix) + LENGTH(v_year_month) + 3) AS INTEGER)
    ), 0) + 1 INTO v_seq
    FROM vouchers
    WHERE voucher_no LIKE v_prefix || v_year_month || '-%';
    
    -- 生成凭证号
    v_voucher_no := v_prefix || v_year_month || '-' || LPAD(v_seq::TEXT, 4, '0');
    
    RETURN v_voucher_no;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION generate_voucher_no IS '生成凭证编号（按月连续）';

-- ============================================
-- 迁移完成
-- ============================================
