use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 创建 aging_alert_rules 表
        manager
            .create_table(
                Table::create()
                    .table(AgingAlertRules::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AgingAlertRules::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AgingAlertRules::RuleName)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AgingAlertRules::RuleCode)
                            .string_len(50)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(AgingAlertRules::AgingBucket)
                            .string_len(20)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AgingAlertRules::ThresholdDays)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(AgingAlertRules::ThresholdAmount)
                            .decimal_len(15, 2)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AgingAlertRules::AlertLevel)
                            .string_len(20)
                            .not_null()
                            .default("warning"),
                    )
                    .col(
                        ColumnDef::new(AgingAlertRules::NotifyMethod)
                            .string_len(50)
                            .not_null()
                            .default("system"),
                    )
                    .col(
                        ColumnDef::new(AgingAlertRules::NotifyRoles)
                            .array(ColumnType::Text)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AgingAlertRules::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(AgingAlertRules::Remarks)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AgingAlertRules::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .col(
                        ColumnDef::new(AgingAlertRules::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .to_owned(),
            )
            .await?;

        // 添加索引
        manager
            .create_index(
                Index::create()
                    .name("idx_aging_alert_rules_aging_bucket")
                    .table(AgingAlertRules::Table)
                    .col(AgingAlertRules::AgingBucket)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_aging_alert_rules_is_active")
                    .table(AgingAlertRules::Table)
                    .col(AgingAlertRules::IsActive)
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
                    .name("idx_aging_alert_rules_is_active")
                    .table(AgingAlertRules::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_aging_alert_rules_aging_bucket")
                    .table(AgingAlertRules::Table)
                    .to_owned(),
            )
            .await?;

        // 删除表
        manager
            .drop_table(Table::drop().table(AgingAlertRules::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum AgingAlertRules {
    Table,
    Id,
    RuleName,
    RuleCode,
    AgingBucket,
    ThresholdDays,
    ThresholdAmount,
    AlertLevel,
    NotifyMethod,
    NotifyRoles,
    IsActive,
    Remarks,
    CreatedAt,
    UpdatedAt,
}
