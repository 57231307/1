-- ========================================
-- 秉羲 ERP 系统 - 供应商管理模块数据库迁移
-- 版本：2026-03-15
-- 模块：供应商管理（Supply Chain - Supplier）
-- 说明：创建供应商管理相关的所有表、索引、触发器
-- ========================================

-- ========================================
-- 1. 供应商基础表
-- ========================================

-- ==================== 供应商表 ====================
CREATE TABLE IF NOT EXISTS suppliers (
    id SERIAL PRIMARY KEY,
    supplier_code VARCHAR(50) NOT NULL,                    -- 供应商编码
    supplier_name VARCHAR(200) NOT NULL,                   -- 供应商名称
    supplier_short_name VARCHAR(100) NOT NULL,             -- 供应商简称
    supplier_type VARCHAR(50) NOT NULL,                    -- 供应商类型
    credit_code VARCHAR(50) NOT NULL,                      -- 统一社会信用代码
    registered_address VARCHAR(500) NOT NULL,              -- 注册地址
    business_address VARCHAR(500),                         -- 经营地址
    legal_representative VARCHAR(50) NOT NULL,             -- 法人代表
    registered_capital DECIMAL(15,2) NOT NULL,             -- 注册资本（万元）
    establishment_date DATE NOT NULL,                      -- 成立日期
    business_term VARCHAR(100),                            -- 营业期限
    business_scope TEXT,                                   -- 经营范围
    taxpayer_type VARCHAR(50) NOT NULL,                    -- 纳税人类型
    bank_name VARCHAR(100) NOT NULL,                       -- 开户银行
    bank_account VARCHAR(50) NOT NULL,                     -- 银行账号
    contact_phone VARCHAR(50) NOT NULL,                    -- 联系电话
    fax VARCHAR(50),                                       -- 传真
    website VARCHAR(200),                                  -- 公司网址
    email VARCHAR(100),                                    -- 联系邮箱
    main_business VARCHAR(500),                            -- 主营业务
    main_market VARCHAR(500),                              -- 主要市场
    employee_count INTEGER,                                -- 员工人数
    annual_revenue DECIMAL(15,2),                          -- 年营业额（万元）
    
    -- 等级管理
    grade VARCHAR(10) DEFAULT 'B',                         -- 供应商等级（A/B/C/D）
    grade_score DECIMAL(5,2) DEFAULT 0.00,                 -- 等级评分
    last_evaluation_date DATE,                             -- 最后评估日期
    
    -- 状态管理
    status VARCHAR(20) DEFAULT 'active',                   -- 状态：active/inactive/disabled/blacklisted
    is_enabled BOOLEAN DEFAULT TRUE,                       -- 是否启用
    
    -- 辅助核算字段（面料行业特色）
    assist_batch BOOLEAN DEFAULT FALSE,                    -- 是否启用批次核算
    assist_supplier BOOLEAN DEFAULT TRUE,                  -- 是否启用供应商核算
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                    -- 创建人 ID
    updated_by INTEGER,                                    -- 更新人 ID
    remarks TEXT                                           -- 备注
);

COMMENT ON TABLE suppliers IS '供应商信息表';
COMMENT ON COLUMN suppliers.supplier_code IS '供应商编码（自动生成）';
COMMENT ON COLUMN suppliers.supplier_name IS '供应商名称';
COMMENT ON COLUMN suppliers.supplier_type IS '供应商类型（fabric/dye/auxiliary/logistics/service/other）';
COMMENT ON COLUMN suppliers.credit_code IS '统一社会信用代码';
COMMENT ON COLUMN suppliers.grade IS '供应商等级（A/B/C/D）';
COMMENT ON COLUMN suppliers.status IS '供应商状态（active/inactive/disabled/blacklisted）';
COMMENT ON COLUMN suppliers.assist_batch IS '是否启用批次核算';
COMMENT ON COLUMN suppliers.assist_supplier IS '是否启用供应商核算';

-- 唯一约束
ALTER TABLE suppliers ADD CONSTRAINT uk_suppliers_code UNIQUE (supplier_code);
ALTER TABLE suppliers ADD CONSTRAINT uk_suppliers_name UNIQUE (supplier_name);
ALTER TABLE suppliers ADD CONSTRAINT uk_suppliers_credit_code UNIQUE (credit_code);

-- 索引
CREATE INDEX IF NOT EXISTS idx_suppliers_type ON suppliers(supplier_type);
CREATE INDEX IF NOT EXISTS idx_suppliers_grade ON suppliers(grade);
CREATE INDEX IF NOT EXISTS idx_suppliers_status ON suppliers(status);
CREATE INDEX IF NOT EXISTS idx_suppliers_credit_code ON suppliers(credit_code);
CREATE INDEX IF NOT EXISTS idx_suppliers_enabled ON suppliers(is_enabled);
CREATE INDEX IF NOT EXISTS idx_suppliers_created_at ON suppliers(created_at DESC);

