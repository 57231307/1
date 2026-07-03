-- 固定资产折旧期间记录表迁移（批次 88 PH-2）
-- 创建时间: 2026-07-03
-- 关联修复: 占位符 PH-2 — service `period` 参数仅写日志，未按期间记录折旧
--
-- 新建 fixed_asset_depreciation_records 表，按期间记录每笔折旧计提明细，
-- 支持审计追溯"资产 X 在 2026-06 期间计提了多少折旧"。
-- (asset_id, period) 唯一约束防止同一资产同一期间重复计提。

CREATE TABLE IF NOT EXISTS "fixed_asset_depreciation_records" (
    "id" SERIAL PRIMARY KEY,
    "asset_id" INTEGER NOT NULL REFERENCES "fixed_assets"("id"),
    "period" VARCHAR(7) NOT NULL,
    "depreciation_amount" DECIMAL(15, 2) NOT NULL,
    "accumulated_before" DECIMAL(15, 2) NOT NULL,
    "accumulated_after" DECIMAL(15, 2) NOT NULL,
    "net_value_before" DECIMAL(15, 2),
    "net_value_after" DECIMAL(15, 2),
    "depreciation_method" VARCHAR(50),
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "uk_fa_depreciation_records_asset_period" UNIQUE ("asset_id", "period")
);

CREATE INDEX IF NOT EXISTS "idx_fa_depreciation_records_asset" ON "fixed_asset_depreciation_records"("asset_id");
CREATE INDEX IF NOT EXISTS "idx_fa_depreciation_records_period" ON "fixed_asset_depreciation_records"("period");

COMMENT ON TABLE "fixed_asset_depreciation_records" IS '固定资产折旧期间记录表（批次 88 PH-2 占位符实现）';
COMMENT ON COLUMN "fixed_asset_depreciation_records"."period" IS '折旧期间（YYYY-MM 格式）';
COMMENT ON COLUMN "fixed_asset_depreciation_records"."depreciation_amount" IS '本期折旧额';
COMMENT ON COLUMN "fixed_asset_depreciation_records"."accumulated_before" IS '本期前累计折旧';
COMMENT ON COLUMN "fixed_asset_depreciation_records"."accumulated_after" IS '本期后累计折旧';
COMMENT ON COLUMN "fixed_asset_depreciation_records"."net_value_before" IS '本期前账面净值';
COMMENT ON COLUMN "fixed_asset_depreciation_records"."net_value_after" IS '本期后账面净值';
COMMENT ON COLUMN "fixed_asset_depreciation_records"."depreciation_method" IS '折旧方法（如 straight_line）';
