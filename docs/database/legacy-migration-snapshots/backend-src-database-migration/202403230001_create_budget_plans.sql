-- 预算方案表迁移脚本
-- 创建时间：2026-03-23
-- 描述：存储预算方案信息

CREATE TABLE IF NOT EXISTS budget_plans (
    id SERIAL PRIMARY KEY,
    plan_no VARCHAR(50) NOT NULL UNIQUE COMMENT '方案编号',
    plan_name VARCHAR(200) NOT NULL COMMENT '方案名称',
    budget_year INTEGER NOT NULL COMMENT '预算年度',
    department_id INTEGER NOT NULL COMMENT '部门ID',
    total_amount DECIMAL(18, 2) NOT NULL DEFAULT 0 COMMENT '总金额',
    start_date DATE NOT NULL COMMENT '开始日期',
    end_date DATE NOT NULL COMMENT '结束日期',
    status VARCHAR(20) NOT NULL DEFAULT 'draft' COMMENT '状态：draft-草稿、approved-已审批、rejected-已驳回、active-执行中、closed-已关闭',
    remark TEXT COMMENT '备注',
    created_by INTEGER COMMENT '创建人ID',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP COMMENT '更新时间'
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_budget_plans_year ON budget_plans(budget_year);
CREATE INDEX IF NOT EXISTS idx_budget_plans_department ON budget_plans(department_id);
CREATE INDEX IF NOT EXISTS idx_budget_plans_status ON budget_plans(status);
CREATE INDEX IF NOT EXISTS idx_budget_plans_plan_no ON budget_plans(plan_no);

-- 添加注释
COMMENT ON TABLE budget_plans IS '预算方案表';
COMMENT ON COLUMN budget_plans.id IS '方案ID';
COMMENT ON COLUMN budget_plans.plan_no IS '方案编号';
COMMENT ON COLUMN budget_plans.plan_name IS '方案名称';
COMMENT ON COLUMN budget_plans.budget_year IS '预算年度';
COMMENT ON COLUMN budget_plans.department_id IS '部门ID';
COMMENT ON COLUMN budget_plans.total_amount IS '总金额';
COMMENT ON COLUMN budget_plans.start_date IS '开始日期';
COMMENT ON COLUMN budget_plans.end_date IS '结束日期';
COMMENT ON COLUMN budget_plans.status IS '状态';
COMMENT ON COLUMN budget_plans.remark IS '备注';
COMMENT ON COLUMN budget_plans.created_by IS '创建人ID';
COMMENT ON COLUMN budget_plans.created_at IS '创建时间';
COMMENT ON COLUMN budget_plans.updated_at IS '更新时间';