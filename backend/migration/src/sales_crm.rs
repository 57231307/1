//! 销售报价与CRM扩展
//!
//! 合并自: 14 个迁移文件

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // === m0021_create_sales_quotations.rs ===
let sql = include_str!("../../migrations/20260616000001_create_sales_quotations/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0022_create_sales_quotation_items.rs ===
let sql =
            include_str!("../../migrations/20260616000002_create_sales_quotation_items/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0023_create_sales_quotation_terms.rs ===
let sql =
            include_str!("../../migrations/20260616000003_create_sales_quotation_terms/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0024_create_product_color_prices.rs ===
let sql =
            include_str!("../../migrations/20260616000004_create_product_color_prices/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0031_add_signature_to_omni_audit_logs.rs ===
let sql =
            include_str!("../../migrations/20260701000001_add_signature_to_omni_audit_logs/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0032_add_notes_to_custom_orders.rs ===
let sql = include_str!("../../migrations/20260703000001_add_notes_to_custom_orders/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0033_add_gain_loss_to_fixed_asset_disposals.rs ===
let sql = include_str!(
            "../../migrations/20260703000002_add_gain_loss_to_fixed_asset_disposals/up.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0034_create_fixed_asset_depreciation_records.rs ===
let sql = include_str!(
            "../../migrations/20260703000003_create_fixed_asset_depreciation_records/up.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0035_create_customer_contacts.rs ===
let sql = include_str!("../../migrations/20260703000004_create_customer_contacts/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0036_create_api_endpoints.rs ===
let sql = include_str!("../../migrations/20260703000005_create_api_endpoints/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0037_alter_fa_depreciation_records_fk.rs ===
let sql =
            include_str!("../../migrations/20260703000006_alter_fa_depreciation_records_fk/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0038_add_notes_to_ar_reconciliations.rs ===
let sql =
            include_str!("../../migrations/20260704000001_add_notes_to_ar_reconciliations/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0039_add_created_by_to_api_keys.rs ===
let sql = include_str!("../../migrations/20260705000001_add_created_by_to_api_keys/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0040_create_crm_tags.rs ===
let sql = include_str!("../../migrations/20260705000002_create_crm_tags/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // === m0021_create_sales_quotations.rs ===
let sql = include_str!("../../migrations/20260616000001_create_sales_quotations/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0022_create_sales_quotation_items.rs ===
let sql =
            include_str!("../../migrations/20260616000002_create_sales_quotation_items/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0023_create_sales_quotation_terms.rs ===
let sql =
            include_str!("../../migrations/20260616000003_create_sales_quotation_terms/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0024_create_product_color_prices.rs ===
let sql =
            include_str!("../../migrations/20260616000004_create_product_color_prices/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0031_add_signature_to_omni_audit_logs.rs ===
let sql = include_str!(
            "../../migrations/20260701000001_add_signature_to_omni_audit_logs/down.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0032_add_notes_to_custom_orders.rs ===
let sql =
            include_str!("../../migrations/20260703000001_add_notes_to_custom_orders/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0033_add_gain_loss_to_fixed_asset_disposals.rs ===
let sql = include_str!(
            "../../migrations/20260703000002_add_gain_loss_to_fixed_asset_disposals/down.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0034_create_fixed_asset_depreciation_records.rs ===
let sql = include_str!(
            "../../migrations/20260703000003_create_fixed_asset_depreciation_records/down.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0035_create_customer_contacts.rs ===
let sql = include_str!("../../migrations/20260703000004_create_customer_contacts/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0036_create_api_endpoints.rs ===
let sql = include_str!("../../migrations/20260703000005_create_api_endpoints/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0037_alter_fa_depreciation_records_fk.rs ===
let sql = include_str!(
            "../../migrations/20260703000006_alter_fa_depreciation_records_fk/down.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0038_add_notes_to_ar_reconciliations.rs ===
let sql = include_str!(
            "../../migrations/20260704000001_add_notes_to_ar_reconciliations/down.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0039_add_created_by_to_api_keys.rs ===
let sql =
            include_str!("../../migrations/20260705000001_add_created_by_to_api_keys/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        // === m0040_create_crm_tags.rs ===
let sql = include_str!("../../migrations/20260705000002_create_crm_tags/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}


