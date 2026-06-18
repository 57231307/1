//! 销售报价单主表迁移
//!
//! 创建时间: 2026-06-18
//! 关联计划: 2026-06-17-p12-batch1-quotation-port-plan.md PR-1
//!
//! 注意：与 test 分支同源迁移在字段类型上有适配
//! - 主键 ID 调整 SERIAL (i32) 以匹配 main 风格（SALES_ORDER 等主表）
//! - 引用 main 现有表的外键使用 INTEGER 与引用表保持一致
//! - quotation_status 枚举按任务规范：DRAFT / SUBMITTED / APPROVED / REJECTED / CONVERTED / CANCELLED / EXPIRED

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260618000001_create_sales_quotations/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260618000001_create_sales_quotations/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
