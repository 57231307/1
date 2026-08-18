use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 创建 period_report_snapshot 表
        manager
            .create_table(
                Table::create()
                    .table(PeriodReportSnapshot::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PeriodReportSnapshot::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(PeriodReportSnapshot::PeriodId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PeriodReportSnapshot::ReportType)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PeriodReportSnapshot::ReportData)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PeriodReportSnapshot::SnapshotHash)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PeriodReportSnapshot::CreatedBy)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PeriodReportSnapshot::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_period_report_snapshot_period_id")
                            .from(PeriodReportSnapshot::Table, PeriodReportSnapshot::PeriodId)
                            .to(AccountingPeriods::Table, AccountingPeriods::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 添加索引
        manager
            .create_index(
                Index::create()
                    .name("idx_period_report_snapshot_period_id")
                    .table(PeriodReportSnapshot::Table)
                    .col(PeriodReportSnapshot::PeriodId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_period_report_snapshot_report_type")
                    .table(PeriodReportSnapshot::Table)
                    .col(PeriodReportSnapshot::ReportType)
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
                    .name("idx_period_report_snapshot_report_type")
                    .table(PeriodReportSnapshot::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_period_report_snapshot_period_id")
                    .table(PeriodReportSnapshot::Table)
                    .to_owned(),
            )
            .await?;

        // 删除表
        manager
            .drop_table(Table::drop().table(PeriodReportSnapshot::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum PeriodReportSnapshot {
    Table,
    Id,
    PeriodId,
    ReportType,
    ReportData,
    SnapshotHash,
    CreatedBy,
    CreatedAt,
}

#[derive(DeriveIden)]
enum AccountingPeriods {
    Table,
    Id,
}