-- ========================================
-- 2. 供应商联系人表
-- ========================================

-- ==================== 供应商联系人表 ====================
CREATE TABLE IF NOT EXISTS supplier_contacts (
    id SERIAL PRIMARY KEY,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id) ON DELETE CASCADE,
    contact_name VARCHAR(50) NOT NULL,                 -- 联系人姓名
    department VARCHAR(50),                            -- 所属部门
    position VARCHAR(50),                              -- 职位
    mobile_phone VARCHAR(20) NOT NULL,                 -- 手机号码
    tel_phone VARCHAR(50),                             -- 联系电话
    email VARCHAR(100),                                -- 联系邮箱
    wechat VARCHAR(50),                                -- 微信
    qq VARCHAR(20),                                    -- QQ
    is_primary BOOLEAN DEFAULT FALSE,                  -- 是否主要联系人
    remarks TEXT,                                      -- 备注
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE supplier_contacts IS '供应商联系人表';
COMMENT ON COLUMN supplier_contacts.is_primary IS '是否主要联系人';

-- 索引
CREATE INDEX IF NOT EXISTS idx_supplier_contacts_supplier_id ON supplier_contacts(supplier_id);
CREATE INDEX IF NOT EXISTS idx_supplier_contacts_mobile ON supplier_contacts(mobile_phone);

-- ========================================
-- 3. 供应商分类表
-- ========================================

