use sea_orm_migration::prelude::*;

pub mod m0021_create_sales_quotations;
pub mod m0022_create_sales_quotation_items;
pub mod m0023_create_sales_quotation_terms;
pub mod m0024_create_product_color_prices;
pub mod m0031_add_signature_to_omni_audit_logs;
pub mod m0032_add_notes_to_custom_orders;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0021_create_sales_quotations::Migration.up(manager).await?;
        m0022_create_sales_quotation_items::Migration.up(manager).await?;
        m0023_create_sales_quotation_terms::Migration.up(manager).await?;
        m0024_create_product_color_prices::Migration.up(manager).await?;
        m0031_add_signature_to_omni_audit_logs::Migration.up(manager).await?;
        m0032_add_notes_to_custom_orders::Migration.up(manager).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
