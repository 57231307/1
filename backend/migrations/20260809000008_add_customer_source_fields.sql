-- batch-15 P3: 客户来源和公海回收原因字段
ALTER TABLE customers ADD COLUMN IF NOT EXISTS source VARCHAR(50);
ALTER TABLE customers ADD COLUMN IF NOT EXISTS pool_recycle_reason TEXT;
COMMENT ON COLUMN customers.source IS '客户来源（manual-手动录入、pool-公海、lead-线索转化、import-导入、api-接口）';
COMMENT ON COLUMN customers.pool_recycle_reason IS '公海客户回收原因';
