-- B-P1-7 修复（批次 384 v13 复审）：事件死信队列表
-- 事件处理失败超过最大重试次数后，将事件 payload 持久化到此表，
-- 供人工排查或补偿处理。
CREATE TABLE IF NOT EXISTS event_dead_letters (
    id SERIAL PRIMARY KEY,
    -- 事件标识
    event_type VARCHAR(100) NOT NULL,
    event_payload JSONB NOT NULL,
    -- 失败信息
    failure_reason TEXT NOT NULL,
    last_error TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 5,
    -- 状态：PENDING（待重试）/ DEAD（已入死信，待人工处理）/ RESOLVED（已人工处理）
    status VARCHAR(20) NOT NULL DEFAULT 'DEAD',
    -- 时间
    first_failed_at TIMESTAMP NOT NULL DEFAULT NOW(),
    last_retry_at TIMESTAMP,
    resolved_at TIMESTAMP,
    resolved_by INTEGER,
    -- 审计
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_event_dead_letters_status ON event_dead_letters(status);
CREATE INDEX idx_event_dead_letters_event_type ON event_dead_letters(event_type);
CREATE INDEX idx_event_dead_letters_created_at ON event_dead_letters(created_at);

COMMENT ON TABLE event_dead_letters IS 'B-P1-7 事件死信队列：事件处理失败超过最大重试次数后的持久化记录';
COMMENT ON COLUMN event_dead_letters.event_type IS '事件类型（BusinessEvent 变体名）';
COMMENT ON COLUMN event_dead_letters.event_payload IS '事件 payload（JSON 序列化）';
COMMENT ON COLUMN event_dead_letters.failure_reason IS '首次失败原因摘要';
COMMENT ON COLUMN event_dead_letters.last_error IS '最后一次失败的完整错误信息';
COMMENT ON COLUMN event_dead_letters.retry_count IS '已重试次数';
COMMENT ON COLUMN event_dead_letters.max_retries IS '最大重试次数（默认 5）';
COMMENT ON COLUMN event_dead_letters.status IS 'PENDING/DEAD/RESOLVED';
