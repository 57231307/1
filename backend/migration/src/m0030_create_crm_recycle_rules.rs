//! CRM 公海回收规则表迁移
//!
//! 创建时间: 2026-06-29
//! 关联修复: 批次 23 v5 P0-4 — CRM 回收规则内存存储导致重启丢失
//!
//! 本迁移创建 `crm_recycle_rules` 表，并将原本硬编码在
//! `handlers/missing_handlers.rs` 中的 3 条初始规则写入数据库：
//! - 30 天标准回收规则
//! - 90 天高价值客户延长规则
//! - 7 天快速回收规则（默认禁用）

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260629000001_create_crm_recycle_rules/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql =
            include_str!("../../migrations/20260629000001_create_crm_recycle_rules/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
