-- 批次 92 P3-12/P3-13 回滚：恢复原外键行为 + 恢复冗余索引

-- 1. 删除 ON DELETE RESTRICT 外键
ALTER TABLE "fixed_asset_depreciation_records"
    DROP CONSTRAINT IF EXISTS "fixed_asset_depreciation_records_asset_id_fkey";

-- 2. 恢复默认外键（ON DELETE NO ACTION）
ALTER TABLE "fixed_asset_depreciation_records"
    ADD CONSTRAINT "fixed_asset_depreciation_records_asset_id_fkey"
    FOREIGN KEY ("asset_id") REFERENCES "fixed_assets"("id");

-- 3. 恢复冗余单列索引
CREATE INDEX IF NOT EXISTS "idx_fa_depreciation_records_asset"
    ON "fixed_asset_depreciation_records"("asset_id");
