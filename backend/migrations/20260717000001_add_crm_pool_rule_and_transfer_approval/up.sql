-- V15 P0-S08 修复：CRM 数据权限完全缺失
--
-- 本迁移补齐 P0-S08 修复方案中缺失的三类基础结构：
-- 1. customers 主表新增 owner_id（业务负责人），实现客户主数据的行级数据权限归属
-- 2. 创建 customer_pool_rules 表：公海规则配置（保护期/领取上限/最大持有数）
-- 3. 创建 customer_transfer_approvals 表：客户转移审批流（多级审批 + 大客户额外审批）
--
-- 设计原则：
-- - 现有 crm_lead.owner_id 已实现线索归属，customers 主表无 owner_id 字段，本迁移补齐
-- - 现有 crm_recycle_rules 表只覆盖"未跟进天数回收"，本迁移新增的 customer_pool_rules
--   覆盖"保护期/领取上限/最大持有数"，与 recycle_rules 互补不重叠
-- - 转移审批表独立于 assignment_history（历史流水），审批表用于流程状态机
--
-- 关联文件：
-- - backend/src/models/customer_pool_rule.rs
-- - backend/src/models/customer_transfer_approval.rs
-- - backend/src/models/customer.rs（新增 owner_id 字段）
-- - backend/src/services/crm/customer_transfer_approval_service.rs
-- - backend/src/services/crm/pool.rs（claim 注入规则校验）
-- - backend/src/services/crm/assign.rs（transfer_lead 触发审批流）

-- =====================================================
-- 1. customers 主表新增 owner_id（业务负责人）
-- =====================================================
-- 客户主数据归属人，用于客户层面的行级数据权限过滤
-- 默认 0 表示未分配（公海客户），与 crm_lead.owner_id 语义一致
ALTER TABLE customers ADD COLUMN IF NOT EXISTS owner_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE customers ADD COLUMN IF NOT EXISTS owner_assigned_at TIMESTAMPTZ;

-- 索引：加速 owner_id 过滤（数据权限核心列）
CREATE INDEX IF NOT EXISTS idx_customers_owner_id ON customers (owner_id);

-- 字段注释
COMMENT ON COLUMN customers.owner_id IS '业务负责人用户 ID（0 表示未分配/公海客户）';
COMMENT ON COLUMN customers.owner_assigned_at IS '业务负责人分配时间（用于公海保护期校验）';

-- 数据迁移：将 customers.created_by 回填到 owner_id（仅当 owner_id=0 时）
-- 保证现有客户有归属人，避免数据权限将所有客户隐藏
UPDATE customers SET owner_id = COALESCE(created_by, 0), owner_assigned_at = created_at
WHERE owner_id = 0;

-- =====================================================
-- 2. 公海规则配置表 customer_pool_rules
-- =====================================================
-- 与 crm_recycle_rules 互补：recycle_rules 管"未跟进天数回收"，
-- pool_rules 管"保护期/领取上限/最大持有数"
CREATE TABLE IF NOT EXISTS customer_pool_rules (
    id SERIAL PRIMARY KEY,
    -- 规则名称（唯一）
    name VARCHAR(100) NOT NULL UNIQUE,
    -- 规则类型：protection_period（保护期）/ claim_limit（领取上限）/ max_holdings（最大持有数）
    rule_type VARCHAR(50) NOT NULL,
    -- 规则数值（天数/次数/数量，按 rule_type 解释）
    rule_value INTEGER NOT NULL CHECK (rule_value >= 0),
    -- 适用客户类型：all/wholesale/retail/vip（all 表示全部）
    customer_type VARCHAR(20) NOT NULL DEFAULT 'all',
    -- 是否启用
    is_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    -- 备注
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- 校验：rule_type 必须在枚举范围内
    CONSTRAINT chk_pool_rule_type CHECK (rule_type IN ('protection_period', 'claim_limit', 'max_holdings')),
    -- 校验：customer_type 必须在枚举范围内
    CONSTRAINT chk_pool_rule_customer_type CHECK (customer_type IN ('all', 'wholesale', 'retail', 'vip'))
);

-- 索引：按启用状态过滤
CREATE INDEX IF NOT EXISTS idx_customer_pool_rules_enabled ON customer_pool_rules (is_enabled);
CREATE INDEX IF NOT EXISTS idx_customer_pool_rules_type ON customer_pool_rules (rule_type);

COMMENT ON TABLE customer_pool_rules IS '公海规则配置表 - 保护期/领取上限/最大持有数';
COMMENT ON COLUMN customer_pool_rules.rule_type IS '规则类型：protection_period=保护期(天), claim_limit=领取上限(次/天), max_holdings=最大持有数';
COMMENT ON COLUMN customer_pool_rules.rule_value IS '规则数值（按 rule_type 解释：天/次/数量）';

-- 初始化默认规则（与现有业务规则保持一致）
INSERT INTO customer_pool_rules (name, rule_type, rule_value, customer_type, notes) VALUES
    ('默认保护期-7天', 'protection_period', 7, 'all', '公海领取后 7 天内不能再被其他人领取'),
    ('默认领取上限-5条/天', 'claim_limit', 5, 'all', '每个销售每天最多从公海领取 5 条线索'),
    ('默认最大持有数-50条', 'max_holdings', 50, 'all', '每个销售最多持有 50 条活跃线索')
