-- ========================================
-- 秉羲 ERP 系统 - BPM 流程引擎数据库迁移
-- 版本：2026-03-16
-- 模块：BPM 流程引擎（Business Process Management）
-- 说明：创建 BPM 流程引擎相关的所有表、索引、触发器
-- ========================================

-- ========================================
-- 1. 流程定义表
-- ========================================

-- ==================== 流程定义表 ====================
CREATE TABLE bpm_process_definition (
    id SERIAL PRIMARY KEY,
    process_key VARCHAR(100) NOT NULL,                   -- 流程标识（英文唯一标识）
    process_name VARCHAR(200) NOT NULL,                  -- 流程名称
    process_version VARCHAR(20) NOT NULL,                -- 流程版本（v1.0.0）
    process_category VARCHAR(50) NOT NULL,               -- 流程分类（procurement/sales/finance/hr/other）
    description TEXT,                                    -- 流程描述
    icon_url VARCHAR(500),                               -- 流程图标
    cover_image_url VARCHAR(500),                        -- 封面图片
    
    -- 流程配置
    form_type VARCHAR(50) DEFAULT 'custom',              -- 表单类型（custom/dynamic）
    form_schema JSONB,                                   -- 表单 schema（JSON 格式）
    flow_definition JSONB NOT NULL,                      -- 流程定义（JSON 格式，包含节点、流转条件等）
    
    -- 状态管理
    status VARCHAR(20) DEFAULT 'draft',                  -- 状态（draft/active/suspended/deprecated）
    is_published BOOLEAN DEFAULT FALSE,                  -- 是否已发布
    published_at TIMESTAMP,                              -- 发布时间
    published_by INTEGER,                                -- 发布人 ID
    
    -- 权限配置
    visible_roles INTEGER[],                             -- 可见角色 ID 列表
    initiator_roles INTEGER[],                           -- 发起人角色 ID 列表
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER,                                  -- 更新人 ID
    remarks TEXT                                         -- 备注
);

COMMENT ON TABLE bpm_process_definition IS '流程定义表';
COMMENT ON COLUMN bpm_process_definition.process_key IS '流程标识（英文唯一标识）';
COMMENT ON COLUMN bpm_process_definition.flow_definition IS '流程定义（JSON 格式，包含节点、流转条件等）';
COMMENT ON COLUMN bpm_process_definition.status IS '状态（draft/active/suspended/deprecated）';
COMMENT ON COLUMN bpm_process_definition.visible_roles IS '可见角色 ID 列表';
COMMENT ON COLUMN bpm_process_definition.initiator_roles IS '发起人角色 ID 列表';

-- 唯一约束
ALTER TABLE bpm_process_definition ADD CONSTRAINT uk_process_key_version UNIQUE (process_key, process_version);

-- 索引
CREATE INDEX IF NOT EXISTS idx_bpm_pd_process_key ON bpm_process_definition(process_key);
CREATE INDEX IF NOT EXISTS idx_bpm_pd_status ON bpm_process_definition(status);
CREATE INDEX IF NOT EXISTS idx_bpm_pd_category ON bpm_process_definition(process_category);
CREATE INDEX IF NOT EXISTS idx_bpm_pd_published ON bpm_process_definition(is_published);
CREATE INDEX IF NOT EXISTS idx_bpm_pd_created_at ON bpm_process_definition(created_at DESC);

-- ========================================
-- 2. 流程实例表
-- ========================================

