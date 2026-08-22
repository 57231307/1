//! 修复与增强
//!
//! 合并自: 4 个迁移文件
//! （m0017_add_crm_supplier_tables / m0018_add_finance_tables 已删除：
//!  二者为纯重复迁移，所建 19 张表与 m0005/m0008/m0009/m0011/m0012/m0013/m0069
//!  列定义完全一致，删除后由原迁移负责建表与回滚，无功能损失。）

use sea_orm_migration::prelude::*;

// 导入所有迁移模块
mod m0015_add_opportunity_id_to_sales_orders;
mod m0016_add_version_to_inventory_stocks;
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
        m0019_add_missing_columns::Migration.up(manager).await?;
        m0020_fix_schema_model_sync::Migration.up(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次回滚所有迁移（逆序）
        m0020_fix_schema_model_sync::Migration.down(manager).await?;
        m0019_add_missing_columns::Migration.down(manager).await?;
        m0016_add_version_to_inventory_stocks::Migration
            .down(manager)
            .await?;
        m0015_add_opportunity_id_to_sales_orders::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}
