//! 固定资产折旧期间记录表迁移（批次 88 PH-2）
//!
//! 创建时间: 2026-07-03
//! 关联修复: 占位符 PH-2 — service `period` 参数仅写日志，未按期间记录折旧
//!
//! 新建 fixed_asset_depreciation_records 表，按期间记录每笔折旧计提明细。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260703000003_create_fixed_asset_depreciation_records/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260703000003_create_fixed_asset_depreciation_records/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
