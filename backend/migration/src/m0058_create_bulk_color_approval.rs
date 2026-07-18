use sea_orm_migration::prelude::*;

// Batch 478 P0-F15：创建 bulk_color_approval 大货批色审批表
//
// 业务背景：面料行业大货生产后，必须由客户对色卡进行批色确认（color approval）才能发货。
// 本表是 P0-F15/F16/F17/F19 四项任务的共同基础：
//   - P0-F15：建表（本迁移）
//   - P0-F16：剪大货样（pending → sampled 状态流转 + 样布扣减）
//   - P0-F17：客户批色确认（sent_to_customer → approved/rejected 状态流转）
//   - P0-F19：ship_order 校验（发货前所有批色记录必须 approved）
//
// 8 态状态机：
//   pending → sampled → sent_to_customer → approved / rejected / rework
//                                                   ↓
//                                              downgraded / scrapped
//
// 设计依据：V15 审计报告 类十一 P0-F15/F16/F17/F19
// 关联文件：models/bulk_color_approval.rs / services/bulk_color_approval_service.rs /
//          handlers/bulk_color_approval.rs / services/so/delivery.rs::validate_bulk_color_approval

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- 大货批色审批表（V15 P0-F15 创建）
                -- 记录面料大货的批色流程：剪样 → 发送客户 → 客户确认 → 通过/拒绝/返工/降级/报废
                CREATE TABLE IF NOT EXISTS "bulk_color_approval" (
                    "id" BIGSERIAL PRIMARY KEY,
                    -- 业务关联字段
                    "sales_order_id" INTEGER NOT NULL REFERENCES "sales_orders"("id") ON DELETE RESTRICT,
                    "dye_batch_id" INTEGER NOT NULL REFERENCES "dye_batch"("id") ON DELETE RESTRICT,
                    "customer_id" BIGINT NOT NULL REFERENCES "customers"("id") ON DELETE RESTRICT,
                    "production_order_id" INTEGER REFERENCES "production_orders"("id") ON DELETE SET NULL,
                    -- 四维标识（与发货明细对齐）
                    "product_id" INTEGER,
                    "color_no" VARCHAR(50),
                    "dye_lot_no" VARCHAR(50),
                    "batch_no" VARCHAR(50),
                    -- 样布信息
                    "sample_type" VARCHAR(20) NOT NULL DEFAULT 'cut_sample',
                    "sample_piece_id" BIGINT,
                    "sample_length_m" NUMERIC(10,2),
                    -- 批色状态与时间锚点
                    "approval_status" VARCHAR(20) NOT NULL DEFAULT 'pending',
                    "approver_id" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    "approval_date" TIMESTAMPTZ,
                    "sent_to_customer_at" TIMESTAMPTZ,
                    "customer_feedback" TEXT,
                    "delta_e_value" NUMERIC(6,3),
                    -- 处理结果
                    "reject_reason" TEXT,
                    "delivery_blocking" BOOLEAN NOT NULL DEFAULT TRUE,
                    "attachment_url" VARCHAR(500),
                    "remark" TEXT,
                    -- 元数据
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 约束
                    CONSTRAINT "chk_bca_sample_type" CHECK ("sample_type" IN ('cut_sample', 'lab_sample')),
                    CONSTRAINT "chk_bca_approval_status" CHECK (
                        "approval_status" IN ('pending', 'sampled', 'sent_to_customer', 'approved', 'rejected', 'rework', 'downgraded', 'scrapped')
                    ),
                    CONSTRAINT "chk_bca_delta_e" CHECK ("delta_e_value" IS NULL OR "delta_e_value" >= 0),
                    CONSTRAINT "chk_bca_sample_length" CHECK ("sample_length_m" IS NULL OR "sample_length_m" >= 0)
                );

                -- 索引（5 个，覆盖高频查询场景）
                CREATE INDEX IF NOT EXISTS "idx_bca_sales_order_id" ON "bulk_color_approval"("sales_order_id");
                CREATE INDEX IF NOT EXISTS "idx_bca_dye_batch_id" ON "bulk_color_approval"("dye_batch_id");
                CREATE INDEX IF NOT EXISTS "idx_bca_customer_id" ON "bulk_color_approval"("customer_id");
                CREATE INDEX IF NOT EXISTS "idx_bca_approval_status" ON "bulk_color_approval"("approval_status");
                CREATE INDEX IF NOT EXISTS "idx_bca_dye_lot_no" ON "bulk_color_approval"("dye_lot_no");

                COMMENT ON TABLE "bulk_color_approval" IS '大货批色审批表 - 剪样/客户批色/状态流转全生命周期';
                COMMENT ON COLUMN "bulk_color_approval"."sample_type" IS '样布类型：cut_sample(剪大货样) / lab_sample(化验室打样)';
                COMMENT ON COLUMN "bulk_color_approval"."approval_status" IS '状态：pending(待剪样) / sampled(已剪样) / sent_to_customer(已发客户) / approved(批色通过) / rejected(批色拒绝) / rework(返工) / downgraded(降级) / scrapped(报废)';
                COMMENT ON COLUMN "bulk_color_approval"."delta_e_value" IS 'CIE D65 色差值 ΔE（≤1.2 同色通过，≤2.5 让步接收，>2.5 不合格）';
                COMMENT ON COLUMN "bulk_color_approval"."delivery_blocking" IS '交货门禁标志（true 时阻止发货，仅 approved 状态可解除）';
                COMMENT ON COLUMN "bulk_color_approval"."sent_to_customer_at" IS '发送客户时间（批色时限计算锚点，超时 7 天自动 reject）';
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
                DROP TABLE IF EXISTS "bulk_color_approval";
                "#,
            )
            .await?;
        Ok(())
    }
}
