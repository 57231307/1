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
        let sql = r#"-- fixed_asset_disposals 处置损益列迁移（批次 88 PH-3）
-- 创建时间: 2026-07-03
-- 关联修复: 占位符 PH-3 — service 计算后 `let _disposal_gain_loss = ...` 丢弃
--
-- 向 fixed_asset_disposals 表添加 gain_loss 列（DECIMAL(15,2)，可选），
-- 存储处置损益 = disposal_amount - 处置时账面净值（正数为收益，负数为损失）。
-- 使用 ADD COLUMN IF NOT EXISTS 防止迁移重入。

ALTER TABLE "fixed_asset_disposals" ADD COLUMN IF NOT EXISTS "gain_loss" DECIMAL(15, 2);
COMMENT ON COLUMN "fixed_asset_disposals"."gain_loss" IS '处置损益 = disposal_amount - 处置时账面净值（正数为收益，负数为损失，批次 88 PH-3 占位符实现）';"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- fixed_asset_disposals 处置损益列回滚（批次 88 PH-3）
ALTER TABLE "fixed_asset_disposals" DROP COLUMN IF EXISTS "gain_loss";"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
