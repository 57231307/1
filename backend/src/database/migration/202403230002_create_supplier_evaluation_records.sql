-- 供应商评估记录表迁移脚本
-- 创建时间：2026-03-23
-- 描述：存储供应商评估的具体评分记录

CREATE TABLE IF NOT EXISTS supplier_evaluation_records (
    id SERIAL PRIMARY KEY,
    supplier_id INTEGER NOT NULL COMMENT '供应商ID',
    evaluation_period VARCHAR(50) NOT NULL COMMENT '评估周期（如：2024Q1）',
    indicator_id INTEGER NOT NULL COMMENT '评估指标ID',
    score DECIMAL(18, 2) NOT NULL COMMENT '得分',
    max_score INTEGER COMMENT '满分',
    weighted_score DECIMAL(18, 2) COMMENT '加权得分',
    evaluator_id INTEGER COMMENT '评估人ID',
    evaluation_date DATE COMMENT '评估日期',
    remark TEXT COMMENT '备注',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间'
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_supplier_eval_records_supplier ON supplier_evaluation_records(supplier_id);
CREATE INDEX IF NOT EXISTS idx_supplier_eval_records_indicator ON supplier_evaluation_records(indicator_id);
CREATE INDEX IF NOT EXISTS idx_supplier_eval_records_period ON supplier_evaluation_records(evaluation_period);

-- 添加注释
COMMENT ON TABLE supplier_evaluation_records IS '供应商评估记录表';
COMMENT ON COLUMN supplier_evaluation_records.id IS '记录ID';
COMMENT ON COLUMN supplier_evaluation_records.supplier_id IS '供应商ID';
COMMENT ON COLUMN supplier_evaluation_records.evaluation_period IS '评估周期';
COMMENT ON COLUMN supplier_evaluation_records.indicator_id IS '评估指标ID';
COMMENT ON COLUMN supplier_evaluation_records.score IS '得分';
COMMENT ON COLUMN supplier_evaluation_records.max_score IS '满分';
COMMENT ON COLUMN supplier_evaluation_records.weighted_score IS '加权得分';
COMMENT ON COLUMN supplier_evaluation_records.evaluator_id IS '评估人ID';
COMMENT ON COLUMN supplier_evaluation_records.evaluation_date IS '评估日期';
COMMENT ON COLUMN supplier_evaluation_records.remark IS '备注';
COMMENT ON COLUMN supplier_evaluation_records.created_at IS '创建时间';