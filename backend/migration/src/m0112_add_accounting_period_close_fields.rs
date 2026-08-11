use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 添加 closed_by 字段到 accounting_periods 表
        manager
            .alter_table(
                Table::alter()
                    .table(AccountingPeriods::Table)
                    .add_column(
                        ColumnDef::new(AccountingPeriods::ClosedBy)
                            .integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 添加 closed_at 字段到 accounting_periods 表
        manager
            .alter_table(
                Table::alter()
                    .table(AccountingPeriods::Table)
                    .add_column(
                        ColumnDef::new(AccountingPeriods::ClosedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 添加 close_notes 字段到 accounting_periods 表
        manager
            .alter_table(
                Table::alter()
                    .table(AccountingPeriods::Table)
                    .add_column(
                        ColumnDef::new(AccountingPeriods::CloseNotes)
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
                    .table(AccountingPeriods::Table)
                    .drop_column(AccountingPeriods::ClosedBy)
                    .drop_column(AccountingPeriods::ClosedAt)
                    .drop_column(AccountingPeriods::CloseNotes)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum AccountingPeriods {
    Table,
    ClosedBy,
    ClosedAt,
    CloseNotes,
}
