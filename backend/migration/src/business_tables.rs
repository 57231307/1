//! 业务表：生产/采购/库存/销售/财务
//!
//! 合并自: 8 个迁移文件

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // === m0007_add_mrp_production_bom.rs ===
let sql = include_str!("../../migrations/20260527000003_add_mrp_production_bom/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0008_add_supplier_and_product_extensions.rs ===
let sql = include_str!(
            "../../migrations/20260527000004_add_supplier_and_product_extensions/up.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0009_add_purchase_extensions.rs ===
let sql = include_str!("../../migrations/20260527000005_add_purchase_extensions/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0010_add_inventory_extensions.rs ===
let sql = include_str!("../../migrations/20260527000006_add_inventory_extensions/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0011_add_sales_and_logistics_extensions.rs ===
let sql = include_str!(
            "../../migrations/20260527000007_add_sales_and_logistics_extensions/up.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0012_add_ap_ar_finance_analysis.rs ===
let sql = include_str!("../../migrations/20260527000008_add_ap_ar_finance_analysis/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0013_add_business_process_and_traceability.rs ===
let sql = include_str!(
            "../../migrations/20260527000009_add_business_process_and_traceability/up.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0014_add_saas_notification_report_email_oa.rs ===
let sql = include_str!(
            "../../migrations/20260527000010_add_saas_notification_report_email_oa/up.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // === m0007_add_mrp_production_bom.rs ===
let sql = include_str!("../../migrations/20260527000003_add_mrp_production_bom/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0008_add_supplier_and_product_extensions.rs ===
let sql = include_str!(
            "../../migrations/20260527000004_add_supplier_and_product_extensions/down.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0009_add_purchase_extensions.rs ===
let sql = include_str!("../../migrations/20260527000005_add_purchase_extensions/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0010_add_inventory_extensions.rs ===
let sql = include_str!("../../migrations/20260527000006_add_inventory_extensions/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0011_add_sales_and_logistics_extensions.rs ===
let sql = include_str!(
            "../../migrations/20260527000007_add_sales_and_logistics_extensions/down.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0012_add_ap_ar_finance_analysis.rs ===
let sql =
            include_str!("../../migrations/20260527000008_add_ap_ar_finance_analysis/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0013_add_business_process_and_traceability.rs ===
let sql = include_str!(
            "../../migrations/20260527000009_add_business_process_and_traceability/down.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0014_add_saas_notification_report_email_oa.rs ===
let sql = include_str!(
            "../../migrations/20260527000010_add_saas_notification_report_email_oa/down.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}


