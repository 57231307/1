use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 添加 dyeing_capability 字段到 color_cards 表
        manager
            .alter_table(
                Table::alter()
                    .table(ColorCards::Table)
                    .add_column(
                        ColumnDef::new(ColorCards::DyeingCapability)
                            .string_len(50)
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 添加 printing_capability 字段到 color_cards 表
        manager
            .alter_table(
                Table::alter()
                    .table(ColorCards::Table)
                    .add_column(
                        ColumnDef::new(ColorCards::PrintingCapability)
                            .string_len(50)
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 添加 color_fastness_grade 字段到 color_cards 表
        manager
            .alter_table(
                Table::alter()
                    .table(ColorCards::Table)
                    .add_column(
                        ColumnDef::new(ColorCards::ColorFastnessGrade)
                            .string_len(20)
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
                    .table(ColorCards::Table)
                    .drop_column(ColorCards::DyeingCapability)
                    .drop_column(ColorCards::PrintingCapability)
                    .drop_column(ColorCards::ColorFastnessGrade)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum ColorCards {
    Table,
    DyeingCapability,
    PrintingCapability,
    ColorFastnessGrade,
}
