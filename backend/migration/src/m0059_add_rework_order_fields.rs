use sea_orm_migration::prelude::*;

// Batch 479 P0-F18/F21：返工/降级/报废闭环 + 返工走生产订单
//
// 业务背景：
//   - P0-F18：bulk_color_approval 的 downgrade/scrap/customer_rework 需联动库存与生产订单
//     · downgrade（降级）：将关联库存的 grade 从"一等品"降为"二等品"或"等外品"
//     · scrap（报废）：将关联库存的 stock_status 改为"报废"、quality_status 改为"不合格"
//     · customer_rework（客户要求返工）：自动创建返工生产订单，重新进缸生产
//   - P0-F21：返工必须走生产订单流程（不能直接修改原批次状态）
//     · production_orders 表新增 order_type 区分正常订单 vs rework 返工订单
//     · production_orders 表新增 original_batch_id 记录返工对应的原批次
//     · dye_batch_rework 表新增 production_order_id 反向关联生产订单（双向可追溯）
//
// 设计依据：V15 审计报告 P0-F18/F21
// 关联文件：
//   - models/production_order.rs（新增 2 字段）
//   - models/dye_batch_rework.rs（新增 1 字段）
//   - services/inventory_stock_service.rs（update_stock_grade + mark_stock_as_scrapped）
//   - services/production_order_service.rs（create_rework_order）
//   - services/bulk_color_approval_service.rs（downgrade/scrap/customer_rework 集成联动）

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- ============================================================
                -- P0-F21：production_orders 表新增返工订单字段
                -- ============================================================

                -- 订单类型：normal 正常生产订单 / rework 返工订单
                -- 默认 normal 保证历史数据兼容（所有现存订单均为正常订单）
                ALTER TABLE "production_orders"
                    ADD COLUMN IF NOT EXISTS "order_type" VARCHAR(20) NOT NULL DEFAULT 'normal';

                -- 原批次 ID（仅 rework 订单使用，记录返工对应的原 dye_batch id）
                -- normal 订单此字段为 NULL
                ALTER TABLE "production_orders"
                    ADD COLUMN IF NOT EXISTS "original_batch_id" INTEGER;

                -- 返工订单索引（按订单类型 + 原批次查询返工链路）
                CREATE INDEX IF NOT EXISTS "idx_production_orders_order_type"
                    ON "production_orders"("order_type");
                CREATE INDEX IF NOT EXISTS "idx_production_orders_original_batch_id"
                    ON "production_orders"("original_batch_id");

                -- CHECK 约束：order_type 仅允许 normal / rework
                ALTER TABLE "production_orders"
                    DROP CONSTRAINT IF EXISTS "chk_production_orders_order_type";
                ALTER TABLE "production_orders"
                    ADD CONSTRAINT "chk_production_orders_order_type"
                    CHECK ("order_type" IN ('normal', 'rework'));

                COMMENT ON COLUMN "production_orders"."order_type" IS '订单类型：normal(正常生产订单) / rework(返工订单，由客户批色 rework 或降级触发)';
                COMMENT ON COLUMN "production_orders"."original_batch_id" IS '原批次 ID（仅 rework 订单使用，关联 dye_batch.id 记录返工的原批次）';

                -- ============================================================
                -- P0-F21：dye_batch_rework 表新增反向关联字段
                -- ============================================================

                -- 关联的生产订单 ID（指向 production_orders.id）
                -- 用于双向追溯：返修单 → 生产订单 → 原批次 → 返修单
                ALTER TABLE "dye_batch_rework"
                    ADD COLUMN IF NOT EXISTS "production_order_id" INTEGER;

                CREATE INDEX IF NOT EXISTS "idx_dbr_production_order_id"
                    ON "dye_batch_rework"("production_order_id");

                COMMENT ON COLUMN "dye_batch_rework"."production_order_id" IS '关联的返工生产订单 ID（P0-F21：返工走生产订单流程的反向追溯锚点）';
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
                -- 回滚 dye_batch_rework 新增字段
                ALTER TABLE "dye_batch_rework"
                    DROP COLUMN IF EXISTS "production_order_id";
                DROP INDEX IF EXISTS "idx_dbr_production_order_id";

                -- 回滚 production_orders 新增字段
                ALTER TABLE "production_orders"
                    DROP CONSTRAINT IF EXISTS "chk_production_orders_order_type";
                DROP INDEX IF EXISTS "idx_production_orders_original_batch_id";
                DROP INDEX IF EXISTS "idx_production_orders_order_type";
                ALTER TABLE "production_orders"
                    DROP COLUMN IF EXISTS "original_batch_id";
                ALTER TABLE "production_orders"
                    DROP COLUMN IF EXISTS "order_type";
                "#,
            )
            .await?;
        Ok(())
    }
}
