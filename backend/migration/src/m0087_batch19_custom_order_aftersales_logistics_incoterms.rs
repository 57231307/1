use sea_orm_migration::prelude::*;

// V15 P1 batch-19：定制订单/售后/运费/Incoterms 10 项 P1 修复
//
// 1. 23.1.2 一人多部门：新建 user_departments 关联表
// 2. 23.2.2 定制订单客户签字确认：custom_orders 加 customer_approved_at/customer_approval_comment/quality_standard_id
// 3. 23.2.3 定制订单变更二级审批：custom_orders 加 approval_instance_id/approved_by/approved_at/rejection_reason
// 4. 23.3.2 售后流程闭环：after_sales 加 accepted_at/evaluation_score/evaluation_comment/evaluated_at
// 5. 23.3.3 售后原因分析月报：after_sales 加 reason_category/reason_detail
// 6. 23.4.1 运单关联采购订单：logistics_waybills 加 order_type
// 7. 23.4.2 物流跟踪历史：新建 logistics_tracking_events 表
// 8. 23.4.3 运费核算：logistics_waybills 加 total_weight/total_volume/distance_km/freight_rate/freight_bearer
// 9. 23.5.2 术语与价格构成集成：sales_quotations 加 freight_cost/insurance_cost/duty_cost
// 10. 23.5.4 术语使用月报：通过 sales_quotations 现有字段聚合（无需 DDL，service 层实现）
//
// 蓝绿部署兼容性：所有新增字段均 NULLABLE 或带 DEFAULT，符合 lib.rs 规则 1

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
                -- 缺陷 23.1.2：用户部门关联表（一人多部门，主部门+兼职）
                -- ============================================================
                CREATE TABLE IF NOT EXISTS "user_departments" (
                    "id" SERIAL PRIMARY KEY,
                    "user_id" INTEGER NOT NULL,
                    "department_id" INTEGER NOT NULL,
                    "is_primary" BOOLEAN NOT NULL DEFAULT FALSE,
                    "start_date" DATE,
                    "end_date" DATE,
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_user_departments_user_id" ON "user_departments"("user_id");
                CREATE INDEX IF NOT EXISTS "idx_user_departments_department_id" ON "user_departments"("department_id");
                CREATE UNIQUE INDEX IF NOT EXISTS "idx_user_departments_user_primary" ON "user_departments"("user_id") WHERE "is_primary" = TRUE;
                COMMENT ON TABLE "user_departments" IS 'V15 P1 batch-19 缺陷 23.1.2：用户部门关联表（一人多部门，主部门+兼职）';
                COMMENT ON COLUMN "user_departments"."user_id" IS '用户 ID（关联 users.id）';
                COMMENT ON COLUMN "user_departments"."department_id" IS '部门 ID（关联 departments.id）';
                COMMENT ON COLUMN "user_departments"."is_primary" IS '是否主部门（true=主部门，false=兼职部门，每用户仅 1 个主部门）';
                COMMENT ON COLUMN "user_departments"."start_date" IS '兼职开始日期（NULL 表示无固定期限）';
                COMMENT ON COLUMN "user_departments"."end_date" IS '兼职结束日期（NULL 表示无固定期限）';

                -- ============================================================
                -- 缺陷 23.2.2：定制订单客户签字确认字段
                -- ============================================================
                ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "customer_approved_at" TIMESTAMP;
                ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "customer_approval_comment" TEXT;
                ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "quality_standard_id" INTEGER;
                CREATE INDEX IF NOT EXISTS "idx_custom_orders_quality_standard_id" ON "custom_orders"("quality_standard_id");
                COMMENT ON COLUMN "custom_orders"."customer_approved_at" IS 'V15 P1 batch-19 缺陷 23.2.2：客户签字确认时间，未确认禁止推进到 yarn_purchasing';
                COMMENT ON COLUMN "custom_orders"."customer_approval_comment" IS 'V15 P1 batch-19 缺陷 23.2.2：客户确认备注（客户对定制订单整体签字确认时填写）';
                COMMENT ON COLUMN "custom_orders"."quality_standard_id" IS 'V15 P1 batch-19 缺陷 23.2.2：客户专属质量标准 ID（关联 quality_standards.id），质检按客户专属标准';

                -- ============================================================
                -- 缺陷 23.2.3：定制订单变更二级审批字段
                -- ============================================================
                ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "approval_instance_id" BIGINT;
                ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "approved_by" BIGINT;
                ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "approved_at" TIMESTAMP;
                ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "rejection_reason" TEXT;
                CREATE INDEX IF NOT EXISTS "idx_custom_orders_approval_instance_id" ON "custom_orders"("approval_instance_id");
                COMMENT ON COLUMN "custom_orders"."approval_instance_id" IS 'V15 P1 batch-19 缺陷 23.2.3：BPM 变更审批实例 ID（非 draft 状态变更走二级审批）';
                COMMENT ON COLUMN "custom_orders"."approved_by" IS 'V15 P1 batch-19 缺陷 23.2.3：审批人 user_id';
                COMMENT ON COLUMN "custom_orders"."approved_at" IS 'V15 P1 batch-19 缺陷 23.2.3：审批时间';
                COMMENT ON COLUMN "custom_orders"."rejection_reason" IS 'V15 P1 batch-19 缺陷 23.2.3：审批拒绝原因';

                -- ============================================================
                -- 缺陷 23.3.2：售后流程闭环（受理+评价）字段
                -- ============================================================
                ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "accepted_at" TIMESTAMP;
                ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "evaluation_score" INTEGER;
                ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "evaluation_comment" TEXT;
                ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "evaluated_at" TIMESTAMP;
                COMMENT ON COLUMN "after_sales"."accepted_at" IS 'V15 P1 batch-19 缺陷 23.3.2：受理时间（opened→accepted 时填入）';
                COMMENT ON COLUMN "after_sales"."evaluation_score" IS 'V15 P1 batch-19 缺陷 23.3.2：客户评价分数（1-5，resolved→evaluated 时填入）';
                COMMENT ON COLUMN "after_sales"."evaluation_comment" IS 'V15 P1 batch-19 缺陷 23.3.2：客户评价评语';
                COMMENT ON COLUMN "after_sales"."evaluated_at" IS 'V15 P1 batch-19 缺陷 23.3.2：客户评价时间';

                -- ============================================================
                -- 缺陷 23.3.3：售后原因分析月报字段
                -- ============================================================
                ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "reason_category" VARCHAR(30);
                ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "reason_detail" VARCHAR(100);
                CREATE INDEX IF NOT EXISTS "idx_after_sales_reason_category" ON "after_sales"("reason_category");
                COMMENT ON COLUMN "after_sales"."reason_category" IS 'V15 P1 batch-19 缺陷 23.3.3：原因分类（quality/logistics/customer_preference/other）';
                COMMENT ON COLUMN "after_sales"."reason_detail" IS 'V15 P1 batch-19 缺陷 23.3.3：原因明细（结构化子类，如"色差超差"/"缸号混铺"）';

                -- ============================================================
                -- 缺陷 23.4.1：运单关联采购订单（order_type 区分销售/采购）
                -- ============================================================
                ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "order_type" VARCHAR(20);
                COMMENT ON COLUMN "logistics_waybills"."order_type" IS 'V15 P1 batch-19 缺陷 23.4.1：订单类型（sales_order/purchase_order/transfer_order）';

                -- ============================================================
                -- 缺陷 23.4.2：物流跟踪事件历史表
                -- ============================================================
                CREATE TABLE IF NOT EXISTS "logistics_tracking_events" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "waybill_id" INTEGER NOT NULL,
                    "event_time" TIMESTAMP NOT NULL,
                    "location" VARCHAR(200),
                    "description" VARCHAR(500) NOT NULL,
                    "event_type" VARCHAR(30) NOT NULL,
                    "data_source" VARCHAR(20) NOT NULL DEFAULT 'manual',
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_logistics_tracking_events_waybill_id" ON "logistics_tracking_events"("waybill_id");
                CREATE INDEX IF NOT EXISTS "idx_logistics_tracking_events_event_time" ON "logistics_tracking_events"("event_time");
                COMMENT ON TABLE "logistics_tracking_events" IS 'V15 P1 batch-19 缺陷 23.4.2：物流跟踪事件历史（运单轨迹）';
                COMMENT ON COLUMN "logistics_tracking_events"."waybill_id" IS '运单 ID（关联 logistics_waybills.id）';
                COMMENT ON COLUMN "logistics_tracking_events"."event_time" IS '事件时间（快递公司上报时间或手工录入时间）';
                COMMENT ON COLUMN "logistics_tracking_events"."location" IS '事件发生地点（如"上海转运中心"）';
                COMMENT ON COLUMN "logistics_tracking_events"."description" IS '事件描述（如"已揽收"/"运输中"/"派送中"/"已签收"）';
                COMMENT ON COLUMN "logistics_tracking_events"."event_type" IS '事件类型：picked_up/in_transit/arrived_at_hub/out_for_delivery/delivered/exception';
                COMMENT ON COLUMN "logistics_tracking_events"."data_source" IS '数据来源：manual（手工录入）/ express_api（快递 API 同步）';

                -- ============================================================
                -- 缺陷 23.4.3：运费核算字段
                -- ============================================================
                ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "total_weight" DECIMAL(12,2);
                ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "total_volume" DECIMAL(12,3);
                ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "distance_km" DECIMAL(10,2);
                ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "freight_rate" DECIMAL(12,4);
                ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "freight_bearer" VARCHAR(10);
                COMMENT ON COLUMN "logistics_waybills"."total_weight" IS 'V15 P1 batch-19 缺陷 23.4.3：总重量（kg）';
                COMMENT ON COLUMN "logistics_waybills"."total_volume" IS 'V15 P1 batch-19 缺陷 23.4.3：总体积（m³）';
                COMMENT ON COLUMN "logistics_waybills"."distance_km" IS 'V15 P1 batch-19 缺陷 23.4.3：运输距离（km）';
                COMMENT ON COLUMN "logistics_waybills"."freight_rate" IS 'V15 P1 batch-19 缺陷 23.4.3：运费费率（按重量/体积/距离核算的基准费率）';
                COMMENT ON COLUMN "logistics_waybills"."freight_bearer" IS 'V15 P1 batch-19 缺陷 23.4.3：运费承担方（customer/company）';

                -- ============================================================
                -- 缺陷 23.5.2：术语与价格构成集成字段
                -- ============================================================
                ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "freight_cost" DECIMAL(14,2);
                ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "insurance_cost" DECIMAL(14,2);
                ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "duty_cost" DECIMAL(14,2);
                COMMENT ON COLUMN "sales_quotations"."freight_cost" IS 'V15 P1 batch-19 缺陷 23.5.2：运费成本（CIF/CFR/CPT/CIP/DAP/DPU/DDP 含运费）';
                COMMENT ON COLUMN "sales_quotations"."insurance_cost" IS 'V15 P1 batch-19 缺陷 23.5.2：保险费成本（CIF/CIP/DDP 含保险）';
                COMMENT ON COLUMN "sales_quotations"."duty_cost" IS 'V15 P1 batch-19 缺陷 23.5.2：关税成本（DDP 含关税）';
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
                -- 回滚 sales_quotations 新增字段
                ALTER TABLE "sales_quotations" DROP COLUMN IF EXISTS "duty_cost";
                ALTER TABLE "sales_quotations" DROP COLUMN IF EXISTS "insurance_cost";
                ALTER TABLE "sales_quotations" DROP COLUMN IF EXISTS "freight_cost";

                -- 回滚 logistics_waybills 运费核算字段
                ALTER TABLE "logistics_waybills" DROP COLUMN IF EXISTS "freight_bearer";
                ALTER TABLE "logistics_waybills" DROP COLUMN IF EXISTS "freight_rate";
                ALTER TABLE "logistics_waybills" DROP COLUMN IF EXISTS "distance_km";
                ALTER TABLE "logistics_waybills" DROP COLUMN IF EXISTS "total_volume";
                ALTER TABLE "logistics_waybills" DROP COLUMN IF EXISTS "total_weight";

                -- 回滚 logistics_waybills order_type 字段
                ALTER TABLE "logistics_waybills" DROP COLUMN IF EXISTS "order_type";

                -- 回滚 logistics_tracking_events 表
                DROP INDEX IF EXISTS "idx_logistics_tracking_events_event_time";
                DROP INDEX IF EXISTS "idx_logistics_tracking_events_waybill_id";
                DROP TABLE IF EXISTS "logistics_tracking_events";

                -- 回滚 after_sales 原因分析字段
                DROP INDEX IF EXISTS "idx_after_sales_reason_category";
                ALTER TABLE "after_sales" DROP COLUMN IF EXISTS "reason_detail";
                ALTER TABLE "after_sales" DROP COLUMN IF EXISTS "reason_category";

                -- 回滚 after_sales 受理+评价字段
                ALTER TABLE "after_sales" DROP COLUMN IF EXISTS "evaluated_at";
                ALTER TABLE "after_sales" DROP COLUMN IF EXISTS "evaluation_comment";
                ALTER TABLE "after_sales" DROP COLUMN IF EXISTS "evaluation_score";
                ALTER TABLE "after_sales" DROP COLUMN IF EXISTS "accepted_at";

                -- 回滚 custom_orders 二级审批字段
                DROP INDEX IF EXISTS "idx_custom_orders_approval_instance_id";
                ALTER TABLE "custom_orders" DROP COLUMN IF EXISTS "rejection_reason";
                ALTER TABLE "custom_orders" DROP COLUMN IF EXISTS "approved_at";
                ALTER TABLE "custom_orders" DROP COLUMN IF EXISTS "approved_by";
                ALTER TABLE "custom_orders" DROP COLUMN IF EXISTS "approval_instance_id";

                -- 回滚 custom_orders 客户签字确认字段
                DROP INDEX IF EXISTS "idx_custom_orders_quality_standard_id";
                ALTER TABLE "custom_orders" DROP COLUMN IF EXISTS "quality_standard_id";
                ALTER TABLE "custom_orders" DROP COLUMN IF EXISTS "customer_approval_comment";
                ALTER TABLE "custom_orders" DROP COLUMN IF EXISTS "customer_approved_at";

                -- 回滚 user_departments 表
                DROP INDEX IF EXISTS "idx_user_departments_user_primary";
                DROP INDEX IF EXISTS "idx_user_departments_department_id";
                DROP INDEX IF EXISTS "idx_user_departments_user_id";
                DROP TABLE IF EXISTS "user_departments";
                "#,
            )
            .await?;
        Ok(())
    }
}
