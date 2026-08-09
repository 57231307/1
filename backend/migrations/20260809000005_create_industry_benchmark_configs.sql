-- batch-15 P3: 行业基准配置表
CREATE TABLE IF NOT EXISTS industry_benchmark_configs (
    id BIGSERIAL PRIMARY KEY,
    benchmark_name VARCHAR(100) NOT NULL,
    industry_type VARCHAR(50) NOT NULL,
    metric_name VARCHAR(100) NOT NULL,
    metric_value DECIMAL(14,4) NOT NULL,
    unit VARCHAR(20),
    data_source VARCHAR(200),
    data_year INTEGER,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    remark TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_industry_benchmark_configs_industry_type ON industry_benchmark_configs(industry_type);
CREATE INDEX idx_industry_benchmark_configs_metric_name ON industry_benchmark_configs(metric_name);

COMMENT ON TABLE industry_benchmark_configs IS '行业基准配置表';
