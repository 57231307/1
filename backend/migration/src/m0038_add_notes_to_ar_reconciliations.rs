//! ar_reconciliations 备注列迁移（批次 109 P1-1）
//!
//! 创建时间: 2026-07-04
//! 关联修复: v7 复审 P1-1 — DTO/Request 中 notes 字段已对外暴露但未持久化
//!
//! 向 ar_reconciliations 表添加 notes 列（TEXT，可选），存储对账单备注。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql =
            include_str!("../../migrations/20260704000001_add_notes_to_ar_reconciliations/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!(
            "../../migrations/20260704000001_add_notes_to_ar_reconciliations/down.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
