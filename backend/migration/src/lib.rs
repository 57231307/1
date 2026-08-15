//! 数据库迁移模块

use sea_orm_migration::prelude::*;

// 合并后的迁移模块
pub mod m0001_core_schema;
pub mod m0002_business_tables;
pub mod m0003_fixes_enhancements;
pub mod m0004_sales_quotation;
pub mod m0005_crm_extensions;
pub mod m0006_production_quality;
pub mod m0007_finance_compliance;
pub mod m0008_v15_core;
pub mod m0009_v15_batch18;
pub mod m0010_v15_batch19;
pub mod m0011_v15_extensions;
pub mod m0012_v15_final;

// 原始模块（被合并模块引用）
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
pub mod m0019_add_missing_columns;
pub mod m0020_fix_schema_model_sync;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m0001_core_schema::Migration),
            Box::new(m0002_business_tables::Migration),
            Box::new(m0003_fixes_enhancements::Migration),
            Box::new(m0004_sales_quotation::Migration),
            Box::new(m0005_crm_extensions::Migration),
            Box::new(m0006_production_quality::Migration),
            Box::new(m0007_finance_compliance::Migration),
            Box::new(m0008_v15_core::Migration),
            Box::new(m0009_v15_batch18::Migration),
            Box::new(m0010_v15_batch19::Migration),
            Box::new(m0011_v15_extensions::Migration),
            Box::new(m0012_v15_final::Migration),
        ]
    }
}