-- ==================== 流程实例表 ====================
CREATE TABLE bpm_process_instance (
    id SERIAL PRIMARY KEY,
    instance_no VARCHAR(100) NOT NULL UNIQUE,            -- 实例编号（自动生成）
    process_definition_id INTEGER NOT NULL,              -- 流程定义 ID
    business_type VARCHAR(50) NOT NULL,                  -- 业务类型（purchase_order/sales_order/payment/leave 等）
    business_id INTEGER NOT NULL,                        -- 业务 ID
    title VARCHAR(500) NOT NULL,                         -- 流程标题
    priority VARCHAR(20) DEFAULT 'normal',               -- 优先级（low/normal/high/urgent）
    
    -- 流程状态
    current_node_id VARCHAR(100),                        -- 当前节点 ID
    current_node_name VARCHAR(200),                      -- 当前节点名称
    status VARCHAR(20) DEFAULT 'running',                -- 状态（running/suspended/completed/terminated/cancelled）
    
    -- 人员信息
    initiator_id INTEGER NOT NULL,                       -- 发起人 ID
    initiator_name VARCHAR(100) NOT NULL,                -- 发起人姓名
    initiator_department_id INTEGER,                     -- 发起人部门 ID
    current_handler_ids INTEGER[],                       -- 当前处理人 ID 列表
    current_handler_names VARCHAR(100)[],                -- 当前处理人姓名列表
    
    -- 流程数据
    form_data JSONB,                                     -- 表单数据（JSON 格式）
    variables JSONB,                                     -- 流程变量（JSON 格式）
    
    -- 时间信息
    started_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 开始时间
    completed_at TIMESTAMP,                              -- 完成时间
    duration_seconds BIGINT,                             -- 耗时（秒）
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    remarks TEXT                                         -- 备注
);

COMMENT ON TABLE bpm_process_instance IS '流程实例表';
COMMENT ON COLUMN bpm_process_instance.business_type IS '业务类型（purchase_order/sales_order/payment/leave 等）';
COMMENT ON COLUMN bpm_process_instance.business_id IS '业务 ID';
COMMENT ON COLUMN bpm_process_instance.status IS '状态（running/suspended/completed/terminated/cancelled）';
COMMENT ON COLUMN bpm_process_instance.form_data IS '表单数据（JSON 格式）';
COMMENT ON COLUMN bpm_process_instance.variables IS '流程变量（JSON 格式）';

-- 外键约束
ALTER TABLE bpm_process_instance ADD CONSTRAINT fk_pi_process_definition
    FOREIGN KEY (process_definition_id) REFERENCES bpm_process_definition(id);
ALTER TABLE bpm_process_instance ADD CONSTRAINT fk_pi_initiator
    FOREIGN KEY (initiator_id) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_bpm_pi_instance_no ON bpm_process_instance(instance_no);
