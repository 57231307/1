//! 启用 pg_stat_statements 扩展迁移
//!
//! 创建时间: 2026-06-18
//! 关联计划: 2026-06-18-p13-batch1-comprehensive-plan.md §2.2
//!
//! 通过 `CREATE EXTENSION IF NOT EXISTS pg_stat_statements` 启用慢查询审计的
//! 数据源视图。CI 环境如未预装共享库，应用层会自动降级（采集任务启动失败时
//! 仅记录日志，不阻断 main）。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260618000005_enable_pg_stat_statements/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql =
            include_str!("../../migrations/20260618000005_enable_pg_stat_statements/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
