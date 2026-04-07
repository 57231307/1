-- ========================================
-- 秉羲 ERP 系统 - 数据可视化报表数据库迁移
-- 版本：2026-03-16
-- 模块：数据可视化与决策支持
-- 说明：创建报表、仪表板相关的所有表、索引
-- ========================================

-- ========================================
-- 1. 报表定义表
-- ========================================

-- ==================== 报表定义表 ====================
CREATE TABLE IF NOT EXISTS report_definition (
    id SERIAL PRIMARY KEY,
    report_key VARCHAR(100) NOT NULL,                    -- 报表标识
    report_name VARCHAR(200) NOT NULL,                   -- 报表名称
    report_category VARCHAR(50) NOT NULL,                -- 报表分类（financial/sales/procurement/inventory/hr）
    report_type VARCHAR(50) NOT NULL,                    -- 报表类型（summary/detail/analysis/dashboard）
    
    -- 报表配置
    data_source_type VARCHAR(50) DEFAULT 'sql',          -- 数据源类型（sql/api/custom）
    data_source_config JSONB,                            -- 数据源配置（JSON 格式）
    query_sql TEXT,                                      -- 查询 SQL
    columns_config JSONB,                                -- 列配置（JSON 格式）
    
    -- 筛选配置
    filter_config JSONB,                                 -- 筛选器配置（JSON 格式）
    default_filters JSONB,                               -- 默认筛选条件
    
    -- 权限配置
    visible_roles INTEGER[],                             -- 可见角色 ID 列表
    export_enabled BOOLEAN DEFAULT TRUE,                 -- 是否允许导出
    print_enabled BOOLEAN DEFAULT TRUE,                  -- 是否允许打印
    
    -- 调度配置
    schedule_enabled BOOLEAN DEFAULT FALSE,              -- 是否启用调度
    schedule_config JSONB,                               -- 调度配置（JSON 格式）
    last_run_at TIMESTAMP,                               -- 最后运行时间
    next_run_at TIMESTAMP,                               -- 下次运行时间
    
    -- 状态管理
    status VARCHAR(20) DEFAULT 'active',                 -- 状态（active/inactive/deprecated）
    is_system BOOLEAN DEFAULT FALSE,                     -- 是否系统报表
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE report_definition IS '报表定义表';
COMMENT ON COLUMN report_definition.data_source_config IS '数据源配置（JSON 格式）';
COMMENT ON COLUMN report_definition.columns_config IS '列配置（JSON 格式）';
COMMENT ON COLUMN report_definition.filter_config IS '筛选器配置（JSON 格式）';

-- 唯一约束
ALTER TABLE report_definition ADD CONSTRAINT uk_report_key UNIQUE (report_key);

-- 索引
CREATE INDEX IF NOT EXISTS idx_report_def_category ON report_definition(report_category);
CREATE INDEX IF NOT EXISTS idx_report_def_type ON report_definition(report_type);
CREATE INDEX IF NOT EXISTS idx_report_def_status ON report_definition(status);
CREATE INDEX IF NOT EXISTS idx_report_def_system ON report_definition(is_system);

-- ========================================
-- 2. 仪表板表
-- ========================================

-- ==================== 仪表板表 ====================
CREATE TABLE IF NOT EXISTS report_dashboard (
    id SERIAL PRIMARY KEY,
    dashboard_name VARCHAR(200) NOT NULL,                -- 仪表板名称
    dashboard_code VARCHAR(100) NOT NULL UNIQUE,         -- 仪表板编码
    description TEXT,                                    -- 仪表板描述
    
    -- 布局配置
    layout_config JSONB,                                 -- 布局配置（JSON 格式）
    widgets_config JSONB,                                -- 组件配置（JSON 格式）
    
    -- 权限配置
    visible_roles INTEGER[],                             -- 可见角色 ID 列表
    is_public BOOLEAN DEFAULT FALSE,                     -- 是否公开
    
    -- 状态管理
    status VARCHAR(20) DEFAULT 'active',                 -- 状态（active/inactive/draft）
    is_default BOOLEAN DEFAULT FALSE,                    -- 是否默认仪表板
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE report_dashboard IS '仪表板表';
COMMENT ON COLUMN report_dashboard.layout_config IS '布局配置（JSON 格式）';
COMMENT ON COLUMN report_dashboard.widgets_config IS '组件配置（JSON 格式）';

-- 唯一约束
ALTER TABLE report_dashboard ADD CONSTRAINT uk_dashboard_code UNIQUE (dashboard_code);

-- 索引
CREATE INDEX IF NOT EXISTS idx_report_dash_status ON report_dashboard(status);
CREATE INDEX IF NOT EXISTS idx_report_dash_public ON report_dashboard(is_public);
CREATE INDEX IF NOT EXISTS idx_report_dash_default ON report_dashboard(is_default);

-- ========================================
-- 3. 报表组件表
-- ========================================

-- ==================== 报表组件表 ====================
CREATE TABLE IF NOT EXISTS report_widget (
    id SERIAL PRIMARY KEY,
    widget_name VARCHAR(200) NOT NULL,                   -- 组件名称
    widget_type VARCHAR(50) NOT NULL,                    -- 组件类型（chart/table/card/map/pivot）
    widget_category VARCHAR(50),                         -- 组件分类
    
    -- 组件配置
    chart_type VARCHAR(50),                              -- 图表类型（bar/line/pie/scatter/area）
    data_source_config JSONB,                            -- 数据源配置
    visual_config JSONB,                                 -- 可视化配置
    interaction_config JSONB,                            -- 交互配置
    
    -- 状态管理
    is_system BOOLEAN DEFAULT FALSE,                     -- 是否系统组件
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否启用
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE report_widget IS '报表组件表';
COMMENT ON COLUMN report_widget.chart_type IS '图表类型（bar/line/pie/scatter/area）';
COMMENT ON COLUMN report_widget.data_source_config IS '数据源配置';
COMMENT ON COLUMN report_widget.visual_config IS '可视化配置';

-- 索引
CREATE INDEX IF NOT EXISTS idx_report_widget_type ON report_widget(widget_type);
CREATE INDEX IF NOT EXISTS idx_report_widget_category ON report_widget(widget_category);
CREATE INDEX IF NOT EXISTS idx_report_widget_active ON report_widget(is_active);

-- ========================================
-- 4. 报表订阅表
-- ========================================

-- ==================== 报表订阅表 ====================
CREATE TABLE IF NOT EXISTS report_subscription (
    id SERIAL PRIMARY KEY,
    report_id INTEGER NOT NULL,                          -- 报表 ID
    user_id INTEGER NOT NULL,                            -- 用户 ID
    
    -- 订阅配置
    subscription_type VARCHAR(50) DEFAULT 'email',       -- 订阅类型（email/sms/wechat/system）
    frequency VARCHAR(50) NOT NULL,                      -- 频率（daily/weekly/monthly/custom）
    schedule_time TIME,                                  -- 发送时间
    schedule_day_of_week INTEGER,                        -- 星期几（1-7）
    schedule_day_of_month INTEGER,                       -- 每月几号（1-31）
    
    -- 筛选条件
    filter_config JSONB,                                 -- 筛选条件（JSON 格式）
    
    -- 状态管理
    is_active BOOLEAN DEFAULT TRUE,                      -- 是否启用
    last_sent_at TIMESTAMP,                              -- 最后发送时间
    next_send_at TIMESTAMP,                              -- 下次发送时间
    
    -- 系统字段
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER,                                  -- 创建人 ID
    updated_by INTEGER                                   -- 更新人 ID
);

COMMENT ON TABLE report_subscription IS '报表订阅表';
COMMENT ON COLUMN report_subscription.subscription_type IS '订阅类型（email/sms/wechat/system）';
COMMENT ON COLUMN report_subscription.frequency IS '频率（daily/weekly/monthly/custom）';

-- 外键约束
ALTER TABLE report_subscription ADD CONSTRAINT fk_report_sub_report
    FOREIGN KEY (report_id) REFERENCES report_definition(id) ON DELETE CASCADE;
ALTER TABLE report_subscription ADD CONSTRAINT fk_report_sub_user
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

-- 唯一约束
ALTER TABLE report_subscription ADD CONSTRAINT uk_report_sub_user UNIQUE (report_id, user_id, subscription_type);

-- 索引
CREATE INDEX IF NOT EXISTS idx_report_sub_report ON report_subscription(report_id);
CREATE INDEX IF NOT EXISTS idx_report_sub_user ON report_subscription(user_id);
CREATE INDEX IF NOT EXISTS idx_report_sub_active ON report_subscription(is_active);
CREATE INDEX IF NOT EXISTS idx_report_sub_frequency ON report_subscription(frequency);

-- ========================================
-- 5. 报表导出历史表
-- ========================================

-- ==================== 报表导出历史表 ====================
CREATE TABLE IF NOT EXISTS report_export_history (
    id SERIAL PRIMARY KEY,
    export_no VARCHAR(100) NOT NULL UNIQUE,              -- 导出编号
    report_id INTEGER NOT NULL,                          -- 报表 ID
    user_id INTEGER NOT NULL,                            -- 用户 ID
    
    -- 导出信息
    export_format VARCHAR(20) NOT NULL,                  -- 导出格式（pdf/excel/csv/html）
    export_type VARCHAR(50) DEFAULT 'full',              -- 导出类型（full/filtered/custom）
    filter_config JSONB,                                 -- 筛选条件
    export_status VARCHAR(20) DEFAULT 'pending',         -- 导出状态（pending/processing/completed/failed）
    
    -- 文件信息
    file_path VARCHAR(500),                              -- 文件路径
    file_size BIGINT,                                    -- 文件大小（字节）
    download_count INTEGER DEFAULT 0,                    -- 下载次数
    download_url TEXT,                                   -- 下载 URL
    
    -- 时间信息
    requested_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP, -- 请求时间
    completed_at TIMESTAMP,                              -- 完成时间
    expires_at TIMESTAMP,                                -- 过期时间
    
    -- 系统字段
    error_message TEXT                                   -- 错误信息
);

COMMENT ON TABLE report_export_history IS '报表导出历史表';
COMMENT ON COLUMN report_export_history.export_format IS '导出格式（pdf/excel/csv/html）';
COMMENT ON COLUMN report_export_history.export_status IS '导出状态（pending/processing/completed/failed）';

-- 外键约束
ALTER TABLE report_export_history ADD CONSTRAINT fk_report_exp_report
    FOREIGN KEY (report_id) REFERENCES report_definition(id);
ALTER TABLE report_export_history ADD CONSTRAINT fk_report_exp_user
    FOREIGN KEY (user_id) REFERENCES users(id);

-- 索引
CREATE INDEX IF NOT EXISTS idx_report_exp_export_no ON report_export_history(export_no);
CREATE INDEX IF NOT EXISTS idx_report_exp_report ON report_export_history(report_id);
CREATE INDEX IF NOT EXISTS idx_report_exp_user ON report_export_history(user_id);
CREATE INDEX IF NOT EXISTS idx_report_exp_status ON report_export_history(export_status);
CREATE INDEX IF NOT EXISTS idx_report_exp_requested ON report_export_history(requested_at DESC);

-- ========================================
-- 6. 物化视图日志表
-- ========================================

-- ==================== 物化视图刷新日志表 ====================
CREATE TABLE IF NOT EXISTS report_mv_refresh_log (
    id SERIAL PRIMARY KEY,
    mv_name VARCHAR(200) NOT NULL,                       -- 物化视图名称
    refresh_type VARCHAR(50) DEFAULT 'full',             -- 刷新类型（full/concurrent）
    refresh_status VARCHAR(20) DEFAULT 'pending',        -- 刷新状态（pending/running/completed/failed）
    
    -- 时间信息
    started_at TIMESTAMP,                                -- 开始时间
    completed_at TIMESTAMP,                              -- 完成时间
    duration_seconds BIGINT,                             -- 耗时（秒）
    
    -- 统计信息
    rows_affected BIGINT,                                -- 影响行数
    
    -- 系统字段
    error_message TEXT,                                  -- 错误信息
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE report_mv_refresh_log IS '物化视图刷新日志表';

-- 索引
CREATE INDEX IF NOT EXISTS idx_report_mv_mv_name ON report_mv_refresh_log(mv_name);
CREATE INDEX IF NOT EXISTS idx_report_mv_status ON report_mv_refresh_log(refresh_status);
CREATE INDEX IF NOT EXISTS idx_report_mv_started ON report_mv_refresh_log(started_at DESC);

-- ========================================
-- 7. 触发器函数
-- ========================================

-- ==================== 自动更新 updated_at ====================
CREATE OR REPLACE FUNCTION update_report_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ========================================
-- 8. 应用触发器
-- ========================================

-- 更新 updated_at 触发器
DROP TRIGGER IF EXISTS trg_report_def_updated_at ON report_definition;
CREATE TRIGGER trg_report_def_updated_at
    BEFORE UPDATE ON report_definition
    FOR EACH ROW
    EXECUTE FUNCTION update_report_updated_at_column();

DROP TRIGGER IF EXISTS trg_report_dashboard_updated_at ON report_dashboard;
CREATE TRIGGER trg_report_dashboard_updated_at
    BEFORE UPDATE ON report_dashboard
    FOR EACH ROW
    EXECUTE FUNCTION update_report_updated_at_column();

DROP TRIGGER IF EXISTS trg_report_widget_updated_at ON report_widget;
CREATE TRIGGER trg_report_widget_updated_at
    BEFORE UPDATE ON report_widget
    FOR EACH ROW
    EXECUTE FUNCTION update_report_updated_at_column();

DROP TRIGGER IF EXISTS trg_report_subscription_updated_at ON report_subscription;
CREATE TRIGGER trg_report_subscription_updated_at
    BEFORE UPDATE ON report_subscription
    FOR EACH ROW
    EXECUTE FUNCTION update_report_updated_at_column();

-- ========================================
-- 9. 初始化数据 - 常用报表
-- ========================================

-- 初始化销售统计报表
INSERT INTO report_definition (report_key, report_name, report_category, report_type, query_sql, columns_config, is_system) VALUES
('sales_daily_summary', '销售日报', 'sales', 'summary', 
 'SELECT DATE(created_at) as date, COUNT(*) as order_count, SUM(total_amount) as total_amount FROM sales_orders GROUP BY DATE(created_at)',
 '[{"key": "date", "label": "日期", "type": "date"}, {"key": "order_count", "label": "订单数", "type": "number"}, {"key": "total_amount", "label": "总金额", "type": "money"}]',
 TRUE),
('sales_product_ranking', '产品销售排行', 'sales', 'analysis',
 'SELECT product_id, product_name, SUM(quantity) as total_quantity, SUM(amount) as total_amount FROM sales_order_items GROUP BY product_id, product_name ORDER BY total_quantity DESC',
 '[{"key": "product_name", "label": "产品名称", "type": "text"}, {"key": "total_quantity", "label": "销售数量", "type": "number"}, {"key": "total_amount", "label": "销售金额", "type": "money"}]',
 TRUE),
('procurement_supplier_stats', '供应商采购统计', 'procurement', 'summary',
 'SELECT supplier_id, supplier_name, COUNT(*) as order_count, SUM(total_amount) as total_amount FROM purchase_order GROUP BY supplier_id, supplier_name',
 '[{"key": "supplier_name", "label": "供应商", "type": "text"}, {"key": "order_count", "label": "订单数", "type": "number"}, {"key": "total_amount", "label": "采购金额", "type": "money"}]',
 TRUE) ON CONFLICT DO NOTHING;

-- 初始化默认仪表板
INSERT INTO report_dashboard (dashboard_name, dashboard_code, description, is_default, is_public) VALUES
('经营驾驶舱', 'executive_dashboard', '企业经营管理驾驶舱，包含关键经营指标', TRUE, TRUE),
('销售看板', 'sales_dashboard', '销售业务数据看板', FALSE, TRUE),
('采购看板', 'procurement_dashboard', '采购业务数据看板', FALSE, TRUE) ON CONFLICT DO NOTHING;

-- ========================================
-- 迁移完成
-- ========================================
