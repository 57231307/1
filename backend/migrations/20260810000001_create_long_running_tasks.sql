-- batch-21 P2 25.4-I: 长任务处理机制
CREATE TABLE IF NOT EXISTS long_running_tasks (
    id BIGSERIAL PRIMARY KEY,
    task_type VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    params JSONB,
    progress INTEGER NOT NULL DEFAULT 0,
    result JSONB,
    error_message TEXT,
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    created_by INTEGER,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_long_running_tasks_status ON long_running_tasks(status);
CREATE INDEX idx_long_running_tasks_type ON long_running_tasks(task_type);
CREATE INDEX idx_long_running_tasks_created_by ON long_running_tasks(created_by);

COMMENT ON TABLE long_running_tasks IS '长任务状态持久化表';
