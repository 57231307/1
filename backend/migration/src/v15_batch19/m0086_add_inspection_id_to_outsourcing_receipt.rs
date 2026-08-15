use sea_orm_migration::prelude::*;

// V15 P1-21 缺陷 2.2：委外收回单关联质检记录
//
// 审计报告 batch-18 缺陷 2.2：委外收回未走质检流程
// trigger_quality_inspection 已创建质检记录，但 inspection_id 未持久化到收回单
// 本迁移新增 inspection_id 字段，建立委外收回→质检的关联链路
//
// 关联文件：
//   - models/outsourcing_receipt.rs（新增 inspection_id 字段）
//   - services/outsourcing_ops/receipt.rs（trigger_quality_inspection 回写 inspection_id）

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE "outsourcing_receipt"
                    ADD COLUMN IF NOT EXISTS "inspection_id" INTEGER;

                COMMENT ON COLUMN "outsourcing_receipt"."inspection_id" IS
                    '缺陷 2.2：关联质检记录 ID（确认收回时自动创建质检记录并回写）';

                CREATE INDEX IF NOT EXISTS "idx_outsourcing_receipt_inspection_id"
                    ON "outsourcing_receipt" ("inspection_id")
                    WHERE "inspection_id" IS NOT NULL;
                "#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP INDEX IF EXISTS "idx_outsourcing_receipt_inspection_id";
                ALTER TABLE "outsourcing_receipt" DROP COLUMN IF EXISTS "inspection_id";
                "#,
            )
            .await?;
        Ok(())
    }
}
