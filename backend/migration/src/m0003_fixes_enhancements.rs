use sea_orm_migration::prelude::*;

pub mod m0015_add_opportunity_id_to_sales_orders;
pub mod m0016_add_version_to_inventory_stocks;
pub mod m0017_add_crm_supplier_tables;
pub mod m0018_add_finance_tables;
pub mod m0019_add_missing_columns;
pub mod m0020_fix_schema_model_sync;
pub mod m0021_create_sales_quotations;
pub mod m0022_create_sales_quotation_items;
pub mod m0023_create_sales_quotation_terms;
pub mod m0024_create_product_color_prices;
pub mod m0025_p4_1_perf_indexes;
pub mod m0026_extend_audit_log;
pub mod m0027_enable_pg_stat_statements;
pub mod m0028_create_slow_query_log;
pub mod m0029_drop_tenant_columns;
pub mod m0030_create_crm_recycle_rules;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0015_add_opportunity_id_to_sales_orders::Migration.up(manager).await?;
        m0016_add_version_to_inventory_stocks::Migration.up(manager).await?;
        m0017_add_crm_supplier_tables::Migration.up(manager).await?;
        m0018_add_finance_tables::Migration.up(manager).await?;
        m0019_add_missing_columns::Migration.up(manager).await?;
        m0020_fix_schema_model_sync::Migration.up(manager).await?;
        m0021_create_sales_quotations::Migration.up(manager).await?;
        m0022_create_sales_quotation_items::Migration.up(manager).await?;
        m0023_create_sales_quotation_terms::Migration.up(manager).await?;
        m0024_create_product_color_prices::Migration.up(manager).await?;
        m0025_p4_1_perf_indexes::Migration.up(manager).await?;
        m0026_extend_audit_log::Migration.up(manager).await?;
        m0027_enable_pg_stat_statements::Migration.up(manager).await?;
        m0028_create_slow_query_log::Migration.up(manager).await?;
        m0029_drop_tenant_columns::Migration.up(manager).await?;
        m0030_create_crm_recycle_rules::Migration.up(manager).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
