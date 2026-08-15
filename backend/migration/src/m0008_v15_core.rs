use sea_orm_migration::prelude::*;

pub mod m0061_create_bad_debt_provisions;
pub mod m0062_create_bad_debt_writeoffs;
pub mod m0063_create_collection_tasks;
pub mod m0064_create_finance_alerts;
pub mod m0065_add_custom_order_sample_quotation_fields;
pub mod m0066_add_after_sales_quality_issue_id;
pub mod m0067_add_logistics_waybill_sign_fields;
pub mod m0068_create_material_shortage_tables;
pub mod m0069_create_supplier_evaluation_records;
pub mod m0070_create_user_role;
pub mod m0071_add_sales_order_id_to_color_card_issues;
pub mod m0072_create_permission_delegations;
pub mod m0073_create_role_relations;
pub mod m0074_v15_p1_integrate_sql_migrations;
pub mod m0075_add_email_queue_fields;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
