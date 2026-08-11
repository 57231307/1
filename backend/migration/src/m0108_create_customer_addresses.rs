use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 创建 customer_addresses 表
        manager
            .create_table(
                Table::create()
                    .table(CustomerAddresses::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CustomerAddresses::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::CustomerId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::AddressType)
                            .string_len(50)
                            .not_null()
                            .default("shipping"),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::ContactName)
                            .string_len(100)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::Phone)
                            .string_len(20)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::Province)
                            .string_len(50)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::City)
                            .string_len(50)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::District)
                            .string_len(50)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::Address)
                            .string_len(500)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::ZipCode)
                            .string_len(20)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::IsDefault)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_customer_addresses_customer_id")
                            .from(CustomerAddresses::Table, CustomerAddresses::CustomerId)
                            .to(Customers::Table, Customers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 创建索引
        manager
            .create_index(
                Index::create()
                    .name("idx_customer_addresses_customer_id")
                    .table(CustomerAddresses::Table)
                    .col(CustomerAddresses::CustomerId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_customer_addresses_is_default")
                    .table(CustomerAddresses::Table)
                    .col(CustomerAddresses::IsDefault)
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
                    .name("idx_customer_addresses_is_default")
                    .table(CustomerAddresses::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_customer_addresses_customer_id")
                    .table(CustomerAddresses::Table)
                    .to_owned(),
            )
            .await?;

        // 删除表
        manager
            .drop_table(Table::drop().table(CustomerAddresses::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum CustomerAddresses {
    Table,
    Id,
    CustomerId,
    AddressType,
    ContactName,
    Phone,
    Province,
    City,
    District,
    Address,
    ZipCode,
    IsDefault,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Customers {
    Table,
    Id,
}
