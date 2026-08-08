-- batch-14 P3: 回滚敏感度和操作类别字段

ALTER TABLE ai_decision_logs
DROP COLUMN IF EXISTS sensitivity_level;

ALTER TABLE ai_decision_logs
DROP COLUMN IF EXISTS operation_category;
