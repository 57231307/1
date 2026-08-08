-- B12-P2-4：创建角色变更审批表

CREATE TABLE IF NOT EXISTS role_change_approvals (
    id SERIAL PRIMARY KEY,
    approval_no VARCHAR(50) NOT NULL UNIQUE,
    change_type VARCHAR(20) NOT NULL,
    target_user_id INTEGER,
    target_role_id INTEGER NOT NULL,
    target_role_code VARCHAR(100) NOT NULL,
    proposed_permission_id INTEGER,
    proposed_resource_type VARCHAR(100),
    proposed_action VARCHAR(50),
    proposed_allowed BOOLEAN,
    applicant_id INTEGER NOT NULL,
    applicant_username VARCHAR(100) NOT NULL,
    approver1_id INTEGER,
    approver1_comment TEXT,
    approver1_at TIMESTAMP WITH TIME ZONE,
    approver2_id INTEGER,
    approver2_comment TEXT,
    approver2_at TIMESTAMP WITH TIME ZONE,
    status VARCHAR(20) NOT NULL DEFAULT 'pending_l1',
    current_level INTEGER NOT NULL DEFAULT 1,
    completed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 索引
CREATE INDEX idx_role_change_approvals_status ON role_change_approvals(status);
CREATE INDEX idx_role_change_approvals_applicant ON role_change_approvals(applicant_id);
CREATE INDEX idx_role_change_approvals_target_role ON role_change_approvals(target_role_id);
CREATE INDEX idx_role_change_approvals_created ON role_change_approvals(created_at);
