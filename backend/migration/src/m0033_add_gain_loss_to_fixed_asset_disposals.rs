//! fixed_asset_disposals 处置损益列迁移（批次 88 PH-3）
//!
//! 创建时间: 2026-07-03
//! 关联修复: 占位符 PH-3 — service 计算后 `let _disposal_gain_loss = ...` 丢弃
//!
//! 向 fixed_asset_disposals 表添加 gain_loss 列（DECIMAL(15,2)，可选），
//! 存储处置损益 = disposal_amount - 处置时账面净值。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260703000002_add_gain_loss_to_fixed_asset_disposals/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260703000002_add_gain_loss_to_fixed_asset_disposals/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
