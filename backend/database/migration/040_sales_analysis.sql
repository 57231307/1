-- P2 级模块：销售分析
-- 创建时间：2026-03-15
-- 功能：销售统计分析、销售趋势、业绩排行

-- 销售统计表
CREATE TABLE sales_statistics (
    id SERIAL PRIMARY KEY,
    statistic_type VARCHAR(20) NOT NULL,
    period VARCHAR(7) NOT NULL,
    dimension_type VARCHAR(20) NOT NULL,
    dimension_id INTEGER,
    dimension_name VARCHAR(200),
    order_count INTEGER DEFAULT 0,
    total_amount DECIMAL(18,2) DEFAULT 0,
    total_qty DECIMAL(14,4) DEFAULT 0,
    total_cost DECIMAL(18,2) DEFAULT 0,
    gross_profit DECIMAL(18,2) DEFAULT 0,
    gross_profit_rate DECIMAL(5,2),
    avg_order_value DECIMAL(18,2),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 销售趋势表
CREATE TABLE sales_trends (
    id SERIAL PRIMARY KEY,
    period VARCHAR(7) NOT NULL,
    product_id INTEGER REFERENCES products(id),
    customer_id INTEGER REFERENCES customers(id),
    sales_amount DECIMAL(18,2) NOT NULL,
    previous_amount DECIMAL(18,2),
    change_amount DECIMAL(18,2),
    change_rate DECIMAL(5,2),
    trend_direction VARCHAR(10),
    qty DECIMAL(14,4),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 销售业绩排行表
CREATE TABLE sales_performance_rankings (
    id SERIAL PRIMARY KEY,
    ranking_type VARCHAR(20) NOT NULL,
    period VARCHAR(7) NOT NULL,
    rank INTEGER NOT NULL,
    entity_id INTEGER NOT NULL,
    entity_name VARCHAR(200) NOT NULL,
    total_amount DECIMAL(18,2) NOT NULL,
    total_qty DECIMAL(14,4) DEFAULT 0,
    order_count INTEGER DEFAULT 0,
    target_amount DECIMAL(18,2),
    achievement_rate DECIMAL(5,2),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 销售目标表
CREATE TABLE sales_targets (
    id SERIAL PRIMARY KEY,
    target_type VARCHAR(20) NOT NULL,
    target_period VARCHAR(7) NOT NULL,
    department_id INTEGER REFERENCES departments(id),
    product_category_id INTEGER REFERENCES product_categories(id),
    target_amount DECIMAL(18,2) NOT NULL,
    actual_amount DECIMAL(18,2) DEFAULT 0,
    achievement_rate DECIMAL(5,2),
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_sales_statistics_period ON sales_statistics(period);
CREATE INDEX IF NOT EXISTS idx_sales_statistics_type ON sales_statistics(statistic_type);
CREATE INDEX IF NOT EXISTS idx_sales_trends_period ON sales_trends(period);
CREATE INDEX IF NOT EXISTS idx_sales_trends_product ON sales_trends(product_id);
CREATE INDEX IF NOT EXISTS idx_sales_performance_rankings_period ON sales_performance_rankings(period);
CREATE INDEX IF NOT EXISTS idx_sales_targets_period ON sales_targets(target_period);

-- 添加中文注释
COMMENT ON TABLE sales_statistics IS '销售统计表';
COMMENT ON COLUMN sales_statistics.statistic_type IS '统计类型（按产品/按客户/按部门）';
COMMENT ON COLUMN sales_statistics.dimension_type IS '维度类型';
COMMENT ON COLUMN sales_statistics.gross_profit IS '毛利润';
COMMENT ON COLUMN sales_statistics.gross_profit_rate IS '毛利率';

COMMENT ON TABLE sales_trends IS '销售趋势表';
COMMENT ON COLUMN sales_trends.previous_amount IS '上期金额';
COMMENT ON COLUMN sales_trends.trend_direction IS '趋势方向（上升/下降/持平）';

COMMENT ON TABLE sales_performance_rankings IS '销售业绩排行表';
COMMENT ON COLUMN sales_performance_rankings.ranking_type IS '排行类型（产品/客户/部门）';
COMMENT ON COLUMN sales_performance_rankings.achievement_rate IS '达成率';

COMMENT ON TABLE sales_targets IS '销售目标表';
COMMENT ON COLUMN sales_targets.target_type IS '目标类型（部门/产品/客户）';
COMMENT ON COLUMN sales_targets.target_period IS '目标期间';
