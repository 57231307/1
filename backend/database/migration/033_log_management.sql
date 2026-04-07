-- ========================================
-- 秉羲 ERP 系统 - 日志管理数据库迁移
-- 版本：2026-03-16
-- 模块：日志管理与追踪
-- 说明：创建日志管理相关的所有表、索引
-- ========================================

-- ========================================
-- 1. 操作日志表
-- ========================================

-- ==================== 操作日志表 ====================
CREATE TABLE IF NOT EXISTS log_operation (
    id BIGSERIAL PRIMARY KEY,
    log_no VARCHAR(100) NOT NULL UNIQUE,                 -- 日志编号
    module VARCHAR(50) NOT NULL,                         -- 模块（procurement/sales/inventory/finance 等）
    operation_type VARCHAR(50) NOT NULL,                 -- 操作类型（create/update/delete/approve/reject 等）
    operation_desc VARCHAR(500),                         -- 操作描述
    
    -- 业务信息
    business_type VARCHAR(50),                           -- 业务类型
    business_id INTEGER,                                 -- 业务 ID
    business_no VARCHAR(100),                            -- 业务编号
    business_desc VARCHAR(500),                          -- 业务描述
    
    -- 操作人信息
    user_id INTEGER NOT NULL,                            -- 用户 ID
    username VARCHAR(100) NOT NULL,                      -- 用户名
    real_name VARCHAR(100),                              -- 真实姓名
    department_id INTEGER,                               -- 部门 ID
    department_name VARCHAR(200),                        -- 部门名称
    
    -- 操作详情
    request_method VARCHAR(20),                          -- 请求方法（GET/POST/PUT/DELETE）
    request_url TEXT,                                    -- 请求 URL
    request_params JSONB,                                -- 请求参数
    request_body JSONB,                                  -- 请求体
    response_status INTEGER,                             -- 响应状态码
    response_body JSONB,                                 -- 响应体
    
    -- 设备信息
    ip_address VARCHAR(50),                              -- IP 地址
    ip_location VARCHAR(200),                            -- IP 所在地
    user_agent TEXT,                                     -- User-Agent
    device_type VARCHAR(50),                             -- 设备类型（PC/Mobile/Tablet）
    browser VARCHAR(100),                                -- 浏览器
    os VARCHAR(100),                                     -- 操作系统
    
    -- 时间信息
    operation_time TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 操作时间
    duration_ms INTEGER,                                 -- 耗时（毫秒）
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE log_operation IS '操作日志表';
COMMENT ON COLUMN log_operation.module IS '模块（procurement/sales/inventory/finance 等）';
COMMENT ON COLUMN log_operation.operation_type IS '操作类型（create/update/delete/approve/reject 等）';
COMMENT ON COLUMN log_operation.request_params IS '请求参数';
COMMENT ON COLUMN log_operation.request_body IS '请求体';
COMMENT ON COLUMN log_operation.response_body IS '响应体';

-- 索引
CREATE INDEX IF NOT EXISTS idx_log_op_log_no ON log_operation(log_no);
CREATE INDEX IF NOT EXISTS idx_log_op_module ON log_operation(module);
CREATE INDEX IF NOT EXISTS idx_log_op_operation_type ON log_operation(operation_type);
CREATE INDEX IF NOT EXISTS idx_log_op_user ON log_operation(user_id);
CREATE INDEX IF NOT EXISTS idx_log_op_business ON log_operation(business_type, business_id);
CREATE INDEX IF NOT EXISTS idx_log_op_operation_time ON log_operation(operation_time DESC);
CREATE INDEX IF NOT EXISTS idx_log_op_module_time ON log_operation(module, operation_time DESC);

-- ========================================
-- 2. 系统日志表
-- ========================================

-- ==================== 系统日志表 ====================
CREATE TABLE IF NOT EXISTS log_system (
    id BIGSERIAL PRIMARY KEY,
    log_no VARCHAR(100) NOT NULL UNIQUE,                 -- 日志编号
    
    -- 日志级别
    log_level VARCHAR(20) NOT NULL,                      -- 日志级别（DEBUG/INFO/WARN/ERROR/FATAL）
    
    -- 日志信息
    logger_name VARCHAR(200) NOT NULL,                   -- 记录器名称
    message TEXT NOT NULL,                               -- 日志消息
    exception_type VARCHAR(200),                         -- 异常类型
    exception_message TEXT,                              -- 异常消息
    stack_trace TEXT,                                    -- 堆栈跟踪
    log_data JSONB,                                      -- 日志数据
    
    -- 线程信息
    thread_name VARCHAR(100),                            -- 线程名称
    thread_id BIGINT,                                    -- 线程 ID
    
    -- 位置信息
    file_name VARCHAR(200),                              -- 文件名
    method_name VARCHAR(200),                            -- 方法名
    line_number INTEGER,                                 -- 行号
    
    -- 时间信息
    log_time TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 日志时间
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE log_system IS '系统日志表';
COMMENT ON COLUMN log_system.log_level IS '日志级别（DEBUG/INFO/WARN/ERROR/FATAL）';
COMMENT ON COLUMN log_system.logger_name IS '记录器名称';
COMMENT ON COLUMN log_system.log_data IS '日志数据';

-- 索引
CREATE INDEX IF NOT EXISTS idx_log_sys_log_no ON log_system(log_no);
CREATE INDEX IF NOT EXISTS idx_log_sys_level ON log_system(log_level);
CREATE INDEX IF NOT EXISTS idx_log_sys_logger ON log_system(logger_name);
CREATE INDEX IF NOT EXISTS idx_log_sys_time ON log_system(log_time DESC);
CREATE INDEX IF NOT EXISTS idx_log_sys_level_time ON log_system(log_level, log_time DESC);

-- ========================================
-- 3. 登录日志表
-- ========================================

-- ==================== 登录日志表 ====================
CREATE TABLE IF NOT EXISTS log_login (
    id BIGSERIAL PRIMARY KEY,
    log_no VARCHAR(100) NOT NULL UNIQUE,                 -- 日志编号
    
    -- 用户信息
    user_id INTEGER,                                     -- 用户 ID
    username VARCHAR(100) NOT NULL,                      -- 用户名
    real_name VARCHAR(100),                              -- 真实姓名
    
    -- 登录信息
    login_status VARCHAR(20) NOT NULL,                   -- 登录状态（success/failed/locked）
    failure_reason VARCHAR(200),                         -- 失败原因
    login_type VARCHAR(50) DEFAULT 'password',           -- 登录类型（password/sms/email/oauth）
    
    -- 设备信息
    ip_address VARCHAR(50),                              -- IP 地址
    ip_location VARCHAR(200),                            -- IP 所在地
    user_agent TEXT,                                     -- User-Agent
    device_type VARCHAR(50),                             -- 设备类型（PC/Mobile/Tablet）
    browser VARCHAR(100),                                -- 浏览器
    os VARCHAR(100),                                     -- 操作系统
    
    -- 时间信息
    login_time TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 登录时间
    logout_time TIMESTAMP,                               -- 登出时间
    session_duration_seconds BIGINT,                     -- 会话时长（秒）
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE log_login IS '登录日志表';
COMMENT ON COLUMN log_login.login_status IS '登录状态（success/failed/locked）';
COMMENT ON COLUMN log_login.login_type IS '登录类型（password/sms/email/oauth）';

-- 索引
CREATE INDEX IF NOT EXISTS idx_log_login_log_no ON log_login(log_no);
CREATE INDEX IF NOT EXISTS idx_log_login_user ON log_login(user_id);
CREATE INDEX IF NOT EXISTS idx_log_login_username ON log_login(username);
CREATE INDEX IF NOT EXISTS idx_log_login_status ON log_login(login_status);
CREATE INDEX IF NOT EXISTS idx_log_login_time ON log_login(login_time DESC);
CREATE INDEX IF NOT EXISTS idx_log_login_ip ON log_login(ip_address);

-- ========================================
-- 4. API 访问日志表
-- ========================================

-- ==================== API 访问日志表 ====================
CREATE TABLE IF NOT EXISTS log_api_access (
    id BIGSERIAL PRIMARY KEY,
    log_no VARCHAR(100) NOT NULL UNIQUE,                 -- 日志编号
    
    -- 请求信息
    request_id VARCHAR(100) NOT NULL,                    -- 请求 ID
    request_method VARCHAR(20) NOT NULL,                 -- 请求方法
    request_url TEXT NOT NULL,                           -- 请求 URL
    request_path VARCHAR(500),                           -- 请求路径
    query_params JSONB,                                  -- 查询参数
    request_headers JSONB,                               -- 请求头
    request_body TEXT,                                   -- 请求体
    content_type VARCHAR(100),                           -- Content-Type
    
    -- 响应信息
    response_status INTEGER NOT NULL,                    -- 响应状态码
    response_headers JSONB,                              -- 响应头
    response_body TEXT,                                  -- 响应体
    response_size BIGINT,                                -- 响应大小（字节）
    
    -- 性能信息
    duration_ms INTEGER NOT NULL,                        -- 耗时（毫秒）
    db_query_count INTEGER DEFAULT 0,                    -- 数据库查询次数
    db_query_time_ms INTEGER DEFAULT 0,                  -- 数据库查询耗时（毫秒）
    
    -- 客户端信息
    client_ip VARCHAR(50),                               -- 客户端 IP
    client_location VARCHAR(200),                        -- 客户端位置
    user_agent TEXT,                                     -- User-Agent
    client_type VARCHAR(50),                             -- 客户端类型（web/mobile/api）
    
    -- 认证信息
    user_id INTEGER,                                     -- 用户 ID
    username VARCHAR(100),                               -- 用户名
    auth_type VARCHAR(50),                               -- 认证类型（jwt/session/api_key）
    
    -- 时间信息
    access_time TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 访问时间
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE log_api_access IS 'API 访问日志表';
COMMENT ON COLUMN log_api_access.request_headers IS '请求头';
COMMENT ON COLUMN log_api_access.response_headers IS '响应头';
COMMENT ON COLUMN log_api_access.db_query_count IS '数据库查询次数';

-- 索引
CREATE INDEX IF NOT EXISTS idx_log_api_log_no ON log_api_access(log_no);
CREATE INDEX IF NOT EXISTS idx_log_api_request_id ON log_api_access(request_id);
CREATE INDEX IF NOT EXISTS idx_log_api_method ON log_api_access(request_method);
CREATE INDEX IF NOT EXISTS idx_log_api_status ON log_api_access(response_status);
CREATE INDEX IF NOT EXISTS idx_log_api_user ON log_api_access(user_id);
CREATE INDEX IF NOT EXISTS idx_log_api_time ON log_api_access(access_time DESC);
CREATE INDEX IF NOT EXISTS idx_log_api_path ON log_api_access(request_path);
CREATE INDEX IF NOT EXISTS idx_log_api_client_ip ON log_api_access(client_ip);

-- ========================================
-- 5. 触发器函数
-- ========================================

-- ==================== 自动生成日志编号 ====================
CREATE OR REPLACE FUNCTION generate_log_no()
RETURNS TRIGGER AS $$
BEGIN
    -- 根据表名生成不同的编号前缀
    IF TG_TABLE_NAME = 'log_operation' THEN
        NEW.log_no := 'OP' || TO_CHAR(CURRENT_DATE, 'YYYYMMDD') || LPAD(NEXTVAL('log_operation_id_seq')::TEXT, 10, '0');
    ELSIF TG_TABLE_NAME = 'log_system' THEN
        NEW.log_no := 'SYS' || TO_CHAR(CURRENT_DATE, 'YYYYMMDD') || LPAD(NEXTVAL('log_system_id_seq')::TEXT, 10, '0');
    ELSIF TG_TABLE_NAME = 'log_login' THEN
        NEW.log_no := 'LOG' || TO_CHAR(CURRENT_DATE, 'YYYYMMDD') || LPAD(NEXTVAL('log_login_id_seq')::TEXT, 10, '0');
    ELSIF TG_TABLE_NAME = 'log_api_access' THEN
        NEW.log_no := 'API' || TO_CHAR(CURRENT_DATE, 'YYYYMMDD') || LPAD(NEXTVAL('log_api_access_id_seq')::TEXT, 10, '0');
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ========================================
-- 6. 应用触发器
-- ========================================

-- 自动生成日志编号触发器
DROP TRIGGER IF EXISTS trg_log_operation_generate_no ON log_operation;
CREATE TRIGGER trg_log_operation_generate_no
    BEFORE INSERT ON log_operation
    FOR EACH ROW
    EXECUTE FUNCTION generate_log_no();

DROP TRIGGER IF EXISTS trg_log_system_generate_no ON log_system;
CREATE TRIGGER trg_log_system_generate_no
    BEFORE INSERT ON log_system
    FOR EACH ROW
    EXECUTE FUNCTION generate_log_no();

DROP TRIGGER IF EXISTS trg_log_login_generate_no ON log_login;
CREATE TRIGGER trg_log_login_generate_no
    BEFORE INSERT ON log_login
    FOR EACH ROW
    EXECUTE FUNCTION generate_log_no();

DROP TRIGGER IF EXISTS trg_log_api_access_generate_no ON log_api_access;
CREATE TRIGGER trg_log_api_access_generate_no
    BEFORE INSERT ON log_api_access
    FOR EACH ROW
    EXECUTE FUNCTION generate_log_no();

-- ========================================
-- 7. 分区表配置（按时间分区，优化大数据量场景）
-- ========================================

-- 注意：PostgreSQL 18 支持声明式分区，按 operation_time 月度分区
-- 由于分区表需要主表先存在，所以先创建主表，再创建分区

-- 创建分区主表（按月分区）
CREATE TABLE IF NOT EXISTS log_operation_partitioned (
    id BIGSERIAL,
    log_no VARCHAR(100) NOT NULL,
    module VARCHAR(50) NOT NULL,
    operation_type VARCHAR(50) NOT NULL,
    operation_desc VARCHAR(500),
    business_type VARCHAR(50),
    business_id INTEGER,
    business_no VARCHAR(100),
    business_desc VARCHAR(500),
    user_id INTEGER NOT NULL,
    username VARCHAR(100) NOT NULL,
    real_name VARCHAR(100),
    department_id INTEGER,
    department_name VARCHAR(200),
    request_method VARCHAR(20),
    request_url TEXT,
    request_params JSONB,
    request_body JSONB,
    response_status INTEGER,
    response_body JSONB,
    ip_address VARCHAR(50),
    ip_location VARCHAR(200),
    user_agent TEXT,
    device_type VARCHAR(50),
    browser VARCHAR(100),
    os VARCHAR(100),
    operation_time TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    duration_ms INTEGER,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id, operation_time)
) PARTITION BY RANGE (operation_time);

COMMENT ON TABLE log_operation_partitioned IS '操作日志分区表（按月度分区）';

-- 为 2026 年创建月度分区
CREATE TABLE IF NOT EXISTS log_operation_202601 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');
CREATE TABLE IF NOT EXISTS log_operation_202602 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');
CREATE TABLE IF NOT EXISTS log_operation_202603 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');
CREATE TABLE IF NOT EXISTS log_operation_202604 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE IF NOT EXISTS log_operation_202605 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE IF NOT EXISTS log_operation_202606 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');
CREATE TABLE IF NOT EXISTS log_operation_202607 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-07-01') TO ('2026-08-01');
CREATE TABLE IF NOT EXISTS log_operation_202608 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-08-01') TO ('2026-09-01');
CREATE TABLE IF NOT EXISTS log_operation_202609 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-09-01') TO ('2026-10-01');
CREATE TABLE IF NOT EXISTS log_operation_202610 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-10-01') TO ('2026-11-01');
CREATE TABLE IF NOT EXISTS log_operation_202611 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-11-01') TO ('2026-12-01');
CREATE TABLE IF NOT EXISTS log_operation_202612 PARTITION OF log_operation_partitioned
    FOR VALUES FROM ('2026-12-01') TO ('2027-01-01');

-- 分区表索引
CREATE INDEX IF NOT EXISTS idx_lop_partitioned_module ON log_operation_partitioned(module);
CREATE INDEX IF NOT EXISTS idx_lop_partitioned_operation_type ON log_operation_partitioned(operation_type);
CREATE INDEX IF NOT EXISTS idx_lop_partitioned_user ON log_operation_partitioned(user_id);
CREATE INDEX IF NOT EXISTS idx_lop_partitioned_business ON log_operation_partitioned(business_type, business_id);
CREATE INDEX IF NOT EXISTS idx_lop_partitioned_operation_time ON log_operation_partitioned(operation_time DESC);

-- 为分区表创建触发器以自动生成日志编号
DROP TRIGGER IF EXISTS trg_log_operation_partitioned_generate_no ON log_operation_partitioned;
CREATE TRIGGER trg_log_operation_partitioned_generate_no
    BEFORE INSERT ON log_operation_partitioned
    FOR EACH ROW
    EXECUTE FUNCTION generate_log_no();

-- 数据迁移：将原表数据迁移到分区表（可选，需要时执行）
-- INSERT INTO log_operation_partitioned SELECT * FROM log_operation;

COMMENT ON TABLE log_operation_partitioned IS '操作日志分区表 - 按月度分区存储';
COMMENT ON COLUMN log_operation_partitioned.operation_time IS '分区键 - 操作时间';

-- ========================================
-- 迁移完成
-- ========================================
