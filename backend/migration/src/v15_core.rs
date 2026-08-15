//! V15 核心功能
//!
//! 合并自: 15 个迁移文件

use sea_orm_migration::prelude::*;

// 导入所有迁移模块
mod m0061_create_bad_debt_provisions;
mod m0062_create_bad_debt_writeoffs;
mod m0063_create_collection_tasks;
mod m0064_create_finance_alerts;
mod m0065_add_custom_order_sample_quotation_fields;
mod m0066_add_after_sales_quality_issue_id;
mod m0067_add_logistics_waybill_sign_fields;
mod m0068_create_material_shortage_tables;
mod m0069_create_supplier_evaluation_records;
mod m0070_create_user_role;
mod m0071_add_sales_order_id_to_color_card_issues;
mod m0072_create_permission_delegations;
mod m0073_create_role_relations;
mod m0074_v15_p1_integrate_sql_migrations;
mod m0075_add_email_queue_fields;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次执行所有迁移
        m0061_create_bad_debt_provisions::Migration.up(manager).await?;
        m0062_create_bad_debt_writeoffs::Migration.up(manager).await?;
        m0063_create_collection_tasks::Migration.up(manager).await?;
        m0064_create_finance_alerts::Migration.up(manager).await?;
        m0065_add_custom_order_sample_quotation_fields::Migration.up(manager).await?;
        m0066_add_after_sales_quality_issue_id::Migration.up(manager).await?;
        m0067_add_logistics_waybill_sign_fields::Migration.up(manager).await?;
        m0068_create_material_shortage_tables::Migration.up(manager).await?;
        m0069_create_supplier_evaluation_records::Migration.up(manager).await?;
        m0070_create_user_role::Migration.up(manager).await?;
        m0071_add_sales_order_id_to_color_card_issues::Migration.up(manager).await?;
        m0072_create_permission_delegations::Migration.up(manager).await?;
        m0073_create_role_relations::Migration.up(manager).await?;
        m0074_v15_p1_integrate_sql_migrations::Migration.up(manager).await?;
        m0075_add_email_queue_fields::Migration.up(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次回滚所有迁移（逆序）
        m0075_add_email_queue_fields::Migration.down(manager).await?;
        m0074_v15_p1_integrate_sql_migrations::Migration.down(manager).await?;
        m0073_create_role_relations::Migration.down(manager).await?;
        m0072_create_permission_delegations::Migration.down(manager).await?;
        m0071_add_sales_order_id_to_color_card_issues::Migration.down(manager).await?;
        m0070_create_user_role::Migration.down(manager).await?;
        m0069_create_supplier_evaluation_records::Migration.down(manager).await?;
        m0068_create_material_shortage_tables::Migration.down(manager).await?;
        m0067_add_logistics_waybill_sign_fields::Migration.down(manager).await?;
        m0066_add_after_sales_quality_issue_id::Migration.down(manager).await?;
        m0065_add_custom_order_sample_quotation_fields::Migration.down(manager).await?;
        m0064_create_finance_alerts::Migration.down(manager).await?;
        m0063_create_collection_tasks::Migration.down(manager).await?;
        m0062_create_bad_debt_writeoffs::Migration.down(manager).await?;
        m0061_create_bad_debt_provisions::Migration.down(manager).await?;
        Ok(())
    }
}
