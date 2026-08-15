use sea_orm_migration::prelude::*;

mod m0106_batch_dye_lot_unique_constraint;
mod m0107_add_color_card_capability_fields;
mod m0108_create_customer_addresses;
mod m0109_add_customer_special_process;
mod m0110_create_aging_grade_configs;
mod m0111_create_industry_benchmark_configs;
mod m0112_add_accounting_period_close_fields;
mod m0113_add_fixed_asset_depreciation_start_date;
mod m0114_add_customer_source_fields;
mod m0115_add_crm_lead_custom_fields;
mod m0116_create_long_running_tasks;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0106_batch_dye_lot_unique_constraint::Migration.up(manager).await?;
        m0107_add_color_card_capability_fields::Migration.up(manager).await?;
        m0108_create_customer_addresses::Migration.up(manager).await?;
        m0109_add_customer_special_process::Migration.up(manager).await?;
        m0110_create_aging_grade_configs::Migration.up(manager).await?;
        m0111_create_industry_benchmark_configs::Migration.up(manager).await?;
        m0112_add_accounting_period_close_fields::Migration.up(manager).await?;
        m0113_add_fixed_asset_depreciation_start_date::Migration.up(manager).await?;
        m0114_add_customer_source_fields::Migration.up(manager).await?;
        m0115_add_crm_lead_custom_fields::Migration.up(manager).await?;
        m0116_create_long_running_tasks::Migration.up(manager).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
