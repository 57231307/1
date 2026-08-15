use sea_orm_migration::prelude::*;

/// V15 P2 20.1-C：胚布批次追溯字段补齐
/// 添加 dye_lot_no（缸号）和 color_no（色号）字段到 greige_fabric 表
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 添加 dye_lot_no 字段（缸号，用于染色批次追溯）
        manager
            .alter_table(
                Table::alter()
                    .table(GreigeFabric::Table)
                    .add_column(ColumnDef::new(GreigeFabric::DyeLotNo).string().null())
                    .to_owned(),
            )
            .await?;

        // 添加 color_no 字段（色号，用于颜色批次追溯）
        manager
            .alter_table(
                Table::alter()
                    .table(GreigeFabric::Table)
                    .add_column(ColumnDef::new(GreigeFabric::ColorNo).string().null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(GreigeFabric::Table)
                    .drop_column(GreigeFabric::DyeLotNo)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(GreigeFabric::Table)
                    .drop_column(GreigeFabric::ColorNo)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum GreigeFabric {
    Table,
    DyeLotNo,
    ColorNo,
}
