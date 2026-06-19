pub use sea_orm_migration::prelude::*;

pub mod m0001_initial_schema;
pub mod m0002_add_crm_and_greige_tables;
pub mod m0003_add_dye_tables;
pub mod m0004_add_field_permissions;
pub mod m0005_add_basic_data_and_system_tables;
pub mod m0006_add_general_ledger_and_finance_base;
pub mod m0007_add_mrp_production_bom;
pub mod m0008_add_supplier_and_product_extensions;
pub mod m0009_add_purchase_extensions;
pub mod m0010_add_inventory_extensions;
pub mod m0011_add_sales_and_logistics_extensions;
pub mod m0012_add_ap_ar_finance_analysis;
pub mod m0013_add_business_process_and_traceability;
pub mod m0014_add_saas_notification_report_email_oa;
pub mod m0015_add_opportunity_id_to_sales_orders;
pub mod m0016_add_version_to_inventory_stocks;
pub mod m0017_add_crm_supplier_tables;
pub mod m0018_add_finance_tables;
pub mod m0019_fix_schema_model_sync;
pub mod m0020_create_sales_quotations;
pub mod m0021_create_sales_quotation_items;
pub mod m0022_create_sales_quotation_terms;
pub mod m0023_create_product_color_prices;
pub mod m0024_p4_1_perf_indexes;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m0001_initial_schema::Migration),
            Box::new(m0002_add_crm_and_greige_tables::Migration),
            Box::new(m0003_add_dye_tables::Migration),
            Box::new(m0004_add_field_permissions::Migration),
            Box::new(m0005_add_basic_data_and_system_tables::Migration),
            Box::new(m0006_add_general_ledger_and_finance_base::Migration),
            Box::new(m0007_add_mrp_production_bom::Migration),
            Box::new(m0008_add_supplier_and_product_extensions::Migration),
            Box::new(m0009_add_purchase_extensions::Migration),
            Box::new(m0010_add_inventory_extensions::Migration),
            Box::new(m0011_add_sales_and_logistics_extensions::Migration),
            Box::new(m0012_add_ap_ar_finance_analysis::Migration),
            Box::new(m0013_add_business_process_and_traceability::Migration),
            Box::new(m0014_add_saas_notification_report_email_oa::Migration),
            Box::new(m0015_add_opportunity_id_to_sales_orders::Migration),
            Box::new(m0016_add_version_to_inventory_stocks::Migration),
            Box::new(m0017_add_crm_supplier_tables::Migration),
            Box::new(m0018_add_finance_tables::Migration),
            Box::new(m0019_fix_schema_model_sync::Migration),
            Box::new(m0020_create_sales_quotations::Migration),
            Box::new(m0021_create_sales_quotation_items::Migration),
            Box::new(m0022_create_sales_quotation_terms::Migration),
            Box::new(m0023_create_product_color_prices::Migration),
            Box::new(m0024_p4_1_perf_indexes::Migration),
            Box::new(m0025_extend_audit_log::Migration),
            Box::new(m0026_enable_pg_stat_statements::Migration),
            Box::new(m0027_create_slow_query_log::Migration),
        ]
    }
}
