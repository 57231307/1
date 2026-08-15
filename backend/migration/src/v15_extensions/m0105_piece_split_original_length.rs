use sea_orm_migration::prelude::*;

/// V15 P2 缺陷 3.2：拆匹数量之和强校验
/// 添加 original_length 和 original_weight 字段到 inventory_piece 表，
/// 用于记录拆分前的原始长度/重量，支持"remaining + children = original"校验。
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 添加 original_length 字段（拆分前原始长度，NULL 表示未拆分过的原始匹）
        manager
            .alter_table(
                Table::alter()
                    .table(InventoryPiece::Table)
                    .add_column(
                        ColumnDef::new(InventoryPiece::OriginalLength)
                            .decimal_len(12, 2)
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 添加 original_weight 字段（拆分前原始重量，NULL 表示未拆分过的原始匹）
        manager
            .alter_table(
                Table::alter()
                    .table(InventoryPiece::Table)
                    .add_column(
                        ColumnDef::new(InventoryPiece::OriginalWeight)
                            .decimal_len(12, 4)
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
                    .table(InventoryPiece::Table)
                    .drop_column(InventoryPiece::OriginalLength)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(InventoryPiece::Table)
                    .drop_column(InventoryPiece::OriginalWeight)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum InventoryPiece {
    Table,
    OriginalLength,
    OriginalWeight,
}
