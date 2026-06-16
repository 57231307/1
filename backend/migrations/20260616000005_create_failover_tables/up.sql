-- 主备隔离模块 migration
-- 创建 3 张核心表：failover_status / failover_event / failover_config

-- 1. 主备实时状态表：记录每个功能（数据库/缓存）当前的主备状态
CREATE TABLE IF NOT EXISTS failover_status (
    id BIGSERIAL PRIMARY KEY,
    function_name VARCHAR(100) NOT NULL UNIQUE,
    current_state VARCHAR(20) NOT NULL DEFAULT 'primary',
    circuit_state VARCHAR(20) NOT NULL DEFAULT 'closed',
    primary_url VARCHAR(500),
    backup_type VARCHAR(50),
    last_switch_at TIMESTAMPTZ,
    last_success_at TIMESTAMPTZ,
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    total_primary_calls BIGINT NOT NULL DEFAULT 0,
    total_backup_calls BIGINT NOT NULL DEFAULT 0,
    total_switches BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_failover_status_state CHECK (current_state IN ('primary', 'backup', 'both_down')),
    CONSTRAINT chk_failover_status_circuit CHECK (circuit_state IN ('closed', 'open', 'half_open'))
);

CREATE INDEX IF NOT EXISTS idx_failover_status_func ON failover_status(function_name);
COMMENT ON TABLE failover_status IS '主备隔离实时状态表';
COMMENT ON COLUMN failover_status.function_name IS '功能名：database / cache';
COMMENT ON COLUMN failover_status.current_state IS '当前状态：primary（主调用中）/ backup（备用中）/ both_down（双不可用）';
COMMENT ON COLUMN failover_status.circuit_state IS '熔断器状态：closed（关闭）/ open（打开）/ half_open（半开）';

-- 2. 切换事件流水表：记录每次主备切换、熔断、回切事件
CREATE TABLE IF NOT EXISTS failover_event (
    id BIGSERIAL PRIMARY KEY,
    function_name VARCHAR(100) NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    from_state VARCHAR(20),
    to_state VARCHAR(20),
    reason TEXT,
    latency_ms INTEGER,
    tenant_id BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_failover_event_type CHECK (event_type IN ('switch_to_backup', 'switch_back', 'primary_recovered', 'both_failed', 'circuit_open', 'circuit_close', 'circuit_half_open'))
);

CREATE INDEX IF NOT EXISTS idx_failover_event_func_time ON failover_event(function_name, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_failover_event_type ON failover_event(event_type);
CREATE INDEX IF NOT EXISTS idx_failover_event_tenant ON failover_event(tenant_id);
COMMENT ON TABLE failover_event IS '主备隔离切换事件流水';
COMMENT ON COLUMN failover_event.event_type IS '事件类型：switch_to_backup/switch_back/primary_recovered/both_failed/circuit_open/circuit_close/circuit_half_open';

-- 3. 配置持久化表：将动态配置持久化（运行时可调整）
CREATE TABLE IF NOT EXISTS failover_config (
    id BIGSERIAL PRIMARY KEY,
    function_name VARCHAR(100) NOT NULL,
    config_key VARCHAR(200) NOT NULL,
    config_value TEXT NOT NULL,
    description TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(function_name, config_key)
);

CREATE INDEX IF NOT EXISTS idx_failover_config_func ON failover_config(function_name) WHERE is_active = TRUE;
COMMENT ON TABLE failover_config IS '主备隔离配置持久化';

-- 4. 初始化数据：插入默认主备状态
INSERT INTO failover_status (function_name, current_state, circuit_state, backup_type)
VALUES
    ('database', 'primary', 'closed', 'postgres'),
    ('cache', 'primary', 'closed', 'lru')
ON CONFLICT (function_name) DO NOTHING;
