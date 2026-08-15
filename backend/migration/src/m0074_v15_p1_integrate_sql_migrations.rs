//! V15 P1 迁移整合：执行所有 V15 P1 批次的 SQL 迁移
//!
//! 整合方案（参考 m0044_integrate_unreferenced_migrations）：
//! 所有 SQL 均使用 IF NOT EXISTS / IF EXISTS，保证幂等可重入。
//!
//! 包含的 V15 P1 SQL 迁移：
//! - batch_trace_log 字段扩展（dye_lot_no/color_no/product_id/from_status/to_status）
//! - fabric_physical_test_record：面料检验物理指标建模
//! - dye_batch_state_machine_on_hold_failed：缸号状态机 OnHold+Failed
//! - product_compliance_fields：产品模型合规字段（执行标准/厂名/厂址/产品等级）
//! - wage_record_detail_overtime：工资明细加班工时字段（《劳动法》第 44 条）

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // 1. batch_trace_log 字段扩展
        db.execute_unprepared(r#"
            ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(50);
            ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(50);
            ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "product_id" INTEGER;
            ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "from_status" VARCHAR(50);
            ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "to_status" VARCHAR(50);
            CREATE INDEX IF NOT EXISTS "idx_batch_trace_log_dye_lot_no" ON "batch_trace_log" ("dye_lot_no");
            CREATE INDEX IF NOT EXISTS "idx_batch_trace_log_color_no" ON "batch_trace_log" ("color_no");
            CREATE INDEX IF NOT EXISTS "idx_batch_trace_log_product_id" ON "batch_trace_log" ("product_id");
            COMMENT ON COLUMN "batch_trace_log"."dye_lot_no" IS '染色批号，按缸号追溯';
            COMMENT ON COLUMN "batch_trace_log"."color_no" IS '色号，按色号追溯';
            COMMENT ON COLUMN "batch_trace_log"."product_id" IS '产品 ID，按产品追溯';
            COMMENT ON COLUMN "batch_trace_log"."from_status" IS '流转前状态';
            COMMENT ON COLUMN "batch_trace_log"."to_status" IS '流转后状态';
        "#).await?;

        // 2. 面料物理指标检测记录表
        db.execute_unprepared(r#"
            CREATE TABLE IF NOT EXISTS "fabric_physical_test_record" (
                "id" SERIAL PRIMARY KEY,
                "inspection_id" INTEGER NOT NULL REFERENCES "fabric_inspection_record"("id") ON DELETE CASCADE,
                "test_item" VARCHAR(50) NOT NULL,
                "test_value" DECIMAL(12,2) NOT NULL,
                "standard_value" DECIMAL(12,2),
                "test_result" VARCHAR(10) NOT NULL,
                "tested_by" INTEGER,
                "tested_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                "remarks" TEXT,
                "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE INDEX IF NOT EXISTS "idx_fabric_physical_test_inspection_id" ON "fabric_physical_test_record" ("inspection_id");
            CREATE INDEX IF NOT EXISTS "idx_fabric_physical_test_test_item" ON "fabric_physical_test_record" ("test_item");
            CREATE INDEX IF NOT EXISTS "idx_fabric_physical_test_test_result" ON "fabric_physical_test_record" ("test_result");
            COMMENT ON TABLE "fabric_physical_test_record" IS '面料物理指标检测记录（十项指标）';
        "#).await?;

        // 3. 缸号状态机增加 on_hold/failed 异常态
        db.execute_unprepared(r#"
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
            VALUES ('scheduled', 'on_hold', 'hold', '暂停', TRUE, '已排缸 → 暂停')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
            VALUES ('preparing', 'on_hold', 'hold', '暂停', TRUE, '备布中 → 暂停')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
            VALUES ('dyeing', 'on_hold', 'hold', '暂停', TRUE, '进缸染色 → 暂停')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
            VALUES ('washing', 'on_hold', 'hold', '暂停', TRUE, '皂洗 → 暂停')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
            VALUES ('fixing', 'on_hold', 'hold', '暂停', TRUE, '固色 → 暂停')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
            VALUES ('dehydrating', 'on_hold', 'hold', '暂停', TRUE, '脱水 → 暂停')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
            VALUES ('drying', 'on_hold', 'hold', '暂停', TRUE, '烘干 → 暂停')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
            VALUES ('on_hold', 'scheduled', 'resume', '恢复', TRUE, '暂停 → 已排缸')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
            VALUES ('on_hold', 'preparing', 'resume', '恢复', TRUE, '暂停 → 备布中')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
            VALUES ('on_hold', 'dyeing', 'resume', '恢复', TRUE, '暂停 → 进缸染色')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
            VALUES ('on_hold', 'washing', 'resume', '恢复', TRUE, '暂停 → 皂洗')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
            VALUES ('on_hold', 'fixing', 'resume', '恢复', TRUE, '暂停 → 固色')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
            VALUES ('on_hold', 'dehydrating', 'resume', '恢复', TRUE, '暂停 → 脱水')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
            VALUES ('on_hold', 'drying', 'resume', '恢复', TRUE, '暂停 → 烘干')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_operator", "description")
            VALUES ('on_hold', 'cancelled', 'cancel', '取消', FALSE, '暂停 → 取消')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
            VALUES ('pending_schedule', 'failed', 'fail', '失败', TRUE, '待排缸 → 失败（终态）')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
            VALUES ('scheduled', 'failed', 'fail', '失败', TRUE, '已排缸 → 失败（终态）')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
            VALUES ('preparing', 'failed', 'fail', '失败', TRUE, '备布中 → 失败（终态）')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
            VALUES ('dyeing', 'failed', 'fail', '失败', TRUE, '进缸染色 → 失败（终态）')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
            VALUES ('washing', 'failed', 'fail', '失败', TRUE, '皂洗 → 失败（终态）')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
            VALUES ('fixing', 'failed', 'fail', '失败', TRUE, '固色 → 失败（终态）')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
            VALUES ('dehydrating', 'failed', 'fail', '失败', TRUE, '脱水 → 失败（终态）')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
            VALUES ('drying', 'failed', 'fail', '失败', TRUE, '烘干 → 失败（终态）')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
            VALUES ('inspecting', 'failed', 'fail', '失败', TRUE, '验布 → 失败（终态）')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
            VALUES ('stored', 'failed', 'fail', '失败', TRUE, '入库 → 失败（终态）')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
            VALUES ('rework', 'failed', 'fail', '失败', TRUE, '回修中 → 失败（终态）')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
            INSERT INTO "dye_batch_state_rule" ("from_status", "to_status", "transition_code", "transition_name", "require_remarks", "description")
            VALUES ('on_hold', 'failed', 'fail', '失败', TRUE, '暂停 → 失败（终态）')
            ON CONFLICT ("from_status", "to_status", "transition_code") DO NOTHING;
        "#).await?;

        // 4. 产品模型合规字段
        db.execute_unprepared(r#"
            ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "execution_standard" VARCHAR(50);
            ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "factory_name" VARCHAR(200);
            ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "factory_address" VARCHAR(500);
            ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "product_grade" VARCHAR(10);
            COMMENT ON COLUMN "products"."execution_standard" IS '面料执行标准号（GB/T 系列）';
            COMMENT ON COLUMN "products"."factory_name" IS '生产厂名（《产品质量法》第 27 条）';
            COMMENT ON COLUMN "products"."factory_address" IS '生产厂址（《产品质量法》第 27 条）';
            COMMENT ON COLUMN "products"."product_grade" IS '产品等级（优等品/一等品/合格品）';
            CREATE INDEX IF NOT EXISTS "idx_products_execution_standard" ON "products" ("execution_standard") WHERE "execution_standard" IS NOT NULL;
            CREATE INDEX IF NOT EXISTS "idx_products_product_grade" ON "products" ("product_grade") WHERE "product_grade" IS NOT NULL;
        "#).await?;

        // 5. 工资明细加班工时字段
        db.execute_unprepared(r#"
            ALTER TABLE "wage_record_detail" ADD COLUMN IF NOT EXISTS "weekday_overtime_minutes" INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE "wage_record_detail" ADD COLUMN IF NOT EXISTS "weekend_overtime_minutes" INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE "wage_record_detail" ADD COLUMN IF NOT EXISTS "holiday_overtime_minutes" INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE "wage_record_detail" ADD COLUMN IF NOT EXISTS "overtime_pay" DECIMAL(12,2) NOT NULL DEFAULT 0.00;
            COMMENT ON COLUMN "wage_record_detail"."weekday_overtime_minutes" IS '工作日加班工时（分钟，150%）';
            COMMENT ON COLUMN "wage_record_detail"."weekend_overtime_minutes" IS '休息日加班工时（分钟，200%）';
            COMMENT ON COLUMN "wage_record_detail"."holiday_overtime_minutes" IS '法定节假日加班工时（分钟，300%）';
            COMMENT ON COLUMN "wage_record_detail"."overtime_pay" IS '加班费';
            CREATE INDEX IF NOT EXISTS "idx_wage_record_detail_worker_id" ON "wage_record_detail" ("worker_id") WHERE "is_deleted" = false;
            CREATE INDEX IF NOT EXISTS "idx_wage_record_detail_step_record_id" ON "wage_record_detail" ("step_record_id") WHERE "is_deleted" = false;
        "#).await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
