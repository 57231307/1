//! API 端点管理表迁移（批次 91 P0-1）
//!
//! 创建时间: 2026-07-03
//! 关联修复: P0-1 — api_gateway 11 端点占位，endpoints CRUD 需新建表
//!
//! 新建 api_endpoints 表，管理 API 网关暴露的端点元数据。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260703000005_create_api_endpoints/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260703000005_create_api_endpoints/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
