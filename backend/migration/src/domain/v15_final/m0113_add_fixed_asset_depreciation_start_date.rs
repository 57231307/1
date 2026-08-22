use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 添加 depreciation_start_date 字段到 fixed_assets 表
        manager
            .alter_table(
                Table::alter()
                    .table(FixedAssets::Table)
                    .add_column(
                        ColumnDef::new(FixedAssets::DepreciationStartDate)
                            .date()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 删除字段
        manager
            .alter_table(
                Table::alter()
                    .table(FixedAssets::Table)
                    .drop_column(FixedAssets::DepreciationStartDate)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum FixedAssets {
    Table,
    DepreciationStartDate,
}
