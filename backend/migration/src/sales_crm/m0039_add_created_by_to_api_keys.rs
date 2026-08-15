//! api_keys 表 created_by 列迁移（批次 112 P1-9）
//!
//! 创建时间: 2026-07-05
//! 关联修复: v7 复审 P1-9 — api_keys 表无 created_by 列，handler 传 0 占位
//!
//! 向 api_keys 表添加 created_by 列（INTEGER，可空），存储 API 密钥创建者用户 ID。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260705000001_add_created_by_to_api_keys/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql =
            include_str!("../../migrations/20260705000001_add_created_by_to_api_keys/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
