use sea_orm_migration::prelude::*;

// Batch 483 P0-B13：物流签收电子签收单 + 签收触发应收确认
//
// 业务背景：
//   V15 审计报告 batch-19 §23.4 缺陷 4 — 物流签收无电子签收单，签收未触发应收确认
//
//   当前 logistics_waybills 表只有 IN_TRANSIT / DELIVERED 两个状态，
//   无签收人 / 签收时间 / 电子签收单 URL 等字段。
//   update_waybill_status 切到 DELIVERED 时仅写 actual_arrival = now，
//   完全没有调用 AR 确认逻辑。
//
//   业务影响：
//   - 客户签收无电子凭证，出现物流纠纷时无法举证
//   - 签收时点不触发应收确认，导致应收账款时点不准确
//     （当前按发货确认而非签收确认，违反收入确认原则）
//   - 客户拒收/部分签收场景无法处理
//
// 修复方案：
//   1. logistics_waybills 表新增 5 个签收字段：
//      - signed_by：签收人 user_id
//      - signed_at：签收时间
//      - sign_receipt_url：电子签收单 URL（必填，签收凭证）
//      - sign_photo_url：签收现场图片 URL（可选）
//      - sign_remark：签收备注（可选，如部分签收说明）
//   2. 新增 SIGNED 状态常量（区分"送达"与"签收"）
//   3. 新增 POST /logistics/:id/sign 端点：
//      接收签收人信息 + 签收单 URL，状态从 DELIVERED → SIGNED
//   4. 签收时联动 AR 应收确认（DRAFT → APPROVED）
//
// 设计依据：V15 审计报告 P0-B13（batch-19 §23.4 缺陷 4）
// 关联文件：
//   - models/logistics_waybill.rs（新增 5 字段）
//   - models/status.rs（新增 SIGNED 常量）
//   - handlers/logistics_handler.rs（新增 sign_waybill 端点）
//   - routes/inventory.rs（注册 /logistics/:id/sign 路由）

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
                -- P0-B13：logistics_waybills 表新增电子签收字段
                -- ============================================================

                -- 签收人 user_id（关联 users.id）
                -- 业务语义：记录实际签收的操作人，用于审计追溯。
                -- 由 sign_waybill 端点从 AuthContext.user_id 自动填充。
                ALTER TABLE "logistics_waybills"
                    ADD COLUMN IF NOT EXISTS "signed_by" INTEGER;

                -- 签收时间（UTC）
                -- 业务语义：客户实际签收的时间点，触发应收确认的时点。
                -- 由 sign_waybill 端点自动填充 Utc::now()。
                ALTER TABLE "logistics_waybills"
                    ADD COLUMN IF NOT EXISTS "signed_at" TIMESTAMP;

                -- 电子签收单 URL（必填，签收凭证）
                -- 业务语义：客户签字确认的电子回单图片/PDF URL，
                -- 出现物流纠纷时作为法律凭证。
                ALTER TABLE "logistics_waybills"
                    ADD COLUMN IF NOT EXISTS "sign_receipt_url" VARCHAR(500);

                -- 签收现场图片 URL（可选）
                -- 业务语义：签收现场照片（如货物外观、签收人合影），
                -- 用于辅助举证。
                ALTER TABLE "logistics_waybills"
                    ADD COLUMN IF NOT EXISTS "sign_photo_url" VARCHAR(500);

                -- 签收备注（可选）
                -- 业务语义：部分签收 / 拒收 / 异常签收等说明。
                ALTER TABLE "logistics_waybills"
                    ADD COLUMN IF NOT EXISTS "sign_remark" VARCHAR(500);

                -- 索引：按签收人 / 签收时间查询
                CREATE INDEX IF NOT EXISTS "idx_logistics_waybills_signed_by"
                    ON "logistics_waybills"("signed_by");
                CREATE INDEX IF NOT EXISTS "idx_logistics_waybills_signed_at"
                    ON "logistics_waybills"("signed_at");

                COMMENT ON COLUMN "logistics_waybills"."signed_by" IS '签收人 user_id（P0-B13：电子签收操作人，关联 users.id，由 sign_waybill 端点从 AuthContext 自动填充）';
                COMMENT ON COLUMN "logistics_waybills"."signed_at" IS '签收时间（P0-B13：客户实际签收时间点，触发应收确认的时点）';
                COMMENT ON COLUMN "logistics_waybills"."sign_receipt_url" IS '电子签收单 URL（P0-B13：客户签字确认的电子回单，物流纠纷法律凭证）';
                COMMENT ON COLUMN "logistics_waybills"."sign_photo_url" IS '签收现场图片 URL（P0-B13：签收现场照片，辅助举证）';
                COMMENT ON COLUMN "logistics_waybills"."sign_remark" IS '签收备注（P0-B13：部分签收/拒收/异常签收说明）';
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
                -- 回滚 logistics_waybills 新增字段
                DROP INDEX IF EXISTS "idx_logistics_waybills_signed_at";
                DROP INDEX IF EXISTS "idx_logistics_waybills_signed_by";
                ALTER TABLE "logistics_waybills"
                    DROP COLUMN IF EXISTS "sign_remark";
                ALTER TABLE "logistics_waybills"
                    DROP COLUMN IF EXISTS "sign_photo_url";
                ALTER TABLE "logistics_waybills"
                    DROP COLUMN IF EXISTS "sign_receipt_url";
                ALTER TABLE "logistics_waybills"
                    DROP COLUMN IF EXISTS "signed_at";
                ALTER TABLE "logistics_waybills"
                    DROP COLUMN IF EXISTS "signed_by";
                "#,
            )
            .await?;
        Ok(())
    }
}
