//! 创建 slow_query_log 表迁移
//!
//! 创建时间: 2026-06-18
//! 关联计划: 2026-06-18-p13-batch1-comprehensive-plan.md §2.2
//!
//! 存储后台采集任务（slow_query_collector）从 pg_stat_statements 视图
//! 拉取的慢查询记录，供前端运维页面（/system/slow-query）查询与统计。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260618000006_create_slow_query_log/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260618000006_create_slow_query_log/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
