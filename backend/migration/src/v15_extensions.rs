//! V15 扩展功能
//!
//! 合并自: 15 个迁移文件

use sea_orm_migration::prelude::*;

// 导入所有迁移模块
mod m0091_create_device_connection;
mod m0092_create_period_adjustment_record;
mod m0093_add_category_id_to_suppliers;
mod m0094_add_processor_fields_to_suppliers;
mod m0095_create_sales_contract_items;
mod m0096_create_period_report_snapshot;
mod m0097_create_aging_alert_rules;
mod m0098_budget_asset_enhancements;
mod m0099_budget_impairment_depreciation;
mod m0100_crm_lead_enhancements;
mod m0101_crm_opp_pool_enhancements;
mod m0102_crm_data_permission_clv;
mod m0103_api_deprecation_fields;
mod m0104_greige_fabric_tracing_fields;
mod m0105_piece_split_original_length;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次执行所有迁移
        m0091_create_device_connection::Migration.up(manager).await?;
        m0092_create_period_adjustment_record::Migration.up(manager).await?;
        m0093_add_category_id_to_suppliers::Migration.up(manager).await?;
        m0094_add_processor_fields_to_suppliers::Migration.up(manager).await?;
        m0095_create_sales_contract_items::Migration.up(manager).await?;
        m0096_create_period_report_snapshot::Migration.up(manager).await?;
        m0097_create_aging_alert_rules::Migration.up(manager).await?;
        m0098_budget_asset_enhancements::Migration.up(manager).await?;
        m0099_budget_impairment_depreciation::Migration.up(manager).await?;
        m0100_crm_lead_enhancements::Migration.up(manager).await?;
        m0101_crm_opp_pool_enhancements::Migration.up(manager).await?;
        m0102_crm_data_permission_clv::Migration.up(manager).await?;
        m0103_api_deprecation_fields::Migration.up(manager).await?;
        m0104_greige_fabric_tracing_fields::Migration.up(manager).await?;
        m0105_piece_split_original_length::Migration.up(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次回滚所有迁移（逆序）
        m0105_piece_split_original_length::Migration.down(manager).await?;
        m0104_greige_fabric_tracing_fields::Migration.down(manager).await?;
        m0103_api_deprecation_fields::Migration.down(manager).await?;
        m0102_crm_data_permission_clv::Migration.down(manager).await?;
        m0101_crm_opp_pool_enhancements::Migration.down(manager).await?;
        m0100_crm_lead_enhancements::Migration.down(manager).await?;
        m0099_budget_impairment_depreciation::Migration.down(manager).await?;
        m0098_budget_asset_enhancements::Migration.down(manager).await?;
        m0097_create_aging_alert_rules::Migration.down(manager).await?;
        m0096_create_period_report_snapshot::Migration.down(manager).await?;
        m0095_create_sales_contract_items::Migration.down(manager).await?;
        m0094_add_processor_fields_to_suppliers::Migration.down(manager).await?;
        m0093_add_category_id_to_suppliers::Migration.down(manager).await?;
        m0092_create_period_adjustment_record::Migration.down(manager).await?;
        m0091_create_device_connection::Migration.down(manager).await?;
        Ok(())
    }
}
