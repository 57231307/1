-- batch-15 P3: 线索自定义字段
ALTER TABLE crm_lead ADD COLUMN IF NOT EXISTS custom_fields JSONB;
COMMENT ON COLUMN crm_lead.custom_fields IS '自定义字段（JSON 格式，用于扩展性）';
