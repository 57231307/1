//! 修复与增强：字段补充/索引/约束
//!
//! 合并自: 16 个迁移文件

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // === m0015_add_opportunity_id_to_sales_orders.rs ===
let sql = include_str!(
            "../../migrations/20260527000011_add_opportunity_id_to_sales_orders/up.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        // === m0016_add_version_to_inventory_stocks.rs ===
let sql =
            include_str!("../../migrations/20260527000012_add_version_to_inventory_stocks/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        // === m0017_add_crm_supplier_tables.rs ===
let sql = include_str!("../../migrations/20260528000001_add_crm_supplier_tables/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        // === m0018_add_finance_tables.rs ===
let sql = include_str!("../../migrations/20260528000002_add_finance_tables/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        // === m0019_add_missing_columns.rs ===
let sql = include_str!("../../migrations/20260613000001_add_missing_columns/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        // === m0020_fix_schema_model_sync.rs ===
let sql = include_str!("../../migrations/20260613000002_fix_schema_model_sync/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        // === m0021_create_sales_quotations.rs ===
let sql = include_str!("../../migrations/20260616000001_create_sales_quotations/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        // === m0022_create_sales_quotation_items.rs ===
let sql =
            include_str!("../../migrations/20260616000002_create_sales_quotation_items/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        // === m0023_create_sales_quotation_terms.rs ===
let sql =
            include_str!("../../migrations/20260616000003_create_sales_quotation_terms/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        // === m0024_create_product_color_prices.rs ===
let sql =
            include_str!("../../migrations/20260616000004_create_product_color_prices/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        // === m0025_p4_1_perf_indexes.rs ===
// 批次 190 修复：所有 tenant_id 索引已删除（引用不存在的列）
        // 保留空 up 实现，避免破坏迁移历史顺序
        Ok(())
        // === m0026_extend_audit_log.rs ===
let sql = include_str!("../../migrations/20260618000004_extend_audit_log/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        // === m0027_enable_pg_stat_statements.rs ===
let sql = include_str!("../../migrations/20260618000005_enable_pg_stat_statements/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        // === m0028_create_slow_query_log.rs ===
let sql = include_str!("../../migrations/20260618000006_create_slow_query_log/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        // === m0029_drop_tenant_columns.rs ===
let sql = include_str!("../../migrations/20260628000001_drop_tenant_columns/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        // === m0030_create_crm_recycle_rules.rs ===
let sql = include_str!("../../migrations/20260629000001_create_crm_recycle_rules/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
