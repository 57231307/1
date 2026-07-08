//! users 表 password_changed_at 列迁移（批次 198 P0-2）
//!
//! 创建时间: 2026-07-08
//! 关联修复: v12 复审 P0-2 — PasswordPolicyService::is_expired 未接入登录流程
//!
//! 向 users 表添加 password_changed_at 列（TIMESTAMP WITH TIME ZONE，可空），
//! 作为密码过期策略的时间锚点。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!(
            "../../migrations/20260708000001_add_password_changed_at_to_users/up.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!(
            "../../migrations/20260708000001_add_password_changed_at_to_users/down.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
