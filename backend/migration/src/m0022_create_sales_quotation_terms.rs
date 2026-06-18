//! 销售报价单贸易条款表迁移
//!
//! 创建时间: 2026-06-18
//! 关联计划: 2026-06-17-p12-batch1-quotation-port-plan.md PR-1
//!
//! 存储报价单中各类贸易条款（物流/付款/样品/检验），
//! 与主表 ON DELETE CASCADE，确保主表删除时条款随之清理。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql =
            include_str!("../../migrations/20260618000003_create_sales_quotation_terms/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql =
            include_str!("../../migrations/20260618000003_create_sales_quotation_terms/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
