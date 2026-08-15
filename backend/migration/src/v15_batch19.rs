//! V15 批次19
//!
//! 合并自: 10 个迁移文件

use sea_orm_migration::prelude::*;

// 导入所有迁移模块
mod m0081_create_fixed_asset_counts;
mod m0082_create_customer_team_and_share;
mod m0083_create_report_template_versions;
mod m0084_add_color_card_issue_export_permissions;
mod m0085_create_bulk_color_approval_history;
mod m0086_add_inspection_id_to_outsourcing_receipt;
mod m0087_batch19_custom_order_aftersales_logistics_incoterms;
mod m0088_audit_log_export_log;
mod m0089_add_rework_cost_to_dye_batch_rework;
mod m0090_create_dye_vat_occupation;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次执行所有迁移
        m0081_create_fixed_asset_counts::Migration.up(manager).await?;
        m0082_create_customer_team_and_share::Migration.up(manager).await?;
        m0083_create_report_template_versions::Migration.up(manager).await?;
        m0084_add_color_card_issue_export_permissions::Migration.up(manager).await?;
        m0085_create_bulk_color_approval_history::Migration.up(manager).await?;
        m0086_add_inspection_id_to_outsourcing_receipt::Migration.up(manager).await?;
        m0087_batch19_custom_order_aftersales_logistics_incoterms::Migration.up(manager).await?;
        m0088_audit_log_export_log::Migration.up(manager).await?;
        m0089_add_rework_cost_to_dye_batch_rework::Migration.up(manager).await?;
        m0090_create_dye_vat_occupation::Migration.up(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次回滚所有迁移（逆序）
        m0090_create_dye_vat_occupation::Migration.down(manager).await?;
        m0089_add_rework_cost_to_dye_batch_rework::Migration.down(manager).await?;
        m0088_audit_log_export_log::Migration.down(manager).await?;
        m0087_batch19_custom_order_aftersales_logistics_incoterms::Migration.down(manager).await?;
        m0086_add_inspection_id_to_outsourcing_receipt::Migration.down(manager).await?;
        m0085_create_bulk_color_approval_history::Migration.down(manager).await?;
        m0084_add_color_card_issue_export_permissions::Migration.down(manager).await?;
        m0083_create_report_template_versions::Migration.down(manager).await?;
        m0082_create_customer_team_and_share::Migration.down(manager).await?;
        m0081_create_fixed_asset_counts::Migration.down(manager).await?;
        Ok(())
    }
}
