use sea_orm_migration::prelude::*;

pub mod m0051_add_data_scope_to_roles;
pub mod m0052_create_role_conflicts;
pub mod m0053_create_permission_change_audit;
pub mod m0054_enable_rls_policies;
pub mod m0055_create_export_approval_request;
pub mod m0056_add_condition_to_audit_logs;
pub mod m0057_create_color_card_issues_and_stock_fields;
pub mod m0058_create_bulk_color_approval;
pub mod m0059_add_rework_order_fields;
pub mod m0060_create_quality_8d_reports;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0051_add_data_scope_to_roles::Migration.up(manager).await?;
        m0052_create_role_conflicts::Migration.up(manager).await?;
        m0053_create_permission_change_audit::Migration.up(manager).await?;
        m0054_enable_rls_policies::Migration.up(manager).await?;
        m0055_create_export_approval_request::Migration.up(manager).await?;
        m0056_add_condition_to_audit_logs::Migration.up(manager).await?;
        m0057_create_color_card_issues_and_stock_fields::Migration.up(manager).await?;
        m0058_create_bulk_color_approval::Migration.up(manager).await?;
        m0059_add_rework_order_fields::Migration.up(manager).await?;
        m0060_create_quality_8d_reports::Migration.up(manager).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
