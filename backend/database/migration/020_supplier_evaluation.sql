-- P2 级模块：供应商评估
-- 创建时间：2026-03-15
-- 功能：供应商绩效评估、评分管理、等级评定

-- 供应商评估指标表
CREATE TABLE supplier_evaluation_indicators (
    id SERIAL PRIMARY KEY,
    indicator_name VARCHAR(100) NOT NULL,
    indicator_code VARCHAR(50) NOT NULL UNIQUE,
    category VARCHAR(20) NOT NULL,
    weight DECIMAL(5,2) NOT NULL,
    max_score INTEGER DEFAULT 100,
    evaluation_method VARCHAR(50),
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 供应商评估记录表
CREATE TABLE supplier_evaluation_records (
    id SERIAL PRIMARY KEY,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id),
    evaluation_period VARCHAR(7) NOT NULL,
    indicator_id INTEGER REFERENCES supplier_evaluation_indicators(id),
    score DECIMAL(5,2),
    max_score INTEGER,
    weighted_score DECIMAL(5,2),
    evaluator_id INTEGER,
    evaluation_date DATE DEFAULT CURRENT_DATE,
    remark TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 供应商综合评分表
CREATE TABLE supplier_overall_scores (
    id SERIAL PRIMARY KEY,
    supplier_id INTEGER NOT NULL REFERENCES suppliers(id),
    evaluation_period VARCHAR(7) NOT NULL,
    total_score DECIMAL(5,2) NOT NULL,
    quality_score DECIMAL(5,2),
    delivery_score DECIMAL(5,2),
    price_score DECIMAL(5,2),
    service_score DECIMAL(5,2),
    level VARCHAR(10),
    rank INTEGER,
    evaluation_status VARCHAR(20) DEFAULT 'pending',
    evaluated_by INTEGER,
    evaluation_date DATE DEFAULT CURRENT_DATE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 供应商等级表
CREATE TABLE supplier_levels (
    id SERIAL PRIMARY KEY,
    level_code VARCHAR(10) NOT NULL UNIQUE,
    level_name VARCHAR(50) NOT NULL,
    min_score INTEGER NOT NULL,
    max_score INTEGER NOT NULL,
    benefits TEXT,
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_supplier_eval_indicators_category ON supplier_evaluation_indicators(category);
CREATE INDEX IF NOT EXISTS idx_supplier_eval_records_supplier ON supplier_evaluation_records(supplier_id);
CREATE INDEX IF NOT EXISTS idx_supplier_eval_records_period ON supplier_evaluation_records(evaluation_period);
CREATE INDEX IF NOT EXISTS idx_supplier_overall_scores_supplier ON supplier_overall_scores(supplier_id);
CREATE INDEX IF NOT EXISTS idx_supplier_overall_scores_period ON supplier_overall_scores(evaluation_period);

-- 添加中文注释
COMMENT ON TABLE supplier_evaluation_indicators IS '供应商评估指标表';
COMMENT ON COLUMN supplier_evaluation_indicators.category IS '指标类别（质量/交货/价格/服务）';
COMMENT ON COLUMN supplier_evaluation_indicators.weight IS '权重';
COMMENT ON COLUMN supplier_evaluation_indicators.max_score IS '满分';

COMMENT ON TABLE supplier_evaluation_records IS '供应商评估记录表';
COMMENT ON COLUMN supplier_evaluation_records.evaluation_period IS '评估期间';
COMMENT ON COLUMN supplier_evaluation_records.score IS '得分';
COMMENT ON COLUMN supplier_evaluation_records.weighted_score IS '加权得分';

COMMENT ON TABLE supplier_overall_scores IS '供应商综合评分表';
COMMENT ON COLUMN supplier_overall_scores.total_score IS '总分';
COMMENT ON COLUMN supplier_overall_scores.quality_score IS '质量得分';
COMMENT ON COLUMN supplier_overall_scores.delivery_score IS '交货得分';
COMMENT ON COLUMN supplier_overall_scores.price_score IS '价格得分';
COMMENT ON COLUMN supplier_overall_scores.service_score IS '服务得分';
COMMENT ON COLUMN supplier_overall_scores.level IS '等级（A/B/C/D）';

COMMENT ON TABLE supplier_levels IS '供应商等级表';
COMMENT ON COLUMN supplier_levels.level_code IS '等级代码';
COMMENT ON COLUMN supplier_levels.min_score IS '最低分';
COMMENT ON COLUMN supplier_levels.max_score IS '最高分';
COMMENT ON COLUMN supplier_levels.benefits IS '等级权益';