CREATE INDEX IF NOT EXISTS idx_bpm_pi_process_definition ON bpm_process_instance(process_definition_id);
CREATE INDEX IF NOT EXISTS idx_bpm_pi_business ON bpm_process_instance(business_type, business_id);
CREATE INDEX IF NOT EXISTS idx_bpm_pi_initiator ON bpm_process_instance(initiator_id);
CREATE INDEX IF NOT EXISTS idx_bpm_pi_status ON bpm_process_instance(status);
CREATE INDEX IF NOT EXISTS idx_bpm_pi_current_node ON bpm_process_instance(current_node_id);
CREATE INDEX IF NOT EXISTS idx_bpm_pi_current_handler ON bpm_process_instance USING GIN (current_handler_ids);
CREATE INDEX IF NOT EXISTS idx_bpm_pi_started_at ON bpm_process_instance(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_bpm_pi_completed_at ON bpm_process_instance(completed_at);

-- ========================================
-- 3. 流程任务表
-- ========================================

-- ==================== 流程任务表 ====================
CREATE TABLE bpm_task (
    id SERIAL PRIMARY KEY,
    task_no VARCHAR(100) NOT NULL UNIQUE,                -- 任务编号
    instance_id INTEGER NOT NULL,                        -- 实例 ID
    process_definition_id INTEGER NOT NULL,              -- 流程定义 ID
    
    -- 节点信息
    node_id VARCHAR(100) NOT NULL,                       -- 节点 ID
    node_name VARCHAR(200) NOT NULL,                     -- 节点名称
    node_type VARCHAR(50) NOT NULL,                      -- 节点类型（start/end/user_task/system_task/condition 等）
    
    -- 任务状态
    task_type VARCHAR(20) DEFAULT 'manual',              -- 任务类型（manual/auto/system）
    status VARCHAR(20) DEFAULT 'pending',                -- 状态（pending/processing/completed/rejected/withdrawn/cancelled）
    priority VARCHAR(20) DEFAULT 'normal',               -- 优先级（low/normal/high/urgent）
    
    -- 处理人信息
    assignee_ids INTEGER[],                              -- 指派人 ID 列表
    assignee_names VARCHAR(100)[],                       -- 指派人姓名列表
    candidate_role_ids INTEGER[],                        -- 候选角色 ID 列表
    candidate_user_ids INTEGER[],                        -- 候选用户 ID 列表
    actual_handler_id INTEGER,                           -- 实际处理人 ID
    actual_handler_name VARCHAR(100),                    -- 实际处理人姓名
    
    -- 审批信息
    action VARCHAR(20),                                  -- 操作（approve/reject/withdraw/terminate/delegate）
    approval_opinion TEXT,                               -- 审批意见
    attachment_urls TEXT[],                              -- 附件 URL 列表
    handled_at TIMESTAMP,                                -- 处理时间
    duration_seconds BIGINT,                             -- 处理耗时（秒）
    
    -- 超时配置
    due_date TIMESTAMP,                                  -- 预计完成时间
    is_overdue BOOLEAN DEFAULT FALSE,                    -- 是否超时
    overdue_days INTEGER,                                -- 超时天数
    
    -- 任务数据
    form_data JSONB,                                     -- 表单数据
    task_variables JSONB,                                -- 任务变量
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    remarks TEXT                                         -- 备注
);

COMMENT ON TABLE bpm_task IS '流程任务表';
COMMENT ON COLUMN bpm_task.node_type IS '节点类型（start/end/user_task/system_task/condition 等）';
COMMENT ON COLUMN bpm_task.status IS '状态（pending/processing/completed/rejected/withdrawn/cancelled）';
COMMENT ON COLUMN bpm_task.action IS '操作（approve/reject/withdraw/terminate/delegate）';
COMMENT ON COLUMN bpm_task.approval_opinion IS '审批意见';

-- 外键约束
ALTER TABLE bpm_task ADD CONSTRAINT fk_bpm_task_instance
    FOREIGN KEY (instance_id) REFERENCES bpm_process_instance(id) ON DELETE CASCADE;
ALTER TABLE bpm_task ADD CONSTRAINT fk_bpm_task_process_definition
    FOREIGN KEY (process_definition_id) REFERENCES bpm_process_definition(id);
ALTER TABLE bpm_task ADD CONSTRAINT fk_bpm_task_actual_handler
    FOREIGN KEY (actual_handler_id) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_bpm_task_task_no ON bpm_task(task_no);
CREATE INDEX IF NOT EXISTS idx_bpm_task_instance ON bpm_task(instance_id);
CREATE INDEX IF NOT EXISTS idx_bpm_task_process_definition ON bpm_task(process_definition_id);
CREATE INDEX IF NOT EXISTS idx_bpm_task_node ON bpm_task(node_id);
CREATE INDEX IF NOT EXISTS idx_bpm_task_status ON bpm_task(status);
CREATE INDEX IF NOT EXISTS idx_bpm_task_assignee_ids ON bpm_task USING GIN (assignee_ids);
CREATE INDEX IF NOT EXISTS idx_bpm_task_candidate_user_ids ON bpm_task USING GIN (candidate_user_ids);
CREATE INDEX IF NOT EXISTS idx_bpm_task_actual_handler ON bpm_task(actual_handler_id);
CREATE INDEX IF NOT EXISTS idx_bpm_task_created_at ON bpm_task(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_bpm_task_due_date ON bpm_task(due_date);
CREATE INDEX IF NOT EXISTS idx_bpm_task_overdue ON bpm_task(is_overdue);

-- ========================================
-- 4. 流程操作日志表
-- ========================================

-- ==================== 流程操作日志表 ====================
CREATE TABLE bpm_operation_log (
    id SERIAL PRIMARY KEY,
    log_no VARCHAR(100) NOT NULL UNIQUE,                 -- 日志编号
    instance_id INTEGER NOT NULL,                        -- 实例 ID
    task_id INTEGER,                                     -- 任务 ID
    
    -- 操作信息
    operation_type VARCHAR(50) NOT NULL,                 -- 操作类型（start/approve/reject/withdraw/terminate/delegate/assign 等）
    operation_desc VARCHAR(500) NOT NULL,                -- 操作描述
    operator_id INTEGER NOT NULL,                        -- 操作人 ID
    operator_name VARCHAR(100) NOT NULL,                 -- 操作人姓名
    operator_department_id INTEGER,                      -- 操作人部门 ID
    
    -- 操作详情
    from_node_id VARCHAR(100),                           -- 源节点 ID
    from_node_name VARCHAR(200),                         -- 源节点名称
    to_node_id VARCHAR(100),                             -- 目标节点 ID
    to_node_name VARCHAR(200),                           -- 目标节点名称
    approval_opinion TEXT,                               -- 审批意见
    attachment_urls TEXT[],                              -- 附件 URL 列表
    
    -- 操作时间
    operated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    
    -- 系统字段
    ip_address VARCHAR(50),                              -- 操作 IP
    device_info VARCHAR(200),                            -- 设备信息
    remarks TEXT                                         -- 备注
);

COMMENT ON TABLE bpm_operation_log IS '流程操作日志表';
COMMENT ON COLUMN bpm_operation_log.operation_type IS '操作类型（start/approve/reject/withdraw/terminate/delegate/assign 等）';
COMMENT ON COLUMN bpm_operation_log.approval_opinion IS '审批意见';

-- 外键约束
ALTER TABLE bpm_operation_log ADD CONSTRAINT fk_bpm_log_instance
    FOREIGN KEY (instance_id) REFERENCES bpm_process_instance(id) ON DELETE CASCADE;
ALTER TABLE bpm_operation_log ADD CONSTRAINT fk_bpm_log_task
    FOREIGN KEY (task_id) REFERENCES bpm_task(id);
ALTER TABLE bpm_operation_log ADD CONSTRAINT fk_bpm_log_operator
    FOREIGN KEY (operator_id) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_bpm_log_instance ON bpm_operation_log(instance_id);
CREATE INDEX IF NOT EXISTS idx_bpm_log_task ON bpm_operation_log(task_id);
CREATE INDEX IF NOT EXISTS idx_bpm_log_operator ON bpm_operation_log(operator_id);
CREATE INDEX IF NOT EXISTS idx_bpm_log_operated_at ON bpm_operation_log(operated_at DESC);
CREATE INDEX IF NOT EXISTS idx_bpm_log_operation_type ON bpm_operation_log(operation_type);

-- ========================================
-- 5. 流程节点配置表
-- ========================================

-- ==================== 流程节点配置表 ====================
CREATE TABLE bpm_node_config (
    id SERIAL PRIMARY KEY,
    process_definition_id INTEGER NOT NULL,              -- 流程定义 ID
    node_id VARCHAR(100) NOT NULL,                       -- 节点 ID
    node_name VARCHAR(200) NOT NULL,                     -- 节点名称
    node_type VARCHAR(50) NOT NULL,                      -- 节点类型
    
    -- 节点配置
    node_config JSONB,                                   -- 节点配置（JSON 格式）
    assignee_type VARCHAR(50),                           -- 指派人类型（user/role/department/variable）
    assignee_value TEXT,                                 -- 指派人值（根据类型不同而不同）
    
    -- 审批配置
    approval_type VARCHAR(20) DEFAULT 'or_sign',         -- 审批类型（or_sign/and_sign/or_first）
    min_approval_count INTEGER DEFAULT 1,                -- 最少审批通过人数
    need_comment BOOLEAN DEFAULT FALSE,                  -- 是否需要审批意见
    
    -- 超时配置
    timeout_seconds INTEGER,                             -- 超时时间（秒）
    timeout_action VARCHAR(50),                          -- 超时动作（auto_approve/auto_reject/notify）
    
    -- 通知配置
    notify_initiator BOOLEAN DEFAULT FALSE,              -- 是否通知发起人
    notify_handler BOOLEAN DEFAULT TRUE,                 -- 是否通知处理人
    
    -- 系统字段
    sort_order INTEGER DEFAULT 0,                        -- 排序
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否启用
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE bpm_node_config IS '流程节点配置表';
COMMENT ON COLUMN bpm_node_config.approval_type IS '审批类型（or_sign/and_sign/or_first）';
COMMENT ON COLUMN bpm_node_config.assignee_type IS '指派人类型（user/role/department/variable）';

-- 外键约束
ALTER TABLE bpm_node_config ADD CONSTRAINT fk_bpm_nc_process_definition
    FOREIGN KEY (process_definition_id) REFERENCES bpm_process_definition(id) ON DELETE CASCADE;

-- 唯一约束
ALTER TABLE bpm_node_config ADD CONSTRAINT uk_nc_process_node UNIQUE (process_definition_id, node_id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_bpm_nc_process_definition ON bpm_node_config(process_definition_id);
CREATE INDEX IF NOT EXISTS idx_bpm_nc_node_type ON bpm_node_config(node_type);
CREATE INDEX IF NOT EXISTS idx_bpm_nc_active ON bpm_node_config(is_active);

-- ========================================
-- 6. 流程流转条件表
-- ========================================

-- ==================== 流程流转条件表 ====================
CREATE TABLE bpm_transition_condition (
    id SERIAL PRIMARY KEY,
    process_definition_id INTEGER NOT NULL,              -- 流程定义 ID
    from_node_id VARCHAR(100) NOT NULL,                  -- 源节点 ID
    to_node_id VARCHAR(100) NOT NULL,                    -- 目标节点 ID
    
    -- 条件配置
    condition_name VARCHAR(200),                         -- 条件名称
    condition_expression TEXT,                           -- 条件表达式（支持脚本）
    condition_type VARCHAR(50) DEFAULT 'expression',     -- 条件类型（expression/script/default）
    priority INTEGER DEFAULT 0,                          -- 优先级（数字越小优先级越高）
    
    -- 系统字段
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否启用
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    remarks TEXT                                         -- 备注
);

COMMENT ON TABLE bpm_transition_condition IS '流程流转条件表';
COMMENT ON COLUMN bpm_transition_condition.condition_expression IS '条件表达式（支持脚本）';
COMMENT ON COLUMN bpm_transition_condition.condition_type IS '条件类型（expression/script/default）';

-- 外键约束
ALTER TABLE bpm_transition_condition ADD CONSTRAINT fk_bpm_tc_process_definition
    FOREIGN KEY (process_definition_id) REFERENCES bpm_process_definition(id) ON DELETE CASCADE;

-- 索引
CREATE INDEX IF NOT EXISTS idx_bpm_tc_process_definition ON bpm_transition_condition(process_definition_id);
CREATE INDEX IF NOT EXISTS idx_bpm_tc_from_node ON bpm_transition_condition(from_node_id);
CREATE INDEX IF NOT EXISTS idx_bpm_tc_to_node ON bpm_transition_condition(to_node_id);
CREATE INDEX IF NOT EXISTS idx_bpm_tc_active ON bpm_transition_condition(is_active);

-- ========================================
-- 7. 流程委托表
-- ========================================

-- ==================== 流程委托表 ====================
CREATE TABLE bpm_task_delegation (
    id SERIAL PRIMARY KEY,
    task_id INTEGER NOT NULL,                            -- 任务 ID
    delegator_id INTEGER NOT NULL,                       -- 委托人 ID
    delegatee_id INTEGER NOT NULL,                       -- 被委托人 ID
    delegation_type VARCHAR(20) NOT NULL,                -- 委托类型（temporary/permanent）
    
    -- 委托时间范围
    start_date DATE NOT NULL,                            -- 开始日期
    end_date DATE NOT NULL,                              -- 结束日期
    
    -- 委托状态
    status VARCHAR(20) DEFAULT 'active',                 -- 状态（active/expired/cancelled）
    
    -- 系统字段
    reason TEXT,                                         -- 委托原因
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    cancelled_at TIMESTAMP,                              -- 取消时间
    cancelled_by INTEGER                                 -- 取消人 ID
);

COMMENT ON TABLE bpm_task_delegation IS '流程委托表';
COMMENT ON COLUMN bpm_task_delegation.delegation_type IS '委托类型（temporary/permanent）';

-- 外键约束
ALTER TABLE bpm_task_delegation ADD CONSTRAINT fk_bpm_td_task
    FOREIGN KEY (task_id) REFERENCES bpm_task(id) ON DELETE CASCADE;
ALTER TABLE bpm_task_delegation ADD CONSTRAINT fk_bpm_td_delegator
    FOREIGN KEY (delegator_id) REFERENCES users(id);
ALTER TABLE bpm_task_delegation ADD CONSTRAINT fk_bpm_td_delegatee
    FOREIGN KEY (delegatee_id) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_bpm_td_task ON bpm_task_delegation(task_id);
CREATE INDEX IF NOT EXISTS idx_bpm_td_delegator ON bpm_task_delegation(delegator_id);
CREATE INDEX IF NOT EXISTS idx_bpm_td_delegatee ON bpm_task_delegation(delegatee_id);
CREATE INDEX IF NOT EXISTS idx_bpm_td_status ON bpm_task_delegation(status);
CREATE INDEX IF NOT EXISTS idx_bpm_td_date_range ON bpm_task_delegation(start_date, end_date);

-- ========================================
-- 8. 流程催办表
-- ========================================

-- ==================== 流程催办表 ====================
CREATE TABLE bpm_task_urge (
    id SERIAL PRIMARY KEY,
    task_id INTEGER NOT NULL,                            -- 任务 ID
    instance_id INTEGER NOT NULL,                        -- 实例 ID
    
    -- 催办信息
    urger_id INTEGER NOT NULL,                           -- 催办人 ID
    urger_name VARCHAR(100) NOT NULL,                    -- 催办人姓名
    urge_reason TEXT,                                    -- 催办原因
    urge_type VARCHAR(20) DEFAULT 'system',              -- 催办类型（system/manual）
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    notified_user_ids INTEGER[],                         -- 已通知用户 ID 列表
    remarks TEXT                                         -- 备注
);

COMMENT ON TABLE bpm_task_urge IS '流程催办表';
COMMENT ON COLUMN bpm_task_urge.urge_type IS '催办类型（system/manual）';

-- 外键约束
ALTER TABLE bpm_task_urge ADD CONSTRAINT fk_bpm_urge_task
    FOREIGN KEY (task_id) REFERENCES bpm_task(id) ON DELETE CASCADE;
ALTER TABLE bpm_task_urge ADD CONSTRAINT fk_bpm_urge_instance
    FOREIGN KEY (instance_id) REFERENCES bpm_process_instance(id) ON DELETE CASCADE;

-- 索引
CREATE INDEX IF NOT EXISTS idx_bpm_urge_task ON bpm_task_urge(task_id);
CREATE INDEX IF NOT EXISTS idx_bpm_urge_instance ON bpm_task_urge(instance_id);
CREATE INDEX IF NOT EXISTS idx_bpm_urge_created_at ON bpm_task_urge(created_at DESC);

-- ========================================
-- 9. 触发器函数
-- ========================================

-- ==================== 自动更新 updated_at ====================
CREATE OR REPLACE FUNCTION update_bpm_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ========================================
-- 10. 应用触发器
-- ========================================

-- 更新 updated_at 触发器
DROP TRIGGER IF EXISTS trg_bpm_pd_updated_at ON bpm_process_definition;
CREATE TRIGGER trg_bpm_pd_updated_at
    BEFORE UPDATE ON bpm_process_definition
    FOR EACH ROW
    EXECUTE FUNCTION update_bpm_updated_at_column();

DROP TRIGGER IF EXISTS trg_bpm_pi_updated_at ON bpm_process_instance;
CREATE TRIGGER trg_bpm_pi_updated_at
    BEFORE UPDATE ON bpm_process_instance
    FOR EACH ROW
    EXECUTE FUNCTION update_bpm_updated_at_column();

DROP TRIGGER IF EXISTS trg_bpm_task_updated_at ON bpm_task;
CREATE TRIGGER trg_bpm_task_updated_at
    BEFORE UPDATE ON bpm_task
    FOR EACH ROW
    EXECUTE FUNCTION update_bpm_updated_at_column();

DROP TRIGGER IF EXISTS trg_bpm_nc_updated_at ON bpm_node_config;
CREATE TRIGGER trg_bpm_nc_updated_at
    BEFORE UPDATE ON bpm_node_config
    FOR EACH ROW
    EXECUTE FUNCTION update_bpm_updated_at_column();

-- ========================================
-- 11. 初始化数据
-- ========================================

-- 初始化流程实例编号序列
CREATE SEQUENCE IF NOT EXISTS bpm_process_instance_no_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

-- 初始化任务编号序列
CREATE SEQUENCE IF NOT EXISTS bpm_task_no_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

-- 初始化日志编号序列
CREATE SEQUENCE IF NOT EXISTS bpm_operation_log_no_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

-- ========================================
-- 迁移完成
-- ========================================
