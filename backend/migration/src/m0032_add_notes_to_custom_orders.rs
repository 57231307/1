//! custom_orders 备注列迁移（批次 88 PH-1）
//!
//! 创建时间: 2026-07-03
//! 关联修复: 占位符 PH-1 — DTO 有 notes 字段但 service 层 `let _ = v;` 丢弃
//!
//! 向 custom_orders 表添加 notes 列（TEXT，可选），存储订单备注。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260703000001_add_notes_to_custom_orders/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260703000001_add_notes_to_custom_orders/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
