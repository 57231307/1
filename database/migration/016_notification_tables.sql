-- ============================================
-- 迁移 016: 消息通知中心表结构
-- ============================================

-- 通知消息表
CREATE TABLE IF NOT EXISTS notifications (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    notification_type VARCHAR(20) NOT NULL DEFAULT 'INTERNAL',
    title VARCHAR(255) NOT NULL,
    content TEXT NOT NULL,
    priority VARCHAR(10) NOT NULL DEFAULT 'NORMAL',
    status VARCHAR(20) NOT NULL DEFAULT 'UNREAD',
    business_type VARCHAR(50),
    business_id INTEGER,
    action_url VARCHAR(500),
    sender_id INTEGER,
    sender_name VARCHAR(100),
    read_at TIMESTAMPTZ WITH TIME ZONE,
    processed_at TIMESTAMPTZ WITH TIME ZONE,
    created_at TIMESTAMPTZ WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_notifications_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- 通知设置表
CREATE TABLE IF NOT EXISTS notification_settings (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    business_type VARCHAR(50) NOT NULL,
    enable_internal BOOLEAN NOT NULL DEFAULT true,
    enable_email BOOLEAN NOT NULL DEFAULT false,
    enable_sms BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_notification_settings_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    CONSTRAINT uk_notification_settings_user_business UNIQUE (user_id, business_type)
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_notifications_user_id ON notifications(user_id);
CREATE INDEX IF NOT EXISTS idx_notifications_status ON notifications(status);
CREATE INDEX IF NOT EXISTS idx_notifications_type ON notifications(notification_type);
CREATE INDEX IF NOT EXISTS idx_notifications_business ON notifications(business_type, business_id);
CREATE INDEX IF NOT EXISTS idx_notifications_created_at ON notifications(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_notification_settings_user ON notification_settings(user_id);

-- 添加注释
COMMENT ON TABLE notifications IS '通知消息表，存储系统内所有通知消息';
COMMENT ON TABLE notification_settings IS '通知设置表，存储用户的通知偏好设置';
COMMENT ON COLUMN notifications.notification_type IS '通知类型：INTERNAL-站内信, EMAIL-邮件, SMS-短信, SYSTEM-系统通知';
COMMENT ON COLUMN notifications.priority IS '优先级：LOW-低, NORMAL-普通, HIGH-高, URGENT-紧急';
COMMENT ON COLUMN notifications.status IS '状态：UNREAD-未读, READ-已读, PROCESSED-已处理, DELETED-已删除';
COMMENT ON COLUMN notifications.business_type IS '业务类型：ORDER-订单, APPROVAL-审批, INVENTORY-库存等';