-- ==================== 供应商分类表 ====================
CREATE TABLE IF NOT EXISTS supplier_categories (
    id SERIAL PRIMARY KEY,
    category_code VARCHAR(50) NOT NULL,                -- 分类编码
    category_name VARCHAR(100) NOT NULL,               -- 分类名称
    parent_id INTEGER REFERENCES supplier_categories(id), -- 父级分类 ID
    level INTEGER NOT NULL DEFAULT 1,                  -- 层级
    sort_order INTEGER NOT NULL DEFAULT 0,             -- 排序
    is_enabled BOOLEAN DEFAULT TRUE,                   -- 是否启用
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE supplier_categories IS '供应商分类表';
COMMENT ON COLUMN supplier_categories.level IS '分类层级（1-3）';

-- 唯一约束
ALTER TABLE supplier_categories ADD CONSTRAINT uk_categories_code UNIQUE (category_code);

-- 索引
CREATE INDEX IF NOT EXISTS idx_supplier_categories_parent ON supplier_categories(parent_id);
CREATE INDEX IF NOT EXISTS idx_supplier_categories_level ON supplier_categories(level);

-- ========================================
-- 4. 供应商等级表
-- ========================================

-- ==================== 供应商等级表 ====================
CREATE TABLE IF NOT EXISTS supplier_grades (
    id SERIAL PRIMARY KEY,
    grade_code VARCHAR(10) NOT NULL,                   -- 等级编码（A/B/C/D）
    grade_name VARCHAR(50) NOT NULL,                   -- 等级名称
    min_score DECIMAL(5,2) NOT NULL,                   -- 最低分数
    max_score DECIMAL(5,2) NOT NULL,                   -- 最高分数
    color_code VARCHAR(20),                            -- 颜色标识
    permission_desc TEXT,                              -- 权限说明
    is_enabled BOOLEAN DEFAULT TRUE,                   -- 是否启用
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE supplier_grades IS '供应商等级表';

-- 唯一约束
ALTER TABLE supplier_grades ADD CONSTRAINT uk_grades_code UNIQUE (grade_code);

-- 初始化数据
INSERT INTO supplier_grades (grade_code, grade_name, min_score, max_score, color_code, permission_desc) VALUES
('A', '战略供应商', 90.00, 100.00, 'green', '优先采购、免检、月结'),
('B', '合格供应商', 75.00, 89.99, 'blue', '正常采购、抽检、月结'),
('C', '考察供应商', 60.00, 74.99, 'yellow', '限制采购、全检、现结'),
('D', '不合格供应商', 0.00, 59.99, 'red', '暂停采购、列入黑名单');

-- ========================================
-- 5. 供应商评估表
-- ========================================

-- ==================== 供应商评估表 ====================
CREATE TABLE IF NOT EXISTS supplier_evaluations (
    id SERIAL PRIMARY KEY,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id),
    evaluation_year INTEGER NOT NULL,                  -- 评估年份
    evaluation_quarter INTEGER NOT NULL,               -- 评估季度（1-4）
    
    -- 质量水平（35 分）
    quality_pass_rate DECIMAL(5,2),                    -- 来料合格率（%）
    quality_score DECIMAL(5,2),                        -- 质量得分
    quality_incidents INTEGER DEFAULT 0,               -- 质量事故次数
    quality_incident_score DECIMAL(5,2),               -- 质量事故得分
    quality_improvement_score DECIMAL(5,2),            -- 质量改进得分
    
    -- 交货能力（25 分）
    delivery_on_time_rate DECIMAL(5,2),                -- 交货及时率（%）
    delivery_score DECIMAL(5,2),                       -- 交货得分
    order_completion_rate DECIMAL(5,2),                -- 订单完成率（%）
    order_completion_score DECIMAL(5,2),               -- 订单完成得分
    
    -- 价格水平（20 分）
    price_competitiveness_score DECIMAL(5,2),          -- 价格竞争力得分
    price_stability_score DECIMAL(5,2),                -- 价格稳定性得分
    
    -- 服务水平（15 分）
    response_speed_score DECIMAL(5,2),                 -- 响应速度得分
    after_sales_score DECIMAL(5,2),                    -- 售后服务得分
    cooperation_score DECIMAL(5,2),                    -- 配合度得分
    
    -- 技术能力（5 分）
    rd_capability_score DECIMAL(5,2),                  -- 研发能力得分
    technical_support_score DECIMAL(5,2),              -- 技术支持得分
    
    -- 总分和等级
    total_score DECIMAL(5,2),                          -- 总分
    grade_before VARCHAR(10),                          -- 评估前等级
    grade_after VARCHAR(10),                           -- 评估后等级
    
    -- 审批
    evaluator_id INTEGER,                              -- 评估人 ID
    approver_id INTEGER,                               -- 审批人 ID
    approval_status VARCHAR(20) DEFAULT 'pending',     -- 审批状态
    approval_date DATE,                                -- 审批日期
    approval_remarks TEXT,                             -- 审批意见
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,
    updated_by INTEGER,
    
    -- 约束
    CONSTRAINT chk_quarter CHECK (evaluation_quarter BETWEEN 1 AND 4)
);

COMMENT ON TABLE supplier_evaluations IS '供应商评估表';
COMMENT ON COLUMN supplier_evaluations.quality_pass_rate IS '来料合格率（%）';
COMMENT ON COLUMN supplier_evaluations.delivery_on_time_rate IS '交货及时率（%）';
COMMENT ON COLUMN supplier_evaluations.order_completion_rate IS '订单完成率（%）';

-- 唯一约束
ALTER TABLE supplier_evaluations ADD CONSTRAINT uk_supplier_quarter UNIQUE (supplier_id, evaluation_year, evaluation_quarter);

-- 索引
CREATE INDEX IF NOT EXISTS idx_supplier_evaluations_supplier ON supplier_evaluations(supplier_id);
CREATE INDEX IF NOT EXISTS idx_supplier_evaluations_year_quarter ON supplier_evaluations(evaluation_year, evaluation_quarter);
CREATE INDEX IF NOT EXISTS idx_supplier_evaluations_total_score ON supplier_evaluations(total_score);

-- ========================================
-- 6. 供应商资质表
-- ========================================

-- ==================== 供应商资质表 ====================
CREATE TABLE IF NOT EXISTS supplier_qualifications (
    id SERIAL PRIMARY KEY,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id) ON DELETE CASCADE,
    qualification_name VARCHAR(200) NOT NULL,          -- 资质名称
    qualification_type VARCHAR(50) NOT NULL,           -- 资质类型
    qualification_no VARCHAR(100) NOT NULL,            -- 资质编号
    issuing_authority VARCHAR(200) NOT NULL,           -- 发证机构
    issue_date DATE NOT NULL,                          -- 发证日期
    valid_until DATE NOT NULL,                         -- 有效期至
    attachment_path VARCHAR(500),                      -- 附件路径
    need_annual_check BOOLEAN DEFAULT FALSE,           -- 是否年检
    annual_check_record TEXT,                          -- 年检记录
    is_expired BOOLEAN DEFAULT FALSE,                  -- 是否过期
    remarks TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE supplier_qualifications IS '供应商资质表';
COMMENT ON COLUMN supplier_qualifications.qualification_type IS '资质类型（营业执照/税务登记证/组织机构代码证/ISO9001 等）';

-- 索引
CREATE INDEX IF NOT EXISTS idx_supplier_qualifications_supplier ON supplier_qualifications(supplier_id);
CREATE INDEX IF NOT EXISTS idx_supplier_qualifications_type ON supplier_qualifications(qualification_type);
CREATE INDEX IF NOT EXISTS idx_supplier_qualifications_valid_until ON supplier_qualifications(valid_until);

-- ========================================
-- 7. 供应商黑名单表
-- ========================================

-- ==================== 供应商黑名单表 ====================
CREATE TABLE IF NOT EXISTS supplier_blacklists (
    id SERIAL PRIMARY KEY,
    supplier_id INTEGER NOT NULL UNIQUE REFERENCES suppliers(id),
    blacklist_date DATE NOT NULL,                      -- 列入日期
    blacklist_reason VARCHAR(50) NOT NULL,             -- 列入原因
    detail_description TEXT NOT NULL,                  -- 详细说明
    evidence TEXT,                                     -- 证据材料
    approver_id INTEGER NOT NULL,                      -- 审批人 ID
    approval_date DATE NOT NULL,                       -- 审批日期
    is_permanent BOOLEAN DEFAULT FALSE,                -- 是否永久
    release_date DATE,                                 -- 解禁日期
    release_condition TEXT,                            -- 解禁条件
    release_status VARCHAR(20) DEFAULT 'blacklisted',  -- 解禁状态
    release_date_actual DATE,                          -- 实际解禁日期
    remarks TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,
    updated_by INTEGER
);

COMMENT ON TABLE supplier_blacklists IS '供应商黑名单表';
COMMENT ON COLUMN supplier_blacklists.blacklist_reason IS '列入原因（质量事故/欺诈/违约/贿赂等）';
COMMENT ON COLUMN supplier_blacklists.release_status IS '解禁状态（blacklisted/released）';

-- 索引
CREATE INDEX IF NOT EXISTS idx_supplier_blacklists_supplier ON supplier_blacklists(supplier_id);
CREATE INDEX IF NOT EXISTS idx_supplier_blacklists_date ON supplier_blacklists(blacklist_date);
CREATE INDEX IF NOT EXISTS idx_supplier_blacklists_status ON supplier_blacklists(release_status);

-- ========================================
-- 8. 触发器函数
-- ========================================

-- ==================== 自动更新 updated_at ====================
CREATE OR REPLACE FUNCTION update_supplier_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ==================== 资质过期检查 ====================
CREATE OR REPLACE FUNCTION update_qualification_expired()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.valid_until < CURRENT_DATE THEN
        NEW.is_expired := TRUE;
    ELSE
        NEW.is_expired := FALSE;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ========================================
-- 9. 应用触发器
-- ========================================

-- 更新 updated_at 触发器
DROP TRIGGER IF EXISTS trg_suppliers_updated_at ON suppliers;
CREATE TRIGGER trg_suppliers_updated_at
    BEFORE UPDATE ON suppliers
    FOR EACH ROW
    EXECUTE FUNCTION update_supplier_updated_at_column();

DROP TRIGGER IF EXISTS trg_supplier_contacts_updated_at ON supplier_contacts;
CREATE TRIGGER trg_supplier_contacts_updated_at
    BEFORE UPDATE ON supplier_contacts
    FOR EACH ROW
    EXECUTE FUNCTION update_supplier_updated_at_column();

DROP TRIGGER IF EXISTS trg_supplier_categories_updated_at ON supplier_categories;
CREATE TRIGGER trg_supplier_categories_updated_at
    BEFORE UPDATE ON supplier_categories
    FOR EACH ROW
    EXECUTE FUNCTION update_supplier_updated_at_column();

DROP TRIGGER IF EXISTS trg_supplier_grades_updated_at ON supplier_grades;
CREATE TRIGGER trg_supplier_grades_updated_at
    BEFORE UPDATE ON supplier_grades
    FOR EACH ROW
    EXECUTE FUNCTION update_supplier_updated_at_column();

DROP TRIGGER IF EXISTS trg_supplier_evaluations_updated_at ON supplier_evaluations;
CREATE TRIGGER trg_supplier_evaluations_updated_at
    BEFORE UPDATE ON supplier_evaluations
    FOR EACH ROW
    EXECUTE FUNCTION update_supplier_updated_at_column();

DROP TRIGGER IF EXISTS trg_supplier_qualifications_updated_at ON supplier_qualifications;
CREATE TRIGGER trg_supplier_qualifications_updated_at
    BEFORE UPDATE ON supplier_qualifications
    FOR EACH ROW
    EXECUTE FUNCTION update_supplier_updated_at_column();

DROP TRIGGER IF EXISTS trg_supplier_blacklists_updated_at ON supplier_blacklists;
CREATE TRIGGER trg_supplier_blacklists_updated_at
    BEFORE UPDATE ON supplier_blacklists
    FOR EACH ROW
    EXECUTE FUNCTION update_supplier_updated_at_column();

-- 资质过期检查触发器
DROP TRIGGER IF EXISTS trg_supplier_qualifications_check_expired ON supplier_qualifications;
CREATE TRIGGER trg_supplier_qualifications_check_expired
    BEFORE INSERT OR UPDATE ON supplier_qualifications
    FOR EACH ROW
    EXECUTE FUNCTION update_qualification_expired();

-- ========================================
-- 迁移完成
-- ========================================
