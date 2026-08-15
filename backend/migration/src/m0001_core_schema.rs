use sea_orm_migration::prelude::*;

pub mod m0001_initial_schema;
pub mod m0002_add_crm_and_greige_tables;
pub mod m0003_add_dye_tables;
pub mod m0004_add_field_permissions;
pub mod m0005_add_basic_data_and_system_tables;
pub mod m0006_add_general_ledger_and_finance_base;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0001_initial_schema::Migration.up(manager).await?;
        m0002_add_crm_and_greige_tables::Migration.up(manager).await?;
        m0003_add_dye_tables::Migration.up(manager).await?;
        m0004_add_field_permissions::Migration.up(manager).await?;
        m0005_add_basic_data_and_system_tables::Migration.up(manager).await?;
        m0006_add_general_ledger_and_finance_base::Migration.up(manager).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
