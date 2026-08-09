-- batch-15 P3: 固定资产折旧起算日字段
ALTER TABLE fixed_assets ADD COLUMN IF NOT EXISTS depreciation_start_date DATE;
COMMENT ON COLUMN fixed_assets.depreciation_start_date IS '折旧起算日（可灵活配置，默认为 in_service_date）';
