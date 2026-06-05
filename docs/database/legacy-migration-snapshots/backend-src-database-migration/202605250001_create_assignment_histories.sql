-- Create assignment_histories table for CRM assignment tracking
CREATE TABLE IF NOT EXISTS assignment_histories (
    id SERIAL PRIMARY KEY,
    lead_id INTEGER NOT NULL,
    lead_no VARCHAR(100),
    company_name VARCHAR(500),
    from_user_id INTEGER,
    from_user_name VARCHAR(100),
    to_user_id INTEGER,
    to_user_name VARCHAR(100),
    action VARCHAR(50) NOT NULL DEFAULT 'ASSIGN',
    reason TEXT,
    notes TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,
    created_by_name VARCHAR(100),
    tenant_id INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_assignment_histories_lead_id ON assignment_histories(lead_id);
CREATE INDEX IF NOT EXISTS idx_assignment_histories_from_user_id ON assignment_histories(from_user_id);
CREATE INDEX IF NOT EXISTS idx_assignment_histories_to_user_id ON assignment_histories(to_user_id);
CREATE INDEX IF NOT EXISTS idx_assignment_histories_tenant_id ON assignment_histories(tenant_id);
CREATE INDEX IF NOT EXISTS idx_assignment_histories_created_at ON assignment_histories(created_at);

COMMENT ON TABLE assignment_histories IS 'CRM 客户分配历史记录表';
COMMENT ON COLUMN assignment_histories.lead_id IS '线索 ID';
COMMENT ON COLUMN assignment_histories.lead_no IS '线索编号';
COMMENT ON COLUMN assignment_histories.company_name IS '公司名称';
COMMENT ON COLUMN assignment_histories.from_user_id IS '原负责人 ID';
COMMENT ON COLUMN assignment_histories.from_user_name IS '原负责人姓名';
COMMENT ON COLUMN assignment_histories.to_user_id IS '新负责人 ID';
COMMENT ON COLUMN assignment_histories.to_user_name IS '新负责人姓名';
COMMENT ON COLUMN assignment_histories.action IS '操作类型：ASSIGN, TRANSFER, RECLAIM';
COMMENT ON COLUMN assignment_histories.reason IS '分配原因';
COMMENT ON COLUMN assignment_histories.notes IS '备注';
COMMENT ON COLUMN assignment_histories.tenant_id IS '租户 ID';