ON CONFLICT (name) DO NOTHING;

-- =====================================================
-- 3. 客户转移审批表 customer_transfer_approvals
-- =====================================================
-- 用于客户转移的多级审批流：销售员申请 → 销售经理审批 → 总监审批（大客户额外触发）
-- 与 assignment_history 互补：assignment_history 是流水（仅记录），approvals 是流程（状态机）
CREATE TABLE IF NOT EXISTS customer_transfer_approvals (
    id SERIAL PRIMARY KEY,
    -- 申请编号（唯一）
    approval_no VARCHAR(50) NOT NULL UNIQUE,
    -- 客户/线索 ID（lead_id，与 crm_lead.id 关联）
    lead_id INTEGER NOT NULL,
    -- 客户名称（冗余字段，避免 join 查询）
    company_name VARCHAR(200),
    -- 原归属人 ID
    from_user_id INTEGER NOT NULL,
    -- 原归属人姓名（冗余字段）
    from_user_name VARCHAR(100),
    -- 新归属人 ID
    to_user_id INTEGER NOT NULL,
    -- 新归属人姓名（冗余字段）
    to_user_name VARCHAR(100),
    -- 申请人 ID
    applicant_id INTEGER NOT NULL,
    -- 申请原因（必填）
    reason TEXT NOT NULL,
    -- 是否大客户转移（信用额度超过阈值时自动标记）
    is_large_customer BOOLEAN NOT NULL DEFAULT FALSE,
    -- 审批状态：pending/approved/rejected/cancelled
    approval_status VARCHAR(20) NOT NULL DEFAULT 'pending',
    -- 当前审批层级（1=销售经理，2=总监；大客户需要 2 层）
    current_level INTEGER NOT NULL DEFAULT 1,
    -- 最大审批层级（普通客户 1，大客户 2）
    max_level INTEGER NOT NULL DEFAULT 1,
    -- 销售经理审批人 ID
    manager_approver_id INTEGER,
    -- 销售经理审批意见
    manager_comment TEXT,
    -- 销售经理审批时间
    manager_approved_at TIMESTAMPTZ,
    -- 总监审批人 ID（仅大客户）
    director_approver_id INTEGER,
    -- 总监审批意见
    director_comment TEXT,
    -- 总监审批时间
    director_approved_at TIMESTAMPTZ,
    -- 最终完成时间（审批通过且转移执行完成）
    completed_at TIMESTAMPTZ,
    -- 创建时间
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- 更新时间
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- 校验：approval_status 必须在枚举范围内
    CONSTRAINT chk_transfer_approval_status CHECK (approval_status IN ('pending', 'approved', 'rejected', 'cancelled')),
    -- 校验：current_level 不能超过 max_level
    CONSTRAINT chk_transfer_approval_level CHECK (current_level <= max_level),
    -- 校验：max_level 必须在 1-2 之间（普通 1 层，大客户 2 层）
    CONSTRAINT chk_transfer_approval_max_level CHECK (max_level IN (1, 2))
);

-- 索引：按状态过滤（待审批列表）
CREATE INDEX IF NOT EXISTS idx_transfer_approvals_status ON customer_transfer_approvals (approval_status);
-- 索引：按申请人过滤（我的申请）
CREATE INDEX IF NOT EXISTS idx_transfer_approvals_applicant ON customer_transfer_approvals (applicant_id);
-- 索引：按客户/线索过滤
CREATE INDEX IF NOT EXISTS idx_transfer_approvals_lead ON customer_transfer_approvals (lead_id);
-- 索引：按当前审批人过滤（待我审批）
CREATE INDEX IF NOT EXISTS idx_transfer_approvals_manager ON customer_transfer_approvals (manager_approver_id) WHERE manager_approver_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_transfer_approvals_director ON customer_transfer_approvals (director_approver_id) WHERE director_approver_id IS NOT NULL;

COMMENT ON TABLE customer_transfer_approvals IS '客户转移审批表 - 多级审批流（销售经理 → 总监）';
COMMENT ON COLUMN customer_transfer_approvals.approval_no IS '审批单号（TA 前缀 + 时间戳）';
COMMENT ON COLUMN customer_transfer_approvals.is_large_customer IS '是否大客户转移（信用额度超阈值时自动标记，需总监二次审批）';
COMMENT ON COLUMN customer_transfer_approvals.current_level IS '当前审批层级（1=经理审批中, 2=总监审批中）';
COMMENT ON COLUMN customer_transfer_approvals.max_level IS '最大审批层级（普通客户 1 层，大客户 2 层）';

-- =====================================================
-- 4. 大客户转移阈值配置（系统参数）
-- =====================================================
-- 通过 system_settings 表配置大客户信用额度阈值，超过则触发二次审批
-- 若 system_settings 表不存在则跳过（部分环境未启用）
INSERT INTO system_settings (key, value, description, created_at, updated_at)
SELECT 'crm.large_customer_credit_threshold', '500000', '大客户信用额度阈值（超过则转移需总监二次审批）', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'system_settings')
ON CONFLICT DO NOTHING;
