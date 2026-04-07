-- ========================================
-- 秉羲 ERP 系统 - BPM 流程引擎扩展表
-- 版本：2026-03-16
-- 模块：BPM 流程引擎扩展
-- 说明：创建 BPM 流程引擎扩展表（通知、统计、超时等）
-- ========================================

-- ========================================
-- 10. 流程通知表
-- ========================================

-- ==================== 流程通知表 ====================
CREATE TABLE IF NOT EXISTS bpm_task_notification (
    id SERIAL PRIMARY KEY,
    task_id INTEGER NOT NULL,                            -- 任务 ID
    instance_id INTEGER NOT NULL,                        -- 实例 ID
    user_id INTEGER NOT NULL,                            -- 用户 ID
    
    -- 通知信息
    notification_type VARCHAR(50) NOT NULL,              -- 通知类型（new_task/urge/overdue/delegation 等）
    notification_method VARCHAR(50),                     -- 通知方式（站内信/邮件/短信/微信）
    title VARCHAR(500) NOT NULL,                         -- 通知标题
    content TEXT NOT NULL,                               -- 通知内容
    
    -- 通知状态
    status VARCHAR(20) DEFAULT 'unread',                 -- 状态（unread/read/deleted）
    is_read BOOLEAN DEFAULT FALSE,                       -- 是否已读
    read_at TIMESTAMP,                                   -- 阅读时间
    
    -- 系统字段
    sent_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 发送时间
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE bpm_task_notification IS '流程通知表';
COMMENT ON COLUMN bpm_task_notification.notification_type IS '通知类型（new_task/urge/overdue/delegation 等）';
COMMENT ON COLUMN bpm_task_notification.status IS '状态（unread/read/deleted）';

-- 外键约束
ALTER TABLE bpm_task_notification ADD CONSTRAINT fk_bpm_tn_task
    FOREIGN KEY (task_id) REFERENCES bpm_task(id) ON DELETE CASCADE;
ALTER TABLE bpm_task_notification ADD CONSTRAINT fk_bpm_tn_instance
    FOREIGN KEY (instance_id) REFERENCES bpm_process_instance(id) ON DELETE CASCADE;
ALTER TABLE bpm_task_notification ADD CONSTRAINT fk_bpm_tn_user
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- 索引
CREATE INDEX IF NOT EXISTS idx_bpm_tn_task ON bpm_task_notification(task_id);
CREATE INDEX IF NOT EXISTS idx_bpm_tn_instance ON bpm_task_notification(instance_id);
CREATE INDEX IF NOT EXISTS idx_bpm_tn_user ON bpm_task_notification(user_id);
CREATE INDEX IF NOT EXISTS idx_bpm_tn_status ON bpm_task_notification(status);
CREATE INDEX IF NOT EXISTS idx_bpm_tn_is_read ON bpm_task_notification(is_read);
CREATE INDEX IF NOT EXISTS idx_bpm_tn_sent_at ON bpm_task_notification(sent_at DESC);

-- ========================================
-- 11. 流程统计表
-- ========================================

-- ==================== 流程统计表（按天） ====================
CREATE TABLE IF NOT EXISTS bpm_statistics_daily (
    id SERIAL PRIMARY KEY,
    statistics_date DATE NOT NULL,                       -- 统计日期
    process_definition_id INTEGER NOT NULL,              -- 流程定义 ID
    
    -- 发起统计
    initiated_count INTEGER DEFAULT 0,                   -- 发起数量
    completed_count INTEGER DEFAULT 0,                   -- 完成数量
    cancelled_count INTEGER DEFAULT 0,                   -- 取消数量
    
    -- 任务统计
    pending_tasks INTEGER DEFAULT 0,                     -- 待处理任务数
    completed_tasks INTEGER DEFAULT 0,                   -- 已完成任务数
    rejected_tasks INTEGER DEFAULT 0,                    -- 已拒绝任务数
    overdue_tasks INTEGER DEFAULT 0,                     -- 超时任务数
    
    -- 时效统计
    avg_duration_seconds BIGINT DEFAULT 0,               -- 平均耗时（秒）
    max_duration_seconds BIGINT DEFAULT 0,               -- 最大耗时（秒）
    min_duration_seconds BIGINT DEFAULT 0,               -- 最小耗时（秒）
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE bpm_statistics_daily IS '流程统计表（按天）';

-- 外键约束
ALTER TABLE bpm_statistics_daily ADD CONSTRAINT fk_bpm_sd_process_definition
    FOREIGN KEY (process_definition_id) REFERENCES bpm_process_definition(id);

-- 唯一约束
ALTER TABLE bpm_statistics_daily ADD CONSTRAINT uk_sd_date_process UNIQUE (statistics_date, process_definition_id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_bpm_sd_date ON bpm_statistics_daily(statistics_date);
CREATE INDEX IF NOT EXISTS idx_bpm_sd_process_definition ON bpm_statistics_daily(process_definition_id);

-- ========================================
-- 12. 流程超时配置表
-- ========================================

-- ==================== 流程超时配置表 ====================
CREATE TABLE IF NOT EXISTS bpm_timeout_config (
    id SERIAL PRIMARY KEY,
    process_definition_id INTEGER,                       -- 流程定义 ID（NULL 表示全局配置）
    node_id VARCHAR(100),                                -- 节点 ID（NULL 表示全局配置）
    
    -- 超时配置
    timeout_seconds INTEGER NOT NULL,                    -- 超时时间（秒）
    timeout_type VARCHAR(50) DEFAULT 'working_hours',    -- 超时类型（working_hours/calendar_hours）
    
    -- 超时动作
    action_type VARCHAR(50) DEFAULT 'notify',            -- 动作类型（notify/auto_approve/auto_reject/escalate）
    action_params JSONB,                                 -- 动作参数（JSON 格式）
    
    -- 通知配置
    notify_before_seconds INTEGER,                       -- 超时前通知时间（秒）
    notify_recipients INTEGER[],                         -- 通知接收人 ID 列表
    
    -- 系统字段
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否启用
    priority INTEGER DEFAULT 0,                          -- 优先级
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE bpm_timeout_config IS '流程超时配置表';
COMMENT ON COLUMN bpm_timeout_config.timeout_type IS '超时类型（working_hours/calendar_hours）';
COMMENT ON COLUMN bpm_timeout_config.action_type IS '动作类型（notify/auto_approve/auto_reject/escalate）';

-- 外键约束
ALTER TABLE bpm_timeout_config ADD CONSTRAINT fk_bpm_tc_process_definition
    FOREIGN KEY (process_definition_id) REFERENCES bpm_process_definition(id) ON DELETE CASCADE;

-- 索引
CREATE INDEX IF NOT EXISTS idx_bpm_tc_process_definition ON bpm_timeout_config(process_definition_id);
CREATE INDEX IF NOT EXISTS idx_bpm_tc_node ON bpm_timeout_config(node_id);
CREATE INDEX IF NOT EXISTS idx_bpm_tc_active ON bpm_timeout_config(is_active);
CREATE INDEX IF NOT EXISTS idx_bpm_tc_priority ON bpm_timeout_config(priority);

-- ========================================
-- 触发器
-- ========================================

-- 更新 updated_at 触发器
DROP TRIGGER IF EXISTS trg_bpm_tn_updated_at ON bpm_task_notification;
CREATE TRIGGER trg_bpm_tn_updated_at
    BEFORE UPDATE ON bpm_task_notification
    FOR EACH ROW
    EXECUTE FUNCTION update_bpm_updated_at_column();

DROP TRIGGER IF EXISTS trg_bpm_sd_updated_at ON bpm_statistics_daily;
CREATE TRIGGER trg_bpm_sd_updated_at
    BEFORE UPDATE ON bpm_statistics_daily
    FOR EACH ROW
    EXECUTE FUNCTION update_bpm_updated_at_column();

DROP TRIGGER IF EXISTS trg_bpm_tc_updated_at ON bpm_timeout_config;
CREATE TRIGGER trg_bpm_tc_updated_at
    BEFORE UPDATE ON bpm_timeout_config
    FOR EACH ROW
    EXECUTE FUNCTION update_bpm_updated_at_column();

-- ========================================
-- 迁移完成
-- ========================================
