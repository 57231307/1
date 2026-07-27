use sea_orm_migration::prelude::*;

// V15 P1 batch-18 缺陷修复（类二十一胚布拆匹 + 类二十二库存排程）：
//
// 1.1: greige_fabric 新增 purchase_order_id/purchase_receipt_id 关联采购流程
// 1.2: greige_fabric 新增 safety_stock/reorder_point/max_stock_point/reorder_quantity 安全库存预警字段
// 2.1: outsourcing_order_item 新增 greige_fabric_id 关联胚布发料追溯
// 4.2: quality_issues 新增 root_cause_method/root_cause_detail 根因分析方法
// 4.3: quality_issues 新增 permanent_action_owner/permanent_action_due_date/permanent_action_completed_at 跟踪
// 5.1: unqualified_products 新增 stock_grade_synced 标记降级同步状态
// 5.3: unqualified_products 新增 scrap_approval_status/approver_id_fin/approver_id_gm/approved_at_fin/approved_at_gm 报废二级审批
// 6.1: inventory_transfer 新增 approval_level/approved_by_role 分级审批
// 7.1: inventory_stocks 新增 replenishment_strategy 补货策略
// 9.1: production_orders 新增 schedule_batch_key 缸号批量排程分组键
// 10.1: work_centers 新增 standard_hours_per_unit/equipment_count/worker_count/shift_hours 产能模型
// 11.1: 新建 work_center_equipment/work_center_worker/work_center_shift 关联实体表
// 11.3: work_centers 新增 auto_reschedule_enabled 调度异常自动重排开关
//
// 关联文件：
//   - models/greige_fabric.rs（新增采购订单 + 安全库存字段）
//   - models/outsourcing_order_item.rs（新增 greige_fabric_id）
//   - models/quality_issue.rs（新增 8D 根因分析字段）
//   - models/inventory_transfer.rs（新增分级审批字段）
//   - models/inventory_stock.rs（新增补货策略字段）
//   - models/work_center.rs（新增产能模型字段）
//   - models/work_center_equipment.rs（新建设备关联表）
//   - models/work_center_worker.rs（新建人员关联表）
//   - models/work_center_shift.rs（新建班次关联表）

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
                -- P1 batch-18 缺陷 1.1：胚布关联采购订单
                -- ============================================================
                ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "purchase_order_id" INTEGER;
                ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "purchase_receipt_id" INTEGER;
                COMMENT ON COLUMN "greige_fabric"."purchase_order_id" IS '关联采购订单ID（NULL表示手工录入未走采购流程）';
                COMMENT ON COLUMN "greige_fabric"."purchase_receipt_id" IS '关联采购入库单ID（支持三单匹配）';

                -- ============================================================
                -- P1 batch-18 缺陷 1.2：胚布安全库存预警字段
                -- ============================================================
                ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "safety_stock" DECIMAL(12,2) DEFAULT 0;
                ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "reorder_point" DECIMAL(12,2) DEFAULT 0;
                ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "max_stock_point" DECIMAL(12,2) DEFAULT 0;
                ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "reorder_quantity" DECIMAL(12,2) DEFAULT 0;
                COMMENT ON COLUMN "greige_fabric"."safety_stock" IS '安全库存（公斤）';
                COMMENT ON COLUMN "greige_fabric"."reorder_point" IS '订货点（公斤，低于此值触发补货建议）';
                COMMENT ON COLUMN "greige_fabric"."max_stock_point" IS '最大库存（公斤，超过此值告警）';
                COMMENT ON COLUMN "greige_fabric"."reorder_quantity" IS '补货量（公斤）';

                -- ============================================================
                -- P1 batch-18 缺陷 2.1：委外发料关联胚布
                -- ============================================================
                ALTER TABLE "outsourcing_order_item" ADD COLUMN IF NOT EXISTS "greige_fabric_id" INTEGER;
                COMMENT ON COLUMN "outsourcing_order_item"."greige_fabric_id" IS '关联胚布ID（精确到卷/匹级追溯）';

                -- ============================================================
                -- P1 batch-18 缺陷 4.2：8D 根因分析方法
                -- ============================================================
                ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "root_cause_method" VARCHAR(20);
                ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "root_cause_detail" JSONB;
                COMMENT ON COLUMN "quality_issues"."root_cause_method" IS '根因分析方法：5why/fishbone/other';
                COMMENT ON COLUMN "quality_issues"."root_cause_detail" IS '根因分析过程结构化存储（5why层级/鱼骨图分支）';

                -- ============================================================
                -- P1 batch-18 缺陷 4.3：纠正预防措施跟踪
                -- ============================================================
                ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "permanent_action_owner" INTEGER;
                ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "permanent_action_due_date" DATE;
                ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "permanent_action_completed_at" TIMESTAMP;
                COMMENT ON COLUMN "quality_issues"."permanent_action_owner" IS '永久措施责任人user_id';
                COMMENT ON COLUMN "quality_issues"."permanent_action_due_date" IS '永久措施完成截止日期';
                COMMENT ON COLUMN "quality_issues"."permanent_action_completed_at" IS '永久措施实际完成时间';

                -- ============================================================
                -- P1 batch-18 缺陷 5.1：降级联动库存等级同步标记
                -- ============================================================
                ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "stock_grade_synced" BOOLEAN NOT NULL DEFAULT FALSE;
                ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "stock_id" INTEGER;
                COMMENT ON COLUMN "unqualified_products"."stock_grade_synced" IS '是否已同步库存等级（B级降级时联动 inventory_stocks.grade）';
                COMMENT ON COLUMN "unqualified_products"."stock_id" IS '关联库存记录ID（用于降级时定位库存）';

                -- ============================================================
                -- P1 batch-18 缺陷 5.3：报废二级审批（财务+总经理）
                -- ============================================================
                ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "scrap_approval_status" VARCHAR(20) NOT NULL DEFAULT 'not_required';
                ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "approver_id_fin" INTEGER;
                ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "approver_id_gm" INTEGER;
                ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "approved_at_fin" TIMESTAMP;
                ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "approved_at_gm" TIMESTAMP;
                ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "scrap_loss_amount" DECIMAL(12,2);
                COMMENT ON COLUMN "unqualified_products"."scrap_approval_status" IS '报废审批状态：not_required/pending_fin/pending_gm/approved/rejected';
                COMMENT ON COLUMN "unqualified_products"."approver_id_fin" IS '财务审批人user_id';
                COMMENT ON COLUMN "unqualified_products"."approver_id_gm" IS '总经理审批人user_id';
                COMMENT ON COLUMN "unqualified_products"."approved_at_fin" IS '财务审批通过时间';
                COMMENT ON COLUMN "unqualified_products"."approved_at_gm" IS '总经理审批通过时间';
                COMMENT ON COLUMN "unqualified_products"."scrap_loss_amount" IS '报废损失金额（审批通过后写入成本）';

                -- ============================================================
                -- P1 batch-18 缺陷 6.1：调拨分级审批
                -- ============================================================
                ALTER TABLE "inventory_transfer" ADD COLUMN IF NOT EXISTS "approval_level" VARCHAR(20);
                ALTER TABLE "inventory_transfer" ADD COLUMN IF NOT EXISTS "approved_by_role" VARCHAR(50);
                ALTER TABLE "inventory_transfer" ADD COLUMN IF NOT EXISTS "total_amount" DECIMAL(14,2) DEFAULT 0;
                COMMENT ON COLUMN "inventory_transfer"."approval_level" IS '审批层级：L1=常规/L2=经理/L3=总监';
                COMMENT ON COLUMN "inventory_transfer"."approved_by_role" IS '审批人角色记录';
                COMMENT ON COLUMN "inventory_transfer"."total_amount" IS '调拨总金额（数量×unit_cost 累计）用于分级审批';

                -- ============================================================
                -- P1 batch-18 缺陷 7.1：补货策略字段
                -- ============================================================
                ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "replenishment_strategy" VARCHAR(20) NOT NULL DEFAULT 'reorder_point';
                COMMENT ON COLUMN "inventory_stocks"."replenishment_strategy" IS '补货策略：reorder_point/eoq/mrp';

                -- ============================================================
                -- P1 batch-18 缺陷 9.1：排程基于缸号批量约束
                -- ============================================================
                ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "schedule_batch_key" VARCHAR(100);
                COMMENT ON COLUMN "production_orders"."schedule_batch_key" IS '排程批量键（按 dye_lot_no 聚合，同缸号订单合并为一个排程单元）';

                -- ============================================================
                -- P1 batch-18 缺陷 10.1：产能模型字段
                -- ============================================================
                ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "standard_hours_per_unit" DECIMAL(10,2);
                ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "equipment_count" INTEGER DEFAULT 1;
                ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "worker_count" INTEGER DEFAULT 1;
                ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "shift_hours" DECIMAL(6,2) DEFAULT 8;
                COMMENT ON COLUMN "work_centers"."standard_hours_per_unit" IS '标准工时（小时/单位）';
                COMMENT ON COLUMN "work_centers"."equipment_count" IS '设备数';
                COMMENT ON COLUMN "work_centers"."worker_count" IS '人员数';
                COMMENT ON COLUMN "work_centers"."shift_hours" IS '班次工时（小时）';

                -- ============================================================
                -- P1 batch-18 缺陷 11.3：调度异常自动重排开关
                -- ============================================================
                ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "auto_reschedule_enabled" BOOLEAN NOT NULL DEFAULT TRUE;
                COMMENT ON COLUMN "work_centers"."auto_reschedule_enabled" IS '工作中心状态异常时是否自动重排受影响订单';

                -- ============================================================
                -- P1 batch-18 缺陷 11.1：工作中心关联实体表
                -- ============================================================

                -- 工作中心-设备关联表
                CREATE TABLE IF NOT EXISTS "work_center_equipment" (
                    "id" SERIAL PRIMARY KEY,
                    "work_center_id" INTEGER NOT NULL,
                    "equipment_name" VARCHAR(100) NOT NULL,
                    "equipment_code" VARCHAR(50),
                    "status" VARCHAR(20) NOT NULL DEFAULT 'active',
                    "capacity_per_hour" DECIMAL(10,2),
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_work_center_equipment_wc" ON "work_center_equipment"("work_center_id");

                -- 工作中心-人员关联表（含多技能）
                CREATE TABLE IF NOT EXISTS "work_center_worker" (
                    "id" SERIAL PRIMARY KEY,
                    "work_center_id" INTEGER NOT NULL,
                    "user_id" INTEGER NOT NULL,
                    "skills" JSONB,
                    "is_primary" BOOLEAN NOT NULL DEFAULT FALSE,
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_work_center_worker_wc" ON "work_center_worker"("work_center_id");
                CREATE INDEX IF NOT EXISTS "idx_work_center_worker_user" ON "work_center_worker"("user_id");

                -- 工作中心-班次关联表
                CREATE TABLE IF NOT EXISTS "work_center_shift" (
                    "id" SERIAL PRIMARY KEY,
                    "work_center_id" INTEGER NOT NULL,
                    "shift_name" VARCHAR(50) NOT NULL,
                    "start_time" TIME NOT NULL,
                    "end_time" TIME NOT NULL,
                    "is_active" BOOLEAN NOT NULL DEFAULT TRUE,
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_work_center_shift_wc" ON "work_center_shift"("work_center_id");

                -- ============================================================
                -- P1 batch-18 缺陷 3.3：piece_mapping 表删除（改用 inventory_piece.parent_piece_id）
                -- ============================================================
                DROP TABLE IF EXISTS "piece_mapping";
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
                -- 还原 piece_mapping 表（仅结构，数据不可恢复）
                CREATE TABLE IF NOT EXISTS "piece_mapping" (
                    "id" SERIAL PRIMARY KEY,
                    "batch_no" VARCHAR(50),
                    "product_id" INTEGER,
                    "piece_no" VARCHAR(50),
                    "length" DECIMAL(12,2),
                    "weight" DECIMAL(12,2),
                    "status" VARCHAR(20),
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );

                DROP TABLE IF EXISTS "work_center_shift";
                DROP TABLE IF EXISTS "work_center_worker";
                DROP TABLE IF EXISTS "work_center_equipment";

                ALTER TABLE "work_centers" DROP COLUMN IF EXISTS "auto_reschedule_enabled";
                ALTER TABLE "work_centers" DROP COLUMN IF EXISTS "shift_hours";
                ALTER TABLE "work_centers" DROP COLUMN IF EXISTS "worker_count";
                ALTER TABLE "work_centers" DROP COLUMN IF EXISTS "equipment_count";
                ALTER TABLE "work_centers" DROP COLUMN IF EXISTS "standard_hours_per_unit";

                ALTER TABLE "production_orders" DROP COLUMN IF EXISTS "schedule_batch_key";

                ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "replenishment_strategy";

                ALTER TABLE "inventory_transfer" DROP COLUMN IF EXISTS "total_amount";
                ALTER TABLE "inventory_transfer" DROP COLUMN IF EXISTS "approved_by_role";
                ALTER TABLE "inventory_transfer" DROP COLUMN IF EXISTS "approval_level";

                ALTER TABLE "unqualified_products" DROP COLUMN IF EXISTS "scrap_loss_amount";
                ALTER TABLE "unqualified_products" DROP COLUMN IF EXISTS "approved_at_gm";
                ALTER TABLE "unqualified_products" DROP COLUMN IF EXISTS "approved_at_fin";
                ALTER TABLE "unqualified_products" DROP COLUMN IF EXISTS "approver_id_gm";
                ALTER TABLE "unqualified_products" DROP COLUMN IF EXISTS "approver_id_fin";
                ALTER TABLE "unqualified_products" DROP COLUMN IF EXISTS "scrap_approval_status";
                ALTER TABLE "unqualified_products" DROP COLUMN IF EXISTS "stock_id";
                ALTER TABLE "unqualified_products" DROP COLUMN IF EXISTS "stock_grade_synced";

                ALTER TABLE "quality_issues" DROP COLUMN IF EXISTS "permanent_action_completed_at";
                ALTER TABLE "quality_issues" DROP COLUMN IF EXISTS "permanent_action_due_date";
                ALTER TABLE "quality_issues" DROP COLUMN IF EXISTS "permanent_action_owner";
                ALTER TABLE "quality_issues" DROP COLUMN IF EXISTS "root_cause_detail";
                ALTER TABLE "quality_issues" DROP COLUMN IF EXISTS "root_cause_method";

                ALTER TABLE "outsourcing_order_item" DROP COLUMN IF EXISTS "greige_fabric_id";

                ALTER TABLE "greige_fabric" DROP COLUMN IF EXISTS "reorder_quantity";
                ALTER TABLE "greige_fabric" DROP COLUMN IF EXISTS "max_stock_point";
                ALTER TABLE "greige_fabric" DROP COLUMN IF EXISTS "reorder_point";
                ALTER TABLE "greige_fabric" DROP COLUMN IF EXISTS "safety_stock";
                ALTER TABLE "greige_fabric" DROP COLUMN IF EXISTS "purchase_receipt_id";
                ALTER TABLE "greige_fabric" DROP COLUMN IF EXISTS "purchase_order_id";
                "#,
            )
            .await?;

        Ok(())
    }
}
