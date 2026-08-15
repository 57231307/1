use sea_orm_migration::prelude::*;

pub mod m0007_add_mrp_production_bom;
pub mod m0008_add_supplier_and_product_extensions;
pub mod m0009_add_purchase_extensions;
pub mod m0010_add_inventory_extensions;
pub mod m0011_add_sales_and_logistics_extensions;
pub mod m0012_add_ap_ar_finance_analysis;
pub mod m0013_add_business_process_and_traceability;
pub mod m0014_add_saas_notification_report_email_oa;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0007_add_mrp_production_bom::Migration.up(manager).await?;
        m0008_add_supplier_and_product_extensions::Migration.up(manager).await?;
        m0009_add_purchase_extensions::Migration.up(manager).await?;
        m0010_add_inventory_extensions::Migration.up(manager).await?;
        m0011_add_sales_and_logistics_extensions::Migration.up(manager).await?;
        m0012_add_ap_ar_finance_analysis::Migration.up(manager).await?;
        m0013_add_business_process_and_traceability::Migration.up(manager).await?;
        m0014_add_saas_notification_report_email_oa::Migration.up(manager).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
