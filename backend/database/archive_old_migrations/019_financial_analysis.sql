-- P2 级模块：财务分析
-- 创建时间：2026-03-15
-- 功能：财务指标分析、趋势分析、财务报表

-- 财务指标表
CREATE TABLE IF NOT EXISTS financial_indicators (
    id SERIAL PRIMARY KEY,
    indicator_name VARCHAR(100) NOT NULL,
    indicator_code VARCHAR(50) NOT NULL UNIQUE,
    indicator_type VARCHAR(20) NOT NULL,
    formula TEXT,
    unit VARCHAR(20),
    status VARCHAR(20) DEFAULT 'active',
    remark TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 财务分析结果表
CREATE TABLE IF NOT EXISTS financial_analysis_results (
    id SERIAL PRIMARY KEY,
    analysis_type VARCHAR(20) NOT NULL,
    period VARCHAR(7) NOT NULL,
    indicator_id INTEGER REFERENCES financial_indicators(id),
    indicator_value DECIMAL(18,4),
    target_value DECIMAL(18,4),
    variance DECIMAL(18,4),
    variance_rate DECIMAL(5,2),
    trend VARCHAR(10),
    analysis_date DATE DEFAULT CURRENT_DATE,
    created_by INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 财务趋势分析表
CREATE TABLE IF NOT EXISTS financial_trends (
    id SERIAL PRIMARY KEY,
    indicator_id INTEGER REFERENCES financial_indicators(id),
    period VARCHAR(7) NOT NULL,
    value DECIMAL(18,4) NOT NULL,
    previous_value DECIMAL(18,4),
    change_amount DECIMAL(18,4),
    change_rate DECIMAL(5,2),
    trend_direction VARCHAR(10),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 财务报表配置表
CREATE TABLE IF NOT EXISTS financial_report_configs (
    id SERIAL PRIMARY KEY,
    report_name VARCHAR(100) NOT NULL,
    report_type VARCHAR(20) NOT NULL,
    period_type VARCHAR(20) DEFAULT 'monthly',
    template_config JSONB,
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_financial_indicators_type ON financial_indicators(indicator_type);
CREATE INDEX IF NOT EXISTS idx_financial_analysis_period ON financial_analysis_results(period);
CREATE INDEX IF NOT EXISTS idx_financial_trends_period ON financial_trends(period);

-- 添加中文注释
COMMENT ON TABLE financial_indicators IS '财务指标表';
COMMENT ON COLUMN financial_indicators.indicator_name IS '指标名称';
COMMENT ON COLUMN financial_indicators.indicator_code IS '指标代码';
COMMENT ON COLUMN financial_indicators.indicator_type IS '指标类型（盈利/偿债/营运/发展）';
COMMENT ON COLUMN financial_indicators.formula IS '计算公式';

COMMENT ON TABLE financial_analysis_results IS '财务分析结果表';
COMMENT ON COLUMN financial_analysis_results.analysis_type IS '分析类型';
COMMENT ON COLUMN financial_analysis_results.period IS '期间（YYYY-MM）';
COMMENT ON COLUMN financial_analysis_results.indicator_value IS '指标值';
COMMENT ON COLUMN financial_analysis_results.target_value IS '目标值';
COMMENT ON COLUMN financial_analysis_results.variance IS '差异';
COMMENT ON COLUMN financial_analysis_results.variance_rate IS '差异率';

COMMENT ON TABLE financial_trends IS '财务趋势分析表';
COMMENT ON COLUMN financial_trends.value IS '当前值';
COMMENT ON COLUMN financial_trends.previous_value IS '上期值';
COMMENT ON COLUMN financial_trends.change_amount IS '变动额';
COMMENT ON COLUMN financial_trends.change_rate IS '变动率';
COMMENT ON COLUMN financial_trends.trend_direction IS '趋势方向（上升/下降/持平）';

COMMENT ON TABLE financial_report_configs IS '财务报表配置表';
COMMENT ON COLUMN financial_report_configs.report_type IS '报表类型（资产负债表/利润表/现金流量表）';
COMMENT ON COLUMN financial_report_configs.period_type IS '期间类型（月/季/年）';
