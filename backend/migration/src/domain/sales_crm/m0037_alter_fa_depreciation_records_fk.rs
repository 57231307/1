//! 批次 92 P3-12/P3-13：fixed_asset_depreciation_records 外键策略 + 冗余索引清理
//!
//! 创建时间: 2026-07-03
//! 关联修复:
//!   P3-12：外键 ON DELETE RESTRICT —— 禁止连带删除资产时静默删除折旧记录（保留审计完整性）
//!   P3-13：DROP 冗余单列索引 idx_fa_depreciation_records_asset ——
//!          UNIQUE(asset_id, period) 复合唯一索引最左前缀已覆盖 WHERE asset_id = ? 查询。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- 批次 92 P3-12/P3-13：fixed_asset_depreciation_records 外键策略 + 冗余索引清理
-- 创建时间: 2026-07-03
-- 关联修复:
--   P3-12：外键 ON DELETE RESTRICT —— 禁止连带删除资产时静默删除折旧记录（保留审计完整性）
--   P3-13：DROP 冗余索引 idx_fa_depreciation_records_asset ——
--          UNIQUE(asset_id, period) 复合唯一索引最左前缀已覆盖 WHERE asset_id = ? 查询，
--          单列索引冗余，徒增写入开销和存储。
--
-- 注：PostgreSQL 不支持直接 ALTER CONSTRAINT 改 ON DELETE 行为，需 DROP + ADD 重建。

-- 1. 删除原外键（ON DELETE NO ACTION 默认行为）
ALTER TABLE "fixed_asset_depreciation_records"
    DROP CONSTRAINT IF EXISTS "fixed_asset_depreciation_records_asset_id_fkey";

-- 2. 重建外键，显式 ON DELETE RESTRICT
ALTER TABLE "fixed_asset_depreciation_records"
    ADD CONSTRAINT "fixed_asset_depreciation_records_asset_id_fkey"
    FOREIGN KEY ("asset_id") REFERENCES "fixed_assets"("id") ON DELETE RESTRICT;

-- 3. 删除冗余单列索引（已被 UNIQUE(asset_id, period) 最左前缀覆盖）
DROP INDEX IF EXISTS "idx_fa_depreciation_records_asset";"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- 批次 92 P3-12/P3-13 回滚：恢复原外键行为 + 恢复冗余索引

-- 1. 删除 ON DELETE RESTRICT 外键
ALTER TABLE "fixed_asset_depreciation_records"
    DROP CONSTRAINT IF EXISTS "fixed_asset_depreciation_records_asset_id_fkey";

-- 2. 恢复默认外键（ON DELETE NO ACTION）
ALTER TABLE "fixed_asset_depreciation_records"
    ADD CONSTRAINT "fixed_asset_depreciation_records_asset_id_fkey"
    FOREIGN KEY ("asset_id") REFERENCES "fixed_assets"("id");

-- 3. 恢复冗余单列索引
CREATE INDEX IF NOT EXISTS "idx_fa_depreciation_records_asset"
    ON "fixed_asset_depreciation_records"("asset_id");"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
