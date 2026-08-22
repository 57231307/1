//! V15 最终完善
//!
//! 合并自: 11 个迁移文件

use sea_orm_migration::prelude::*;

// 导入所有迁移模块
mod m0106_batch_dye_lot_unique_constraint;
mod m0107_add_color_card_capability_fields;
mod m0108_create_customer_addresses;
mod m0109_add_customer_special_process;
mod m0110_create_aging_grade_configs;
mod m0111_create_industry_benchmark_configs;
mod m0112_add_accounting_period_close_fields;
mod m0113_add_fixed_asset_depreciation_start_date;
mod m0114_add_customer_source_fields;
mod m0115_add_crm_lead_custom_fields;
mod m0116_create_long_running_tasks;
mod m0117_add_defect_type_to_quality_inspection_records;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次执行所有迁移
        m0106_batch_dye_lot_unique_constraint::Migration
            .up(manager)
            .await?;
        m0107_add_color_card_capability_fields::Migration
            .up(manager)
            .await?;
        m0108_create_customer_addresses::Migration
            .up(manager)
            .await?;
        m0109_add_customer_special_process::Migration
            .up(manager)
            .await?;
        m0110_create_aging_grade_configs::Migration
            .up(manager)
            .await?;
        m0111_create_industry_benchmark_configs::Migration
            .up(manager)
            .await?;
        m0112_add_accounting_period_close_fields::Migration
            .up(manager)
            .await?;
        m0113_add_fixed_asset_depreciation_start_date::Migration
            .up(manager)
            .await?;
        m0114_add_customer_source_fields::Migration
            .up(manager)
            .await?;
        m0115_add_crm_lead_custom_fields::Migration
            .up(manager)
            .await?;
        m0116_create_long_running_tasks::Migration
            .up(manager)
            .await?;
        m0117_add_defect_type_to_quality_inspection_records::Migration
            .up(manager)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次回滚所有迁移（逆序）
        m0117_add_defect_type_to_quality_inspection_records::Migration
            .down(manager)
            .await?;
        m0116_create_long_running_tasks::Migration
            .down(manager)
            .await?;
        m0115_add_crm_lead_custom_fields::Migration
            .down(manager)
            .await?;
        m0114_add_customer_source_fields::Migration
            .down(manager)
            .await?;
        m0113_add_fixed_asset_depreciation_start_date::Migration
            .down(manager)
            .await?;
        m0112_add_accounting_period_close_fields::Migration
            .down(manager)
            .await?;
        m0111_create_industry_benchmark_configs::Migration
            .down(manager)
            .await?;
        m0110_create_aging_grade_configs::Migration
            .down(manager)
            .await?;
        m0109_add_customer_special_process::Migration
            .down(manager)
            .await?;
        m0108_create_customer_addresses::Migration
            .down(manager)
            .await?;
        m0107_add_color_card_capability_fields::Migration
            .down(manager)
            .await?;
        m0106_batch_dye_lot_unique_constraint::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}
