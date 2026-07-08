//! 删除 audit_alert_rules 表（批次 202 P1-2）
//!
//! 创建时间: 2026-07-08
//! 关联修复: v12 复审 P1-2 — audit_alert_rule 模型死代码清理
//!
//! audit_alert_rules 表在 m0005 中创建，但对应的模型从未被任何
//! handler/service/route 引用（grep "use crate::models::audit_alert_rule"
//! 无匹配），且审计告警功能不在项目规划文档中，属于遗留死代码。
//! 本迁移删除该表，同步删除对应的 Rust 模型文件。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260708000002_drop_audit_alert_rules/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql =
            include_str!("../../migrations/20260708000002_drop_audit_alert_rules/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
