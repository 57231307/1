-- ============================================
-- 销售订单变更历史表
-- ============================================

CREATE TABLE IF NOT EXISTS sales_order_change_history (
    id SERIAL PRIMARY KEY,
    order_id INTEGER NOT NULL REFERENCES sales_orders(id) ON DELETE CASCADE,
    change_type VARCHAR(20) NOT NULL,  -- CREATE, UPDATE, DELETE, STATUS_CHANGE
    field_name VARCHAR(100),           -- 变更字段名
    old_value TEXT,                    -- 旧值
    new_value TEXT,                    -- 新值
    changed_by INTEGER NOT NULL REFERENCES users(id),
    changed_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    change_reason TEXT,                -- 变更原因
    ip_address VARCHAR(45),            -- 操作IP
    user_agent TEXT,                   -- 用户代理
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_order_history_order_id ON sales_order_change_history(order_id);
CREATE INDEX IF NOT EXISTS idx_order_history_changed_at ON sales_order_change_history(changed_at DESC);
CREATE INDEX IF NOT EXISTS idx_order_history_changed_by ON sales_order_change_history(changed_by);

-- 更新时间戳触发器
CREATE OR REPLACE FUNCTION update_order_history_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_order_history_updated_at ON sales_order_change_history;
CREATE TRIGGER trg_order_history_updated_at
    BEFORE UPDATE ON sales_order_change_history
    FOR EACH ROW
    EXECUTE FUNCTION update_order_history_updated_at();
