-- batch-14 P3: 为 ai_decision_logs 表添加敏感度和操作类别字段

ALTER TABLE ai_decision_logs
ADD COLUMN IF NOT EXISTS sensitivity_level VARCHAR(20) NOT NULL DEFAULT 'low';

ALTER TABLE ai_decision_logs
ADD COLUMN IF NOT EXISTS operation_category VARCHAR(20) NOT NULL DEFAULT 'inference';

-- 更新现有记录的 operation_category
UPDATE ai_decision_logs SET operation_category = 'inference'
WHERE decision_type IN ('process_optimization', 'quality_prediction', 'sales_forecast', 'inventory_optimization', 'anomaly_detection', 'recommendation');

UPDATE ai_decision_logs SET operation_category = 'management'
WHERE decision_type IN ('model_management', 'model_activation', 'model_approval');
