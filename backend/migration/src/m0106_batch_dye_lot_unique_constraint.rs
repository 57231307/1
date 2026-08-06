use sea_orm_migration::prelude::*;

/// V15 P0 缺陷 1.5-P0：匹号唯一约束修正
/// 将 batch_dye_lot 表的 batch_no 单字段 UNIQUE 替换为 (dye_lot_no, batch_no) 组合唯一约束。
/// 业务语义：同一缸号下匹号唯一（而非全局唯一）。
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. 删除 batch_no 单字段 UNIQUE 约束
        manager
            .drop_index(
                Index::drop()
                    .name("batch_dye_lot_batch_no_key")
                    .table(BatchDyeLot::Table)
                    .to_owned(),
            )
            .await?;

        // 2. 添加 (dye_lot_no, batch_no) 组合唯一约束
        manager
            .create_index(
                Index::create()
                    .name("idx_batch_dye_lot_dye_lot_no_batch_no")
                    .table(BatchDyeLot::Table)
                    .col(BatchDyeLot::DyeLotNo)
                    .col(BatchDyeLot::BatchNo)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // 3. 为 dye_batch 表也添加 (dye_lot_no, batch_no) 组合索引（非唯一，辅助查询）
        manager
            .create_index(
                Index::create()
                    .name("idx_dye_batch_dye_lot_no_batch_no")
                    .table(DyeBatch::Table)
                    .col(DyeBatch::DyeLotNo)
                    .col(DyeBatch::BatchNo)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚：删除组合唯一约束，恢复 batch_no 单字段 UNIQUE
        manager
            .drop_index(
                Index::drop()
                    .name("idx_batch_dye_lot_dye_lot_no_batch_no")
                    .table(BatchDyeLot::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_dye_batch_dye_lot_no_batch_no")
                    .table(DyeBatch::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("batch_dye_lot_batch_no_key")
                    .table(BatchDyeLot::Table)
                    .col(BatchDyeLot::BatchNo)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum BatchDyeLot {
    Table,
    DyeLotNo,
    BatchNo,
}

#[derive(Iden)]
enum DyeBatch {
    Table,
    DyeLotNo,
    BatchNo,
}
