use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 添加 source 字段到 customers 表
        manager
            .alter_table(
                Table::alter()
                    .table(Customers::Table)
                    .add_column(
                        ColumnDef::new(Customers::Source)
                            .string_len(50)
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 添加 pool_recycle_reason 字段到 customers 表
        manager
            .alter_table(
                Table::alter()
                    .table(Customers::Table)
                    .add_column(
                        ColumnDef::new(Customers::PoolRecycleReason)
                            .text()
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
                    .table(Customers::Table)
                    .drop_column(Customers::Source)
                    .drop_column(Customers::PoolRecycleReason)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Customers {
    Table,
    Source,
    PoolRecycleReason,
}
