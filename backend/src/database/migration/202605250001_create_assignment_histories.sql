-- Create assignment_histories table for CRM assignment tracking
CREATE TABLE IF NOT EXISTS assignment_histories (
    id SERIAL PRIMARY KEY,
    lead_id INTEGER NOT NULL,
    from_user_id INTEGER NOT NULL,
    to_user_id INTEGER NOT NULL,
    action VARCHAR(50) NOT NULL DEFAULT 'assign',
    reason TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,
    tenant_id INTEGER NOT NULL DEFAULT 0,
    
    INDEX idx_lead_id (lead_id),
    INDEX idx_from_user_id (from_user_id),
    INDEX idx_to_user_id (to_user_id),
    INDEX idx_tenant_id (tenant_id),
    INDEX idx_created_at (created_at)
);

COMMENT ON TABLE assignment_histories IS 'CRM 客户分配历史记录表';
COMMENT ON COLUMN assignment_histories.lead_id IS '线索 ID';
COMMENT ON COLUMN assignment_histories.from_user_id IS '原负责人 ID';
COMMENT ON COLUMN assignment_histories.to_user_id IS '新负责人 ID';
COMMENT ON COLUMN assignment_histories.action IS '操作类型：assign, transfer, reclaim';
COMMENT ON COLUMN assignment_histories.reason IS '分配原因';
COMMENT ON COLUMN assignment_histories.tenant_id IS '租户 ID';
