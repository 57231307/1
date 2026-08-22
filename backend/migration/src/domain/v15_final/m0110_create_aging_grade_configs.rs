use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 创建 aging_grade_configs 表
        manager
            .create_table(
                Table::create()
                    .table(AgingGradeConfigs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AgingGradeConfigs::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AgingGradeConfigs::GradeName)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AgingGradeConfigs::MinDays)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AgingGradeConfigs::MaxDays).integer().null())
                    .col(
                        ColumnDef::new(AgingGradeConfigs::ProvisionRate)
                            .decimal_len(5, 4)
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(AgingGradeConfigs::Description).text().null())
                    .col(
                        ColumnDef::new(AgingGradeConfigs::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(AgingGradeConfigs::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .col(
                        ColumnDef::new(AgingGradeConfigs::UpdatedAt)
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
        // 删除表
        manager
            .drop_table(Table::drop().table(AgingGradeConfigs::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum AgingGradeConfigs {
    Table,
    Id,
    GradeName,
    MinDays,
    MaxDays,
    ProvisionRate,
    Description,
    IsActive,
    CreatedAt,
    UpdatedAt,
}
