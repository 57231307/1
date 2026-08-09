-- batch-13 P3: 客户特殊工艺要求字段
ALTER TABLE customers ADD COLUMN IF NOT EXISTS special_process TEXT;
COMMENT ON COLUMN customers.special_process IS '客户特殊工艺要求';
