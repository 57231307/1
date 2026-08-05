use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 添加 category_id 字段到 suppliers 表
        manager
            .alter_table(
                Table::alter()
                    .table(Suppliers::Table)
                    .add_column(
                        ColumnDef::new(Suppliers::CategoryId)
                            .integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 添加外键约束
        manager
            .alter_table(
                Table::alter()
                    .table(Suppliers::Table)
                    .add_foreign_key(
                        ForeignKey::create()
                            .name("fk_suppliers_category_id")
                            .from(Suppliers::Table, Suppliers::CategoryId)
                            .to(SupplierCategories::Table, SupplierCategories::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .to_owned(),
                    )
                    .to_owned(),
            )
            .await?;

        // 添加索引
        manager
            .create_index(
                Index::create()
                    .name("idx_suppliers_category_id")
                    .table(Suppliers::Table)
                    .col(Suppliers::CategoryId)
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
                    .name("idx_suppliers_category_id")
                    .table(Suppliers::Table)
                    .to_owned(),
            )
            .await?;

        // 删除外键约束
        manager
            .alter_table(
                Table::alter()
                    .table(Suppliers::Table)
                    .drop_foreign_key("fk_suppliers_category_id")
                    .to_owned(),
            )
            .await?;

        // 删除字段
        manager
            .alter_table(
                Table::alter()
                    .table(Suppliers::Table)
                    .drop_column(Suppliers::CategoryId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Suppliers {
    Table,
    CategoryId,
}

#[derive(DeriveIden)]
enum SupplierCategories {
    Table,
    Id,
}
