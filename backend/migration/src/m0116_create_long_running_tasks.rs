use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 创建 long_running_tasks 表
        manager
            .create_table(
                Table::create()
                    .table(LongRunningTasks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LongRunningTasks::Id)
                            .string_len(36)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(LongRunningTasks::TaskType)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LongRunningTasks::Status)
                            .string_len(20)
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(LongRunningTasks::Progress)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(LongRunningTasks::TotalSteps)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(LongRunningTasks::CurrentStep)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(LongRunningTasks::StepDescription)
                            .string_len(500)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LongRunningTasks::Result)
                            .json_binary()
                            .null(),
                    )
                    .col(ColumnDef::new(LongRunningTasks::ErrorMessage).text().null())
                    .col(ColumnDef::new(LongRunningTasks::StartedBy).integer().null())
                    .col(
                        ColumnDef::new(LongRunningTasks::StartedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LongRunningTasks::CompletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LongRunningTasks::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .col(
                        ColumnDef::new(LongRunningTasks::UpdatedAt)
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
                    .name("idx_long_running_tasks_status")
                    .table(LongRunningTasks::Table)
                    .col(LongRunningTasks::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_long_running_tasks_task_type")
                    .table(LongRunningTasks::Table)
                    .col(LongRunningTasks::TaskType)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_long_running_tasks_started_by")
                    .table(LongRunningTasks::Table)
                    .col(LongRunningTasks::StartedBy)
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
                    .name("idx_long_running_tasks_started_by")
                    .table(LongRunningTasks::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_long_running_tasks_task_type")
                    .table(LongRunningTasks::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_long_running_tasks_status")
                    .table(LongRunningTasks::Table)
                    .to_owned(),
            )
            .await?;

        // 删除表
        manager
            .drop_table(Table::drop().table(LongRunningTasks::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum LongRunningTasks {
    Table,
    Id,
    TaskType,
    Status,
    Progress,
    TotalSteps,
    CurrentStep,
    StepDescription,
    Result,
    ErrorMessage,
    StartedBy,
    StartedAt,
    CompletedAt,
    CreatedAt,
    UpdatedAt,
}
