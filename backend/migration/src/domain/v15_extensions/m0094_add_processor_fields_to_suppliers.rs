use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 添加 is_processor 字段到 suppliers 表
        manager
            .alter_table(
                Table::alter()
                    .table(Suppliers::Table)
                    .add_column(
                        ColumnDef::new(Suppliers::IsProcessor)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        // 添加 processor_type 字段到 suppliers 表
        manager
            .alter_table(
                Table::alter()
                    .table(Suppliers::Table)
                    .add_column(
                        ColumnDef::new(Suppliers::ProcessorType)
                            .string_len(20)
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 添加索引
        manager
            .create_index(
                Index::create()
                    .name("idx_suppliers_is_processor")
                    .table(Suppliers::Table)
                    .col(Suppliers::IsProcessor)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 删除索引
        manager
            .drop_index(
                Index::drop()
                    .name("idx_suppliers_is_processor")
                    .table(Suppliers::Table)
                    .to_owned(),
            )
            .await?;

        // 删除字段
        manager
            .alter_table(
                Table::alter()
                    .table(Suppliers::Table)
                    .drop_column(Suppliers::ProcessorType)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Suppliers::Table)
                    .drop_column(Suppliers::IsProcessor)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Suppliers {
    Table,
    IsProcessor,
    ProcessorType,
}
