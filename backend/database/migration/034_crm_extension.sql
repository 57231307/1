-- ========================================
-- 秉羲 ERP 系统 - CRM 客户管理扩展数据库迁移
-- 版本：2026-03-16
-- 模块：CRM 客户管理扩展
-- 说明：创建 CRM 扩展相关的所有表、索引
-- ========================================

-- ========================================
-- 1. 销售线索表
-- ========================================

-- ==================== 销售线索表 ====================
CREATE TABLE crm_lead (
    id SERIAL PRIMARY KEY,
    lead_no VARCHAR(100) NOT NULL UNIQUE,                -- 线索编号
    lead_source VARCHAR(50) NOT NULL,                    -- 线索来源（website/referral/exhibition/cold_call/other）
    lead_status VARCHAR(20) DEFAULT 'new',               -- 线索状态（new/contacted/qualified/converted/lost）
    
    -- 客户信息
    company_name VARCHAR(200),                           -- 公司名称
    contact_name VARCHAR(100) NOT NULL,                  -- 联系人姓名
    contact_title VARCHAR(100),                          -- 联系人职位
    mobile_phone VARCHAR(20),                            -- 手机号码
    tel_phone VARCHAR(50),                               -- 联系电话
    email VARCHAR(100),                                  -- 联系邮箱
    wechat VARCHAR(50),                                  -- 微信
    qq VARCHAR(20),                                      -- QQ
    address TEXT,                                        -- 地址
    
    -- 需求信息
    product_interest TEXT,                               -- 意向产品
    estimated_quantity DECIMAL(10,2),                    -- 预计数量
    estimated_amount DECIMAL(10,2),                      -- 预计金额
    expected_delivery_date DATE,                         -- 期望交货日期
    requirement_desc TEXT,                               -- 需求描述
    
    -- 跟进信息
    owner_id INTEGER NOT NULL,                           -- 负责人 ID
    owner_name VARCHAR(100) NOT NULL,                    -- 负责人姓名
    last_follow_up_date DATE,                            -- 最后跟进日期
    next_follow_up_date DATE,                            -- 下次跟进日期
    follow_up_plan TEXT,                                 -- 跟进计划
    
    -- 转化信息
    converted_at TIMESTAMP,                              -- 转化时间
    converted_customer_id INTEGER,                       -- 转化后客户 ID
    converted_opportunity_id INTEGER,                    -- 转化后商机 ID
    lost_reason VARCHAR(200),                            -- 丢失原因
    
    -- 系统字段
    priority VARCHAR(20) DEFAULT 'medium',               -- 优先级（low/medium/high/urgent）
    rating INTEGER DEFAULT 0,                            -- 评级（0-5）
    tags TEXT[],                                         -- 标签列表
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE crm_lead IS '销售线索表';
COMMENT ON COLUMN crm_lead.lead_source IS '线索来源（website/referral/exhibition/cold_call/other）';
COMMENT ON COLUMN crm_lead.lead_status IS '线索状态（new/contacted/qualified/converted/lost）';

-- 外键约束
ALTER TABLE crm_lead ADD CONSTRAINT fk_crm_lead_owner
    FOREIGN KEY (owner_id) REFERENCES users(id);
ALTER TABLE crm_lead ADD CONSTRAINT fk_crm_lead_converted_customer
    FOREIGN KEY (converted_customer_id) REFERENCES customers(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_crm_lead_lead_no ON crm_lead(lead_no);
CREATE INDEX IF NOT EXISTS idx_crm_lead_source ON crm_lead(lead_source);
CREATE INDEX IF NOT EXISTS idx_crm_lead_status ON crm_lead(lead_status);
CREATE INDEX IF NOT EXISTS idx_crm_lead_owner ON crm_lead(owner_id);
CREATE INDEX IF NOT EXISTS idx_crm_lead_created_at ON crm_lead(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_crm_lead_priority ON crm_lead(priority);

-- ========================================
-- 2. 商机表
-- ========================================

-- ==================== 商机表 ====================
CREATE TABLE crm_opportunity (
    id SERIAL PRIMARY KEY,
    opportunity_no VARCHAR(100) NOT NULL UNIQUE,         -- 商机编号
    opportunity_name VARCHAR(500) NOT NULL,              -- 商机名称
    customer_id INTEGER NOT NULL,                        -- 客户 ID
    lead_id INTEGER,                                     -- 来源线索 ID
    
    -- 商机信息
    opportunity_type VARCHAR(50),                        -- 商机类型（new_business/existing_business/upsell）
    opportunity_stage VARCHAR(50) DEFAULT 'prospecting', -- 商机阶段
    win_probability DECIMAL(5,2) DEFAULT 0.00,           -- 成功概率（%）
    
    -- 金额信息
    estimated_amount DECIMAL(10,2),                      -- 预计金额
    actual_amount DECIMAL(10,2),                         -- 实际金额
    currency VARCHAR(10) DEFAULT 'CNY',                  -- 币种
    
    -- 时间信息
    expected_close_date DATE,                            -- 预计成交日期
    actual_close_date DATE,                              -- 实际成交日期
    
    -- 产品信息
    product_ids INTEGER[],                               -- 产品 ID 列表
    product_names VARCHAR(200)[],                        -- 产品名称列表
    product_desc TEXT,                                   -- 产品描述
    
    -- 跟进信息
    owner_id INTEGER NOT NULL,                           -- 负责人 ID
    owner_name VARCHAR(100) NOT NULL,                    -- 负责人姓名
    last_follow_up_date DATE,                            -- 最后跟进日期
    next_follow_up_date DATE,                            -- 下次跟进日期
    follow_up_plan TEXT,                                 -- 跟进计划
    
    -- 竞争对手
    competitor_names TEXT[],                             -- 竞争对手列表
    competitive_advantage TEXT,                          -- 竞争优势
    
    -- 状态信息
    opportunity_status VARCHAR(20) DEFAULT 'open',       -- 状态（open/won/lost/cancelled）
    won_reason TEXT,                                     -- 赢单原因
    lost_reason TEXT,                                    -- 丢单原因
    
    -- 系统字段
    priority VARCHAR(20) DEFAULT 'medium',               -- 优先级
    rating INTEGER DEFAULT 0,                            -- 评级（0-5）
    tags TEXT[],                                         -- 标签列表
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE crm_opportunity IS '商机表';
COMMENT ON COLUMN crm_opportunity.opportunity_stage IS '商机阶段';
COMMENT ON COLUMN crm_opportunity.win_probability IS '成功概率（%）';
COMMENT ON COLUMN crm_opportunity.opportunity_status IS '状态（open/won/lost/cancelled）';

-- 外键约束
ALTER TABLE crm_opportunity ADD CONSTRAINT fk_crm_opp_customer
    FOREIGN KEY (customer_id) REFERENCES customers(id);
ALTER TABLE crm_opportunity ADD CONSTRAINT fk_crm_opp_lead
    FOREIGN KEY (lead_id) REFERENCES crm_lead(id);
ALTER TABLE crm_opportunity ADD CONSTRAINT fk_crm_opp_owner
    FOREIGN KEY (owner_id) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_crm_opp_opportunity_no ON crm_opportunity(opportunity_no);
CREATE INDEX IF NOT EXISTS idx_crm_opp_customer ON crm_opportunity(customer_id);
CREATE INDEX IF NOT EXISTS idx_crm_opp_lead ON crm_opportunity(lead_id);
CREATE INDEX IF NOT EXISTS idx_crm_opp_stage ON crm_opportunity(opportunity_stage);
CREATE INDEX IF NOT EXISTS idx_crm_opp_status ON crm_opportunity(opportunity_status);
CREATE INDEX IF NOT EXISTS idx_crm_opp_owner ON crm_opportunity(owner_id);
CREATE INDEX IF NOT EXISTS idx_crm_opp_expected_close ON crm_opportunity(expected_close_date);

-- ========================================
-- 3. 客户跟进记录表
-- ========================================

-- ==================== 客户跟进记录表 ====================
CREATE TABLE crm_follow_up (
    id SERIAL PRIMARY KEY,
    follow_up_no VARCHAR(100) NOT NULL UNIQUE,           -- 跟进记录编号
    
    -- 关联信息
    lead_id INTEGER,                                     -- 线索 ID
    opportunity_id INTEGER,                              -- 商机 ID
    customer_id INTEGER,                                 -- 客户 ID
    
    -- 跟进信息
    follow_up_type VARCHAR(50) NOT NULL,                 -- 跟进类型（phone_call/meeting/email/wechat/other）
    follow_up_date DATE NOT NULL,                        -- 跟进日期
    follow_up_time TIME,                                 -- 跟进时间
    duration_minutes INTEGER,                            -- 时长（分钟）
    
    -- 跟进内容
    subject VARCHAR(500),                                -- 主题
    content TEXT NOT NULL,                               -- 跟进内容
    summary TEXT,                                        -- 跟进总结
    feedback TEXT,                                       -- 客户反馈
    
    -- 下一步计划
    next_step TEXT,                                      -- 下一步计划
    next_follow_up_date DATE,                            -- 下次跟进日期
    
    -- 附件
    attachment_urls TEXT[],                              -- 附件 URL 列表
    
    -- 系统字段
    owner_id INTEGER NOT NULL,                           -- 负责人 ID
    owner_name VARCHAR(100) NOT NULL,                    -- 负责人姓名
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE crm_follow_up IS '客户跟进记录表';
COMMENT ON COLUMN crm_follow_up.follow_up_type IS '跟进类型（phone_call/meeting/email/wechat/other）';

-- 外键约束
ALTER TABLE crm_follow_up ADD CONSTRAINT fk_crm_fu_lead
    FOREIGN KEY (lead_id) REFERENCES crm_lead(id);
ALTER TABLE crm_follow_up ADD CONSTRAINT fk_crm_fu_opportunity
    FOREIGN KEY (opportunity_id) REFERENCES crm_opportunity(id);
ALTER TABLE crm_follow_up ADD CONSTRAINT fk_crm_fu_customer
    FOREIGN KEY (customer_id) REFERENCES customers(id);
ALTER TABLE crm_follow_up ADD CONSTRAINT fk_crm_fu_owner
    FOREIGN KEY (owner_id) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_crm_fu_lead ON crm_follow_up(lead_id);
CREATE INDEX IF NOT EXISTS idx_crm_fu_opportunity ON crm_follow_up(opportunity_id);
CREATE INDEX IF NOT EXISTS idx_crm_fu_customer ON crm_follow_up(customer_id);
CREATE INDEX IF NOT EXISTS idx_crm_fu_owner ON crm_follow_up(owner_id);
CREATE INDEX IF NOT EXISTS idx_crm_fu_date ON crm_follow_up(follow_up_date DESC);
CREATE INDEX IF NOT EXISTS idx_crm_fu_type ON crm_follow_up(follow_up_type);

-- ========================================
-- 4. 客户联系人表（扩展）
-- ========================================

-- ==================== 客户联系人表（如果不存在则创建） ====================
CREATE TABLE IF NOT EXISTS crm_contact (
    id SERIAL PRIMARY KEY,
    customer_id INTEGER NOT NULL,                        -- 客户 ID
    contact_name VARCHAR(100) NOT NULL,                  -- 联系人姓名
    contact_title VARCHAR(100),                          -- 职位
    department VARCHAR(100),                             -- 部门
    mobile_phone VARCHAR(20),                            -- 手机号码
    tel_phone VARCHAR(50),                               -- 联系电话
    email VARCHAR(100),                                  -- 联系邮箱
    wechat VARCHAR(50),                                  -- 微信
    qq VARCHAR(20),                                      -- QQ
    is_primary BOOLEAN DEFAULT FALSE,                    -- 是否主要联系人
    is_decision_maker BOOLEAN DEFAULT FALSE,             -- 是否决策者
    contact_preference VARCHAR(50),                      -- 联系偏好
    birthday DATE,                                       -- 生日
    remarks TEXT,                                        -- 备注
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE crm_contact IS '客户联系人表';
COMMENT ON COLUMN crm_contact.is_decision_maker IS '是否决策者';

-- 外键约束
ALTER TABLE crm_contact ADD CONSTRAINT fk_crm_contact_customer
    FOREIGN KEY (customer_id) REFERENCES customers(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_crm_contact_customer ON crm_contact(customer_id);
CREATE INDEX IF NOT EXISTS idx_crm_contact_mobile ON crm_contact(mobile_phone);
CREATE INDEX IF NOT EXISTS idx_crm_contact_email ON crm_contact(email);

-- ========================================
-- 5. 客户公海表
-- ========================================

-- ==================== 客户公海表 ====================
CREATE TABLE crm_customer_sea (
    id SERIAL PRIMARY KEY,
    customer_id INTEGER NOT NULL,                        -- 客户 ID
    reason_type VARCHAR(50) NOT NULL,                    -- 进入公海原因（no_follow_up/active_release/passive_release）
    reason_detail TEXT,                                  -- 原因详情
    
    -- 时间信息
    released_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 释放时间
    released_by INTEGER NOT NULL,                        -- 释放人 ID
    released_by_name VARCHAR(100) NOT NULL,              -- 释放人姓名
    
    -- 领取信息
    claimed_at TIMESTAMP,                                -- 领取时间
    claimed_by INTEGER,                                  -- 领取人 ID
    claimed_by_name VARCHAR(100),                        -- 领取人姓名
    
    -- 系统字段
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否有效
    priority INTEGER DEFAULT 0,                          -- 优先级
    remarks TEXT,                                        -- 备注
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE crm_customer_sea IS '客户公海表';
COMMENT ON COLUMN crm_customer_sea.reason_type IS '进入公海原因（no_follow_up/active_release/passive_release）';

-- 外键约束
ALTER TABLE crm_customer_sea ADD CONSTRAINT fk_crm_cs_customer
    FOREIGN KEY (customer_id) REFERENCES customers(id);
ALTER TABLE crm_customer_sea ADD CONSTRAINT fk_crm_cs_released_by
    FOREIGN KEY (released_by) REFERENCES users(id);
ALTER TABLE crm_customer_sea ADD CONSTRAINT fk_crm_cs_claimed_by
    FOREIGN KEY (claimed_by) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_crm_cs_customer ON crm_customer_sea(customer_id);
CREATE INDEX IF NOT EXISTS idx_crm_cs_released_at ON crm_customer_sea(released_at DESC);
CREATE INDEX IF NOT EXISTS idx_crm_cs_active ON crm_customer_sea(is_active);

-- ========================================
-- 6. 销售漏斗配置表
-- ========================================

-- ==================== 销售漏斗配置表 ====================
CREATE TABLE crm_sales_funnel_config (
    id SERIAL PRIMARY KEY,
    funnel_name VARCHAR(200) NOT NULL,                   -- 漏斗名称
    funnel_type VARCHAR(50) NOT NULL,                    -- 漏斗类型（standard/custom）
    
    -- 阶段配置
    stages JSONB NOT NULL,                               -- 阶段配置（JSON 数组）
    
    -- 系统字段
    is_default BOOLEAN DEFAULT FALSE,                    -- 是否默认漏斗
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否启用
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE crm_sales_funnel_config IS '销售漏斗配置表';
COMMENT ON COLUMN crm_sales_funnel_config.stages IS '阶段配置（JSON 数组）';

-- 索引
CREATE INDEX IF NOT EXISTS idx_crm_sfc_type ON crm_sales_funnel_config(funnel_type);
CREATE INDEX IF NOT EXISTS idx_crm_sfc_active ON crm_sales_funnel_config(is_active);

-- ========================================
-- 7. 触发器函数
-- ========================================

-- ==================== 自动更新 updated_at ====================
CREATE OR REPLACE FUNCTION update_crm_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ========================================
-- 8. 应用触发器
-- ========================================

-- 更新 updated_at 触发器
DROP TRIGGER IF EXISTS trg_crm_lead_updated_at ON crm_lead;
CREATE TRIGGER trg_crm_lead_updated_at
    BEFORE UPDATE ON crm_lead
    FOR EACH ROW
    EXECUTE FUNCTION update_crm_updated_at_column();

DROP TRIGGER IF EXISTS trg_crm_opportunity_updated_at ON crm_opportunity;
CREATE TRIGGER trg_crm_opportunity_updated_at
    BEFORE UPDATE ON crm_opportunity
    FOR EACH ROW
    EXECUTE FUNCTION update_crm_updated_at_column();

DROP TRIGGER IF EXISTS trg_crm_follow_up_updated_at ON crm_follow_up;
CREATE TRIGGER trg_crm_follow_up_updated_at
    BEFORE UPDATE ON crm_follow_up
    FOR EACH ROW
    EXECUTE FUNCTION update_crm_updated_at_column();

DROP TRIGGER IF EXISTS trg_crm_contact_updated_at ON crm_contact;
CREATE TRIGGER trg_crm_contact_updated_at
    BEFORE UPDATE ON crm_contact
    FOR EACH ROW
    EXECUTE FUNCTION update_crm_updated_at_column();

DROP TRIGGER IF EXISTS trg_crm_sales_funnel_updated_at ON crm_sales_funnel_config;
CREATE TRIGGER trg_crm_sales_funnel_updated_at
    BEFORE UPDATE ON crm_sales_funnel_config
    FOR EACH ROW
    EXECUTE FUNCTION update_crm_updated_at_column();

-- ========================================
-- 9. 初始化数据
-- ========================================

-- 初始化销售漏斗默认配置
INSERT INTO crm_sales_funnel_config (funnel_name, funnel_type, stages, is_default) VALUES
('标准销售漏斗', 'standard', 
 '[
    {"stage": "prospecting", "name": "初步接触", "probability": 10},
    {"stage": "qualification", "name": "需求确认", "probability": 30},
    {"stage": "proposal", "name": "方案报价", "probability": 50},
    {"stage": "negotiation", "name": "商务谈判", "probability": 70},
    {"stage": "closing", "name": "成交", "probability": 90}
  ]'::JSONB,
 TRUE);

-- ========================================
-- 迁移完成
-- ========================================
