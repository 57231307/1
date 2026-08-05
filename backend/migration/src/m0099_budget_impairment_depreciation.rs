//! P2-Phase-6-Batch-B：预算版本管理 + 资产减值测试 + 折旧政策变更
//!
//! - 新建 budget_versions 表（预算版本管理）
//! - 新建 asset_impairment_tests 表（资产减值测试）
//! - 新建 depreciation_policy_changes 表（折旧政策变更）

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. 创建 budget_versions 表
        manager
            .create_table(
                Table::create()
                    .table(BudgetVersions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(BudgetVersions::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(BudgetVersions::PlanId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BudgetVersions::VersionNo)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BudgetVersions::VersionName)
                            .string_len(200)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BudgetVersions::TotalAmount)
                            .decimal_len(14, 2)
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(BudgetVersions::Status)
                            .string_len(20)
                            .not_null()
                            .default("draft"),
                    )
                    .col(
                        ColumnDef::new(BudgetVersions::ChangeReason)
                            .string_len(500)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(BudgetVersions::ApprovedBy)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(BudgetVersions::ApprovedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(BudgetVersions::CreatedBy)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BudgetVersions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .col(
                        ColumnDef::new(BudgetVersions::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .to_owned(),
            )
            .await?;

        // 2. 创建 asset_impairment_tests 表
        manager
            .create_table(
                Table::create()
                    .table(AssetImpairmentTests::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AssetImpairmentTests::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AssetImpairmentTests::AssetId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AssetImpairmentTests::TestDate)
                            .date()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AssetImpairmentTests::CarryingAmount)
                            .decimal_len(14, 2)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AssetImpairmentTests::RecoverableAmount)
                            .decimal_len(14, 2)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AssetImpairmentTests::ImpairmentLoss)
                            .decimal_len(14, 2)
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(AssetImpairmentTests::TestBasis)
                            .string_len(200)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AssetImpairmentTests::Notes)
                            .string_len(500)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AssetImpairmentTests::Status)
                            .string_len(20)
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(AssetImpairmentTests::ReviewedBy)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AssetImpairmentTests::ReviewedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AssetImpairmentTests::CreatedBy)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AssetImpairmentTests::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .col(
                        ColumnDef::new(AssetImpairmentTests::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .to_owned(),
            )
            .await?;

        // 3. 创建 depreciation_policy_changes 表
        manager
            .create_table(
                Table::create()
                    .table(DepreciationPolicyChanges::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DepreciationPolicyChanges::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(DepreciationPolicyChanges::AssetId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DepreciationPolicyChanges::ChangeDate)
                            .date()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DepreciationPolicyChanges::OldMethod)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DepreciationPolicyChanges::NewMethod)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DepreciationPolicyChanges::OldUsefulLife)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DepreciationPolicyChanges::NewUsefulLife)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DepreciationPolicyChanges::OldSalvageRate)
                            .decimal_len(5, 4)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DepreciationPolicyChanges::NewSalvageRate)
                            .decimal_len(5, 4)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DepreciationPolicyChanges::Reason)
                            .string_len(500)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DepreciationPolicyChanges::ApprovedBy)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DepreciationPolicyChanges::ApprovedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DepreciationPolicyChanges::Status)
                            .string_len(20)
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(DepreciationPolicyChanges::CreatedBy)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DepreciationPolicyChanges::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .col(
                        ColumnDef::new(DepreciationPolicyChanges::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DepreciationPolicyChanges::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(AssetImpairmentTests::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(BudgetVersions::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum BudgetVersions {
    Table,
    Id,
    PlanId,
    VersionNo,
    VersionName,
    TotalAmount,
    Status,
    ChangeReason,
    ApprovedBy,
    ApprovedAt,
    CreatedBy,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum AssetImpairmentTests {
    Table,
    Id,
    AssetId,
    TestDate,
    CarryingAmount,
    RecoverableAmount,
    ImpairmentLoss,
    TestBasis,
    Notes,
    Status,
    ReviewedBy,
    ReviewedAt,
    CreatedBy,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum DepreciationPolicyChanges {
    Table,
    Id,
    AssetId,
    ChangeDate,
    OldMethod,
    NewMethod,
    OldUsefulLife,
    NewUsefulLife,
    OldSalvageRate,
    NewSalvageRate,
    Reason,
    ApprovedBy,
    ApprovedAt,
    Status,
    CreatedBy,
    CreatedAt,
    UpdatedAt,
}
