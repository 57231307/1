//! 财务/合规/审计
//!
//! 合并自: 10 个迁移文件

use sea_orm_migration::prelude::*;

// 导入所有迁移模块
mod m0051_add_data_scope_to_roles;
mod m0052_create_role_conflicts;
mod m0053_create_permission_change_audit;
mod m0054_enable_rls_policies;
mod m0055_create_export_approval_request;
mod m0056_add_condition_to_audit_logs;
mod m0057_create_color_card_issues_and_stock_fields;
mod m0058_create_bulk_color_approval;
mod m0059_add_rework_order_fields;
mod m0060_create_quality_8d_reports;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次执行所有迁移
        m0051_add_data_scope_to_roles::Migration.up(manager).await?;
        m0052_create_role_conflicts::Migration.up(manager).await?;
        m0053_create_permission_change_audit::Migration
            .up(manager)
            .await?;
        m0054_enable_rls_policies::Migration.up(manager).await?;
        m0055_create_export_approval_request::Migration
            .up(manager)
            .await?;
        m0056_add_condition_to_audit_logs::Migration
            .up(manager)
            .await?;
        m0057_create_color_card_issues_and_stock_fields::Migration
            .up(manager)
            .await?;
        m0058_create_bulk_color_approval::Migration
            .up(manager)
            .await?;
        m0059_add_rework_order_fields::Migration.up(manager).await?;
        m0060_create_quality_8d_reports::Migration
            .up(manager)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次回滚所有迁移（逆序）
        m0060_create_quality_8d_reports::Migration
            .down(manager)
            .await?;
        m0059_add_rework_order_fields::Migration
            .down(manager)
            .await?;
        m0058_create_bulk_color_approval::Migration
            .down(manager)
            .await?;
        m0057_create_color_card_issues_and_stock_fields::Migration
            .down(manager)
            .await?;
        m0056_add_condition_to_audit_logs::Migration
            .down(manager)
            .await?;
        m0055_create_export_approval_request::Migration
            .down(manager)
            .await?;
        m0054_enable_rls_policies::Migration.down(manager).await?;
        m0053_create_permission_change_audit::Migration
            .down(manager)
            .await?;
        m0052_create_role_conflicts::Migration.down(manager).await?;
        m0051_add_data_scope_to_roles::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}
