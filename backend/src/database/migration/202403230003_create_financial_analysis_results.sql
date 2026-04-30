-- 财务分析结果表迁移脚本
-- 创建时间：2026-03-23
-- 描述：存储财务分析的具体结果数据

CREATE TABLE IF NOT EXISTS financial_analysis_results (
    id SERIAL PRIMARY KEY,
    analysis_type VARCHAR(100) NOT NULL COMMENT '分析类型',
    period VARCHAR(50) NOT NULL COMMENT '周期（如：2024-01）',
    indicator_id INTEGER NOT NULL COMMENT '指标ID',
    indicator_value DECIMAL(18, 2) NOT NULL COMMENT '实际值',
    target_value DECIMAL(18, 2) COMMENT '目标值',
    variance DECIMAL(18, 2) COMMENT '差异（实际值-目标值）',
    variance_rate DECIMAL(10, 2) COMMENT '差异率（百分比）',
    trend VARCHAR(20) COMMENT '趋势：上升/下降/持平',
    analysis_date DATE COMMENT '分析日期',
    created_by INTEGER COMMENT '创建人ID',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间'
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_fin_analysis_results_type ON financial_analysis_results(analysis_type);
CREATE INDEX IF NOT EXISTS idx_fin_analysis_results_period ON financial_analysis_results(period);
CREATE INDEX IF NOT EXISTS idx_fin_analysis_results_indicator ON financial_analysis_results(indicator_id);
CREATE INDEX IF NOT EXISTS idx_fin_analysis_results_date ON financial_analysis_results(analysis_date);

-- 添加注释
COMMENT ON TABLE financial_analysis_results IS '财务分析结果表';
COMMENT ON COLUMN financial_analysis_results.id IS '记录ID';
COMMENT ON COLUMN financial_analysis_results.analysis_type IS '分析类型';
COMMENT ON COLUMN financial_analysis_results.period IS '周期';
COMMENT ON COLUMN financial_analysis_results.indicator_id IS '指标ID';
COMMENT ON COLUMN financial_analysis_results.indicator_value IS '实际值';
COMMENT ON COLUMN financial_analysis_results.target_value IS '目标值';
COMMENT ON COLUMN financial_analysis_results.variance IS '差异';
COMMENT ON COLUMN financial_analysis_results.variance_rate IS '差异率';
COMMENT ON COLUMN financial_analysis_results.trend IS '趋势';
COMMENT ON COLUMN financial_analysis_results.analysis_date IS '分析日期';
COMMENT ON COLUMN financial_analysis_results.created_by IS '创建人ID';
COMMENT ON COLUMN financial_analysis_results.created_at IS '创建时间';