-- 预算执行明细表迁移脚本
-- 创建时间：2026-03-23
-- 描述：存储预算方案的实际执行明细记录

CREATE TABLE IF NOT EXISTS budget_executions (
    id SERIAL PRIMARY KEY,
    plan_id INTEGER NOT NULL COMMENT '预算方案ID',
    execution_type VARCHAR(20) NOT NULL COMMENT '执行类型：下达/调整/使用',
    amount DECIMAL(18, 2) NOT NULL COMMENT '金额',
    expense_type VARCHAR(50) COMMENT '费用类型',
    expense_date DATE NOT NULL COMMENT '费用日期',
    related_document_type VARCHAR(50) COMMENT '关联单据类型',
    related_document_id INTEGER COMMENT '关联单据ID',
    remark TEXT COMMENT '备注',
    created_by INTEGER COMMENT '创建人ID',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间'
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_budget_executions_plan ON budget_executions(plan_id);
CREATE INDEX IF NOT EXISTS idx_budget_executions_type ON budget_executions(execution_type);
CREATE INDEX IF NOT EXISTS idx_budget_executions_date ON budget_executions(expense_date);
CREATE INDEX IF NOT EXISTS idx_budget_executions_related_doc ON budget_executions(related_document_type, related_document_id);

-- 添加注释
COMMENT ON TABLE budget_executions IS '预算执行明细表';
COMMENT ON COLUMN budget_executions.id IS '执行明细ID';
COMMENT ON COLUMN budget_executions.plan_id IS '预算方案ID';
COMMENT ON COLUMN budget_executions.execution_type IS '执行类型';
COMMENT ON COLUMN budget_executions.amount IS '金额';
COMMENT ON COLUMN budget_executions.expense_type IS '费用类型';
COMMENT ON COLUMN budget_executions.expense_date IS '费用日期';
COMMENT ON COLUMN budget_executions.related_document_type IS '关联单据类型';
COMMENT ON COLUMN budget_executions.related_document_id IS '关联单据ID';
COMMENT ON COLUMN budget_executions.remark IS '备注';
COMMENT ON COLUMN budget_executions.created_by IS '创建人ID';
COMMENT ON COLUMN budget_executions.created_at IS '创建时间';