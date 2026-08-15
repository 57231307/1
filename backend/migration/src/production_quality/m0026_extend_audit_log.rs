//! 审计日志增强迁移
//!
//! 创建时间: 2026-06-18
//! 关联计划: 2026-06-18-p13-batch1-comprehensive-plan.md §2.1
//!
//! 在 main 已有的 audit_logs 表（m0001 创建）基础上，
//! 增量添加 operation_type / severity / request_id / before_snapshot / after_snapshot 五列，
//! 全部使用 ADD COLUMN IF NOT EXISTS 防止迁移重入。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260618000004_extend_audit_log/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260618000004_extend_audit_log/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
