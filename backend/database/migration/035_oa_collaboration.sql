-- ========================================
-- 秉羲 ERP 系统 - OA 协同办公数据库迁移
-- 版本：2026-03-16
-- 模块：OA 协同办公
-- 说明：创建 OA 通知、公告相关的所有表、索引
-- ========================================

-- ========================================
-- 1. 通知公告表
-- ========================================

-- ==================== 通知公告表 ====================
CREATE TABLE IF NOT EXISTS oa_announcement (
    id SERIAL PRIMARY KEY,
    announcement_no VARCHAR(100) NOT NULL UNIQUE,        -- 公告编号
    title VARCHAR(500) NOT NULL,                         -- 公告标题
    content TEXT NOT NULL,                               -- 公告内容
    announcement_type VARCHAR(50) NOT NULL,              -- 公告类型（company_notice/department_notice/system_notice）
    priority VARCHAR(20) DEFAULT 'normal',               -- 优先级（low/normal/high/urgent）
    
    -- 发布信息
    publisher_id INTEGER NOT NULL,                       -- 发布人 ID
    publisher_name VARCHAR(100) NOT NULL,                -- 发布人姓名
    publisher_department_id INTEGER,                     -- 发布人部门 ID
    publish_date DATE NOT NULL,                          -- 发布日期
    publish_time TIME,                                   -- 发布时间
    
    -- 生效信息
    effective_date DATE,                                 -- 生效日期
    expiration_date DATE,                                -- 失效日期
    is_permanent BOOLEAN DEFAULT FALSE,                  -- 是否永久有效
    
    -- 范围配置
    visible_scope VARCHAR(50) DEFAULT 'all',             -- 可见范围（all/company/department/specific）
    visible_department_ids INTEGER[],                    -- 可见部门 ID 列表
    visible_role_ids INTEGER[],                          -- 可见角色 ID 列表
    visible_user_ids INTEGER[],                          -- 可见用户 ID 列表
    
    -- 状态管理
    status VARCHAR(20) DEFAULT 'draft',                  -- 状态（draft/published/archived/cancelled）
    is_top BOOLEAN DEFAULT FALSE,                        -- 是否置顶
    top_until DATE,                                      -- 置顶至
    view_count INTEGER DEFAULT 0,                        -- 浏览次数
    
    -- 附件
    attachment_urls TEXT[],                              -- 附件 URL 列表
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE oa_announcement IS '通知公告表';
COMMENT ON COLUMN oa_announcement.announcement_type IS '公告类型（company_notice/department_notice/system_notice）';
COMMENT ON COLUMN oa_announcement.visible_scope IS '可见范围（all/company/department/specific）';
COMMENT ON COLUMN oa_announcement.status IS '状态（draft/published/archived/cancelled）';

-- 外键约束
ALTER TABLE oa_announcement ADD CONSTRAINT fk_oa_ann_publisher
    FOREIGN KEY (publisher_id) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_oa_ann_announcement_no ON oa_announcement(announcement_no);
CREATE INDEX IF NOT EXISTS idx_oa_ann_type ON oa_announcement(announcement_type);
CREATE INDEX IF NOT EXISTS idx_oa_ann_status ON oa_announcement(status);
CREATE INDEX IF NOT EXISTS idx_oa_ann_priority ON oa_announcement(priority);
CREATE INDEX IF NOT EXISTS idx_oa_ann_publish_date ON oa_announcement(publish_date DESC);
CREATE INDEX IF NOT EXISTS idx_oa_ann_is_top ON oa_announcement(is_top);
CREATE INDEX IF NOT EXISTS idx_oa_ann_effective ON oa_announcement(effective_date);

-- ========================================
-- 2. 公告阅读记录表
-- ========================================

-- ==================== 公告阅读记录表 ====================
CREATE TABLE IF NOT EXISTS oa_announcement_read (
    id SERIAL PRIMARY KEY,
    announcement_id INTEGER NOT NULL,                    -- 公告 ID
    user_id INTEGER NOT NULL,                            -- 用户 ID
    
    -- 阅读信息
    is_read BOOLEAN DEFAULT FALSE,                       -- 是否已读
    read_at TIMESTAMP,                                   -- 阅读时间
    read_duration_seconds INTEGER,                       -- 阅读时长（秒）
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uk_ann_read UNIQUE (announcement_id, user_id)
);

COMMENT ON TABLE oa_announcement_read IS '公告阅读记录表';

-- 外键约束
ALTER TABLE oa_announcement_read ADD CONSTRAINT fk_oa_ar_announcement
    FOREIGN KEY (announcement_id) REFERENCES oa_announcement(id) ON DELETE CASCADE;
ALTER TABLE oa_announcement_read ADD CONSTRAINT fk_oa_ar_user
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- 索引
CREATE INDEX IF NOT EXISTS idx_oa_ar_announcement ON oa_announcement_read(announcement_id);
CREATE INDEX IF NOT EXISTS idx_oa_ar_user ON oa_announcement_read(user_id);
CREATE INDEX IF NOT EXISTS idx_oa_ar_is_read ON oa_announcement_read(is_read);

-- ========================================
-- 3. 站内消息表
-- ========================================

-- ==================== 站内消息表 ====================
CREATE TABLE IF NOT EXISTS oa_message (
    id SERIAL PRIMARY KEY,
    message_no VARCHAR(100) NOT NULL UNIQUE,             -- 消息编号
    
    -- 消息信息
    message_type VARCHAR(50) NOT NULL,                   -- 消息类型（system/task/approval/notice/personal）
    title VARCHAR(500) NOT NULL,                         -- 消息标题
    content TEXT NOT NULL,                               -- 消息内容
    priority VARCHAR(20) DEFAULT 'normal',               -- 优先级（low/normal/high/urgent）
    
    -- 发送信息
    sender_id INTEGER,                                   -- 发送人 ID（系统消息为 NULL）
    sender_name VARCHAR(100),                            -- 发送人姓名
    send_time TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 发送时间
    
    -- 接收信息
    receiver_type VARCHAR(50) DEFAULT 'user',            -- 接收者类型（user/department/role/all）
    receiver_ids INTEGER[],                              -- 接收者 ID 列表
    receiver_names VARCHAR(100)[],                       -- 接收者姓名列表
    
    -- 状态管理
    is_read BOOLEAN DEFAULT FALSE,                       -- 是否已读（对群发消息而言）
    read_count INTEGER DEFAULT 0,                        -- 已读人数
    total_count INTEGER DEFAULT 0,                       -- 总人数
    
    -- 关联信息
    business_type VARCHAR(50),                           -- 关联业务类型
    business_id INTEGER,                                 -- 关联业务 ID
    action_url VARCHAR(500),                             -- 操作 URL
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP                                 -- 过期时间
);

COMMENT ON TABLE oa_message IS '站内消息表';
COMMENT ON COLUMN oa_message.message_type IS '消息类型（system/task/approval/notice/personal）';
COMMENT ON COLUMN oa_message.receiver_type IS '接收者类型（user/department/role/all）';

-- 外键约束
ALTER TABLE oa_message ADD CONSTRAINT fk_oa_msg_sender
    FOREIGN KEY (sender_id) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_oa_msg_message_no ON oa_message(message_no);
CREATE INDEX IF NOT EXISTS idx_oa_msg_type ON oa_message(message_type);
CREATE INDEX IF NOT EXISTS idx_oa_msg_sender ON oa_message(sender_id);
CREATE INDEX IF NOT EXISTS idx_oa_msg_receiver_ids ON oa_message USING GIN (receiver_ids);
CREATE INDEX IF NOT EXISTS idx_oa_msg_send_time ON oa_message(send_time DESC);
CREATE INDEX IF NOT EXISTS idx_oa_msg_business ON oa_message(business_type, business_id);

-- ========================================
-- 4. 用户消息状态表
-- ========================================

-- ==================== 用户消息状态表 ====================
CREATE TABLE IF NOT EXISTS oa_user_message_status (
    id SERIAL PRIMARY KEY,
    message_id INTEGER NOT NULL,                         -- 消息 ID
    user_id INTEGER NOT NULL,                            -- 用户 ID
    
    -- 状态信息
    is_read BOOLEAN DEFAULT FALSE,                       -- 是否已读
    read_at TIMESTAMP,                                   -- 阅读时间
    is_starred BOOLEAN DEFAULT FALSE,                    -- 是否星标
    is_deleted BOOLEAN DEFAULT FALSE,                    -- 是否删除
    is_archived BOOLEAN DEFAULT FALSE,                   -- 是否归档
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uk_ums_message_user UNIQUE (message_id, user_id)
);

COMMENT ON TABLE oa_user_message_status IS '用户消息状态表';

-- 外键约束
ALTER TABLE oa_user_message_status ADD CONSTRAINT fk_oa_ums_message
    FOREIGN KEY (message_id) REFERENCES oa_message(id) ON DELETE CASCADE;
ALTER TABLE oa_user_message_status ADD CONSTRAINT fk_oa_ums_user
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- 索引
CREATE INDEX IF NOT EXISTS idx_oa_ums_message ON oa_user_message_status(message_id);
CREATE INDEX IF NOT EXISTS idx_oa_ums_user ON oa_user_message_status(user_id);
CREATE INDEX IF NOT EXISTS idx_oa_ums_is_read ON oa_user_message_status(is_read);
CREATE INDEX IF NOT EXISTS idx_oa_ums_is_starred ON oa_user_message_status(is_starred);

-- ========================================
-- 5. 触发器函数
-- ========================================

-- ==================== 自动更新 updated_at ====================
CREATE OR REPLACE FUNCTION update_oa_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ========================================
-- 6. 应用触发器
-- ========================================

-- 更新 updated_at 触发器
DROP TRIGGER IF EXISTS trg_oa_ann_updated_at ON oa_announcement;
CREATE TRIGGER trg_oa_ann_updated_at
    BEFORE UPDATE ON oa_announcement
    FOR EACH ROW
    EXECUTE FUNCTION update_oa_updated_at_column();

DROP TRIGGER IF EXISTS trg_oa_ar_updated_at ON oa_announcement_read;
CREATE TRIGGER trg_oa_ar_updated_at
    BEFORE UPDATE ON oa_announcement_read
    FOR EACH ROW
    EXECUTE FUNCTION update_oa_updated_at_column();

DROP TRIGGER IF EXISTS trg_oa_ums_updated_at ON oa_user_message_status;
CREATE TRIGGER trg_oa_ums_updated_at
    BEFORE UPDATE ON oa_user_message_status
    FOR EACH ROW
    EXECUTE FUNCTION update_oa_updated_at_column();

-- ========================================
-- 迁移完成
-- ========================================
