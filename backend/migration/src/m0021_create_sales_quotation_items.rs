//! 销售报价单明细表迁移
//!
//! 创建时间: 2026-06-18
//! 关联计划: 2026-06-17-p12-batch1-quotation-port-plan.md PR-1
//!
//! 注意：明细表 ID 与 quotation_id 均使用 SERIAL/INTEGER，与主表保持一致；
//! product_id / color_id 引用 main 已有 products / product_colors 表（i32）。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql =
            include_str!("../../migrations/20260618000002_create_sales_quotation_items/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql =
            include_str!("../../migrations/20260618000002_create_sales_quotation_items/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
