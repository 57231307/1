//! webhooks 表 user_id 列迁移（批次 320 v9 中风险修复 M-4）
//!
//! 创建时间: 2026-07-12
//! 关联修复: v9 中风险 M-4 — webhook 端点 IDOR，无所有权校验
//!
//! 向 webhooks 表添加 user_id（INTEGER，可空）列，用于记录 webhook 所有者：
//! - NULL：系统级 webhook（历史数据，所有认证用户可访问，向后兼容）
//! - 非 NULL：用户私有 webhook，仅所有者可操作

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!(
            "../../migrations/20260712000001_add_user_id_to_webhooks/up.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!(
            "../../migrations/20260712000001_add_user_id_to_webhooks/down.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
