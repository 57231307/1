use sea_orm_migration::prelude::*;

pub mod m0030_create_crm_recycle_rules;
pub mod m0033_add_gain_loss_to_fixed_asset_disposals;
pub mod m0034_create_fixed_asset_depreciation_records;
pub mod m0035_create_customer_contacts;
pub mod m0036_create_api_endpoints;
pub mod m0037_alter_fa_depreciation_records_fk;
pub mod m0038_add_notes_to_ar_reconciliations;
pub mod m0039_add_created_by_to_api_keys;
pub mod m0040_create_crm_tags;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0030_create_crm_recycle_rules::Migration.up(manager).await?;
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

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
