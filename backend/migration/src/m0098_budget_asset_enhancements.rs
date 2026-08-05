//! P2-Phase-5：预算科目-会计科目映射 + 资产分类管理
//!
//! - budget_items 表增加 account_subject_id 字段（预算科目-会计科目映射）
//! - 新建 asset_categories 表（资产分类管理）

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. budget_items 增加 account_subject_id 字段
        manager
            .alter_table(
                Table::alter()
                    .table(BudgetItems::Table)
                    .add_column(
                        ColumnDef::new(BudgetItems::AccountSubjectId)
                            .integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 2. 创建 asset_categories 表
        manager
            .create_table(
                Table::create()
                    .table(AssetCategories::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AssetCategories::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AssetCategories::CategoryCode)
                            .string_len(50)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(AssetCategories::CategoryName)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AssetCategories::ParentId)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AssetCategories::DefaultUsefulLife)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AssetCategories::DefaultDepreciationMethod)
                            .string_len(50)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AssetCategories::DefaultSalvageRate)
                            .decimal_len(5, 4)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AssetCategories::Description)
                            .string_len(500)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AssetCategories::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(AssetCategories::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .col(
                        ColumnDef::new(AssetCategories::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .to_owned(),
            )
            .await?;

        // 3. fixed_assets 增加 asset_category_id 字段（关联到 asset_categories）
        manager
            .alter_table(
                Table::alter()
                    .table(FixedAssets::Table)
                    .add_column(
                        ColumnDef::new(FixedAssets::AssetCategoryId)
                            .integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(FixedAssets::Table)
                    .drop_column(FixedAssets::AssetCategoryId)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(AssetCategories::Table).to_owned())
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(BudgetItems::Table)
                    .drop_column(BudgetItems::AccountSubjectId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum BudgetItems {
    Table,
    AccountSubjectId,
}

#[derive(DeriveIden)]
enum AssetCategories {
    Table,
    Id,
    CategoryCode,
    CategoryName,
    ParentId,
    DefaultUsefulLife,
    DefaultDepreciationMethod,
    DefaultSalvageRate,
    Description,
    IsActive,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum FixedAssets {
    Table,
    AssetCategoryId,
}
