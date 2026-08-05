//! V15 P2 20.7-B：API 向后兼容性 / deprecation 标注
//!
//! 为 api_endpoints 表新增 deprecation 相关字段：
//! - `deprecated_at`：标记为废弃的时间（TIMESTAMPTZ，可空）
//! - `sunset_at`：计划下线的时间（TIMESTAMPTZ，可空）
//!
//! 支持 RFC 8594 Sunset Header，在响应中返回 `Deprecation: true` 和 `Sunset: <date>`。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 添加 deprecated_at 字段
        manager
            .alter_table(
                Table::alter()
                    .table(ApiEndpoints::Table)
                    .add_column(
                        ColumnDef::new(ApiEndpoints::DeprecatedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 添加 sunset_at 字段
        manager
            .alter_table(
                Table::alter()
                    .table(ApiEndpoints::Table)
                    .add_column(
                        ColumnDef::new(ApiEndpoints::SunsetAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 添加 deprecation_note 字段（废弃原因说明）
        manager
            .alter_table(
                Table::alter()
                    .table(ApiEndpoints::Table)
                    .add_column(
                        ColumnDef::new(ApiEndpoints::DeprecationNote)
                            .string_len(500)
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 移除新增字段
        manager
            .alter_table(
                Table::alter()
                    .table(ApiEndpoints::Table)
                    .drop_column(ApiEndpoints::DeprecatedAt)
                    .drop_column(ApiEndpoints::SunsetAt)
                    .drop_column(ApiEndpoints::DeprecationNote)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum ApiEndpoints {
    Table,
    DeprecatedAt,
    SunsetAt,
    DeprecationNote,
}
