use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 创建 sales_contract_items 表
        manager
            .create_table(
                Table::create()
                    .table(SalesContractItems::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SalesContractItems::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(SalesContractItems::ContractId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SalesContractItems::ProductId)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(SalesContractItems::ProductName)
                            .string_len(200)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SalesContractItems::ProductSpec)
                            .string_len(500)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(SalesContractItems::Unit)
                            .string_len(20)
                            .not_null()
                            .default("米"),
                    )
                    .col(
                        ColumnDef::new(SalesContractItems::Quantity)
                            .decimal_len(15, 2)
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(SalesContractItems::UnitPrice)
                            .decimal_len(15, 4)
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(SalesContractItems::Amount)
                            .decimal_len(15, 2)
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(SalesContractItems::DeliveryDate)
                            .date()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(SalesContractItems::Remarks)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(SalesContractItems::SortOrder)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(SalesContractItems::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .col(
                        ColumnDef::new(SalesContractItems::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_sales_contract_items_contract_id")
                            .from(SalesContractItems::Table, SalesContractItems::ContractId)
                            .to(SalesContracts::Table, SalesContracts::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 添加索引
        manager
            .create_index(
                Index::create()
                    .name("idx_sales_contract_items_contract_id")
                    .table(SalesContractItems::Table)
                    .col(SalesContractItems::ContractId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_sales_contract_items_product_id")
                    .table(SalesContractItems::Table)
                    .col(SalesContractItems::ProductId)
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
                    .name("idx_sales_contract_items_product_id")
                    .table(SalesContractItems::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_sales_contract_items_contract_id")
                    .table(SalesContractItems::Table)
                    .to_owned(),
            )
            .await?;

        // 删除表
        manager
            .drop_table(Table::drop().table(SalesContractItems::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum SalesContractItems {
    Table,
    Id,
    ContractId,
    ProductId,
    ProductName,
    ProductSpec,
    Unit,
    Quantity,
    UnitPrice,
    Amount,
    DeliveryDate,
    Remarks,
    SortOrder,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum SalesContracts {
    Table,
    Id,
}
