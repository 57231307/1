-- batch-15 P3: 会计期间结账操作日志字段
ALTER TABLE accounting_periods ADD COLUMN IF NOT EXISTS close_remark TEXT;
ALTER TABLE accounting_periods ADD COLUMN IF NOT EXISTS close_ip VARCHAR(50);
COMMENT ON COLUMN accounting_periods.close_remark IS '结账操作备注';
COMMENT ON COLUMN accounting_periods.close_ip IS '结账操作 IP';
