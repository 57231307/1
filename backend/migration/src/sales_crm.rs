//! 销售报价与CRM扩展
//!
//! 合并自: 14 个迁移文件

use sea_orm_migration::prelude::*;

// 导入所有迁移模块
mod m0021_create_sales_quotations;
mod m0022_create_sales_quotation_items;
mod m0023_create_sales_quotation_terms;
mod m0024_create_product_color_prices;
mod m0031_add_signature_to_omni_audit_logs;
mod m0032_add_notes_to_custom_orders;
mod m0033_add_gain_loss_to_fixed_asset_disposals;
mod m0034_create_fixed_asset_depreciation_records;
mod m0035_create_customer_contacts;
mod m0036_create_api_endpoints;
mod m0037_alter_fa_depreciation_records_fk;
mod m0038_add_notes_to_ar_reconciliations;
mod m0039_add_created_by_to_api_keys;
mod m0040_create_crm_tags;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次执行所有迁移
        m0021_create_sales_quotations::Migration.up(manager).await?;
        m0022_create_sales_quotation_items::Migration.up(manager).await?;
        m0023_create_sales_quotation_terms::Migration.up(manager).await?;
        m0024_create_product_color_prices::Migration.up(manager).await?;
        m0031_add_signature_to_omni_audit_logs::Migration.up(manager).await?;
        m0032_add_notes_to_custom_orders::Migration.up(manager).await?;
        m0033_add_gain_loss_to_fixed_asset_disposals::Migration.up(manager).await?;
        m0034_create_fixed_asset_depreciation_records::Migration.up(manager).await?;
        m0035_create_customer_contacts::Migration.up(manager).await?;
        m0036_create_api_endpoints::Migration.up(manager).await?;
        m0037_alter_fa_depreciation_records_fk::Migration.up(manager).await?;
        m0038_add_notes_to_ar_reconciliations::Migration.up(manager).await?;
        m0039_add_created_by_to_api_keys::Migration.up(manager).await?;
        m0040_create_crm_tags::Migration.up(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次回滚所有迁移（逆序）
        m0040_create_crm_tags::Migration.down(manager).await?;
        m0039_add_created_by_to_api_keys::Migration.down(manager).await?;
        m0038_add_notes_to_ar_reconciliations::Migration.down(manager).await?;
        m0037_alter_fa_depreciation_records_fk::Migration.down(manager).await?;
        m0036_create_api_endpoints::Migration.down(manager).await?;
        m0035_create_customer_contacts::Migration.down(manager).await?;
        m0034_create_fixed_asset_depreciation_records::Migration.down(manager).await?;
        m0033_add_gain_loss_to_fixed_asset_disposals::Migration.down(manager).await?;
        m0032_add_notes_to_custom_orders::Migration.down(manager).await?;
        m0031_add_signature_to_omni_audit_logs::Migration.down(manager).await?;
        m0024_create_product_color_prices::Migration.down(manager).await?;
        m0023_create_sales_quotation_terms::Migration.down(manager).await?;
        m0022_create_sales_quotation_items::Migration.down(manager).await?;
        m0021_create_sales_quotations::Migration.down(manager).await?;
        Ok(())
    }
}
