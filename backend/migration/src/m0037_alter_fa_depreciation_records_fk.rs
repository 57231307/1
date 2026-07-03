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
        let sql = include_str!("../../migrations/20260703000006_alter_fa_depreciation_records_fk/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260703000006_alter_fa_depreciation_records_fk/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
