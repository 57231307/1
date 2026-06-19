-- 001_init.sql - notifications 微服务初始化 schema
-- 多租户隔离：所有表必含 tenant_id 字段
-- 索引：tenant_id + user_id + created_at DESC（按时间倒序拉取）

CREATE TABLE IF NOT EXISTS notification_messages (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    title VARCHAR(255) NOT NULL,
    content TEXT NOT NULL,
    category VARCHAR(50) NOT NULL DEFAULT 'system',
    priority SMALLINT NOT NULL DEFAULT 5,
    is_read BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 多租户 + 用户 + 时间倒序联合索引
CREATE INDEX IF NOT EXISTS idx_notif_tenant_user_time
    ON notification_messages (tenant_id, user_id, created_at DESC);

-- 租户 + 已读状态索引（用于未读数查询）
CREATE INDEX IF NOT EXISTS idx_notif_tenant_unread
    ON notification_messages (tenant_id, user_id, is_read)
    WHERE is_read = false;

-- 表注释
COMMENT ON TABLE notification_messages IS 'P3-1 notifications 微服务消息表';
COMMENT ON COLUMN notification_messages.tenant_id IS '租户 ID（多租户隔离强制字段）';
COMMENT ON COLUMN notification_messages.priority IS '优先级 1-10，1 最高';
