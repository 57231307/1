-- B-P1-8 修复（批次 365 v13 复审）：事件幂等去重表
-- 用于防止 BusinessEvent 重复消费导致重复副作用（如重复生成凭证、重复更新状态）。
-- 主键 (consumer_id, event_key) 保证同一消费者对同一事件键仅处理一次。
-- processed_at 索引便于定期清理过期记录。
CREATE TABLE IF NOT EXISTS processed_events (
    consumer_id   VARCHAR(100) NOT NULL,
    event_key     VARCHAR(200) NOT NULL,
    event_type    VARCHAR(100) NOT NULL,
    processed_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (consumer_id, event_key)
);

CREATE INDEX IF NOT EXISTS idx_processed_events_processed_at
    ON processed_events(processed_at);
