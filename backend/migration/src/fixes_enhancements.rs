//! 修复与增强
//!
//! 合并自: 6 个迁移文件

use sea_orm_migration::prelude::*;

// 导入所有迁移模块
mod m0015_add_opportunity_id_to_sales_orders;
mod m0016_add_version_to_inventory_stocks;
mod m0017_add_crm_supplier_tables;
mod m0018_add_finance_tables;
mod m0019_add_missing_columns;
mod m0020_fix_schema_model_sync;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次执行所有迁移
        m0015_add_opportunity_id_to_sales_orders::Migration
            .up(manager)
            .await?;
        m0016_add_version_to_inventory_stocks::Migration
            .up(manager)
            .await?;
        m0017_add_crm_supplier_tables::Migration.up(manager).await?;
        m0018_add_finance_tables::Migration.up(manager).await?;
        m0019_add_missing_columns::Migration.up(manager).await?;
        m0020_fix_schema_model_sync::Migration.up(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次回滚所有迁移（逆序）
        m0020_fix_schema_model_sync::Migration.down(manager).await?;
        m0019_add_missing_columns::Migration.down(manager).await?;
        m0018_add_finance_tables::Migration.down(manager).await?;
        m0017_add_crm_supplier_tables::Migration
            .down(manager)
            .await?;
        m0016_add_version_to_inventory_stocks::Migration
            .down(manager)
            .await?;
        m0015_add_opportunity_id_to_sales_orders::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}
