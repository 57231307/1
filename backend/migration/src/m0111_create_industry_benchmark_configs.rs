use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 创建 industry_benchmark_configs 表
        manager
            .create_table(
                Table::create()
                    .table(IndustryBenchmarkConfigs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(IndustryBenchmarkConfigs::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(IndustryBenchmarkConfigs::Industry)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(IndustryBenchmarkConfigs::MetricName)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(IndustryBenchmarkConfigs::BenchmarkValue)
                            .decimal_len(15, 4)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(IndustryBenchmarkConfigs::Unit)
                            .string_len(50)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(IndustryBenchmarkConfigs::Description)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(IndustryBenchmarkConfigs::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(IndustryBenchmarkConfigs::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .col(
                        ColumnDef::new(IndustryBenchmarkConfigs::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .to_owned(),
            )
            .await?;

        // 创建索引
        manager
            .create_index(
                Index::create()
                    .name("idx_industry_benchmark_configs_industry")
                    .table(IndustryBenchmarkConfigs::Table)
                    .col(IndustryBenchmarkConfigs::Industry)
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
                    .name("idx_industry_benchmark_configs_industry")
                    .table(IndustryBenchmarkConfigs::Table)
                    .to_owned(),
            )
            .await?;

        // 删除表
        manager
            .drop_table(
                Table::drop()
                    .table(IndustryBenchmarkConfigs::Table)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum IndustryBenchmarkConfigs {
    Table,
    Id,
    Industry,
    MetricName,
    BenchmarkValue,
    Unit,
    Description,
    IsActive,
    CreatedAt,
    UpdatedAt,
}
