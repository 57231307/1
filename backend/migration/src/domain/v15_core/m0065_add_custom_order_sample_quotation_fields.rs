use sea_orm_migration::prelude::*;

// Batch 483 P0-B11：定制订单流程补齐打样和报价环节
//
// 业务背景：
//   V15 审计报告 batch-19 §23.2 缺陷 1 — 定制订单流程缺失"打样"和"报价"环节
//
//   面料行业定制业务的核心是"先打样确认→再报价→再大货生产"。
//   当前定制订单直接从 draft 进入 yarn_purchasing（纱线采购），
//   完全跳过了打样（lab_dip_request）和报价（sales_quotation）环节。
//
//   打样通知单（lab_dip_request）作为独立模块存在，含客户确认字段
//   （customer_approved_at / approved_sample_id），但与 custom_orders 表无关联。
//   报价单（sales_quotation）同样与 custom_orders 脱钩。
//
// 修复方案：
//   1. custom_orders 表新增 lab_dip_request_id 字段关联打样通知单
//   2. custom_orders 表新增 quotation_id 字段关联报价单
//   3. 后续在状态机（process_state_machine.rs）中插入 LabDip / Quotation 状态
//      并增加状态门：未通过打样确认禁止推进到 yarn_purchasing
//
// 设计依据：V15 审计报告 P0-B11（batch-19 §23.2 缺陷 1）
// 关联文件：
//   - models/custom_order.rs（新增 2 字段 + 2 Relations）
//   - utils/process_state_machine.rs（新增 LabDip / Quotation 状态）
//   - services/custom_order_state_service.rs（状态门校验）

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
                -- P0-B11：custom_orders 表新增打样通知单 + 报价单关联字段
                -- ============================================================

                -- 关联打样通知单 ID（指向 lab_dip_request.id）
                -- 业务语义：定制订单 draft → lab_dip 状态时关联打样通知单，
                -- 客户在打样通知单上确认 OK 样（approved_sample_id）后，
                -- 定制订单才能推进到报价阶段。
                ALTER TABLE "custom_orders"
                    ADD COLUMN IF NOT EXISTS "lab_dip_request_id" INTEGER;

                -- 关联报价单 ID（指向 sales_quotations.id）
                -- 业务语义：定制订单 lab_dip → quotation 状态时关联报价单，
                -- 报价单审批通过后，total_amount 自动同步到定制订单，
                -- 定制订单才能推进到 yarn_purchasing（纱线采购）阶段。
                ALTER TABLE "custom_orders"
                    ADD COLUMN IF NOT EXISTS "quotation_id" BIGINT;

                -- 索引：按打样通知单 / 报价单反查定制订单
                CREATE INDEX IF NOT EXISTS "idx_custom_orders_lab_dip_request_id"
                    ON "custom_orders"("lab_dip_request_id");
                CREATE INDEX IF NOT EXISTS "idx_custom_orders_quotation_id"
                    ON "custom_orders"("quotation_id");

                COMMENT ON COLUMN "custom_orders"."lab_dip_request_id" IS '关联打样通知单 ID（P0-B11：定制订单打样环节锚点，指向 lab_dip_request.id；客户确认 OK 样后此 ID 的 approved_sample_id 非空才允许推进到报价阶段）';
                COMMENT ON COLUMN "custom_orders"."quotation_id" IS '关联报价单 ID（P0-B11：定制订单报价环节锚点，指向 sales_quotations.id；报价单审批通过后 total_amount 自动同步到本订单）';
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
                -- 回滚 custom_orders 新增字段
                DROP INDEX IF EXISTS "idx_custom_orders_quotation_id";
                DROP INDEX IF EXISTS "idx_custom_orders_lab_dip_request_id";
                ALTER TABLE "custom_orders"
                    DROP COLUMN IF EXISTS "quotation_id";
                ALTER TABLE "custom_orders"
                    DROP COLUMN IF EXISTS "lab_dip_request_id";
                "#,
            )
            .await?;
        Ok(())
    }
}
