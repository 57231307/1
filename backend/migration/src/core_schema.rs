//! 核心表结构：角色/部门/用户/权限
//!
//! 合并自: 6 个迁移文件

use sea_orm_migration::prelude::*;

// 导入所有迁移模块
mod m0001_initial_schema;
mod m0002_add_crm_and_greige_tables;
mod m0003_add_dye_tables;
mod m0004_add_field_permissions;
mod m0005_add_basic_data_and_system_tables;
mod m0006_add_general_ledger_and_finance_base;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次执行所有迁移
        m0001_initial_schema::Migration.up(manager).await?;
        m0002_add_crm_and_greige_tables::Migration
            .up(manager)
            .await?;
        m0003_add_dye_tables::Migration.up(manager).await?;
        m0004_add_field_permissions::Migration.up(manager).await?;
        m0005_add_basic_data_and_system_tables::Migration
            .up(manager)
            .await?;
        m0006_add_general_ledger_and_finance_base::Migration
            .up(manager)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 依次回滚所有迁移（逆序）
        m0006_add_general_ledger_and_finance_base::Migration
            .down(manager)
            .await?;
        m0005_add_basic_data_and_system_tables::Migration
            .down(manager)
            .await?;
        m0004_add_field_permissions::Migration.down(manager).await?;
        m0003_add_dye_tables::Migration.down(manager).await?;
        m0002_add_crm_and_greige_tables::Migration
            .down(manager)
            .await?;
        m0001_initial_schema::Migration.down(manager).await?;
        Ok(())
    }
}
