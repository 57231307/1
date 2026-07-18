use sea_orm_migration::prelude::*;

// Batch 484 P0-B15：缺料预警状态持久化（修复审计报告 batch-18 §8.1 缺陷）
//
// 业务背景：
//   V15 审计报告 batch-18 §8.1 — 缺料预警状态不持久化，无法形成处理闭环
//
//   当前 material_shortage_service.rs 的 save_threshold_config / load_threshold_config /
//   update_status 三个方法均为"租户配置表已删除，配置不再持久化"的桩实现，
//   仅返回默认值或实时计算的 severity，不写 DB。
//   缺料检测每次都从生产订单 + BOM + 库存实时计算，无持久化缺料单据，
//   无法形成"识别→采购申请→采购订单→入库→解除"闭环。
//
//   业务影响：
//   - 缺料预警是"一次性查询"而非"工单流转"
//   - 无法跟踪缺料处理进度，无法统计平均处理周期
//   - 无法关联采购订单验证解除
//   - 供应链闭环核心能力缺失
//
// 修复方案：
//   1. 新建 material_shortage_alerts 表：持久化缺料预警记录
//      字段：缺料单号 / 物料 ID / 物料名称 / 物料编码 / 需求量 / 可用量 / 缺口量 /
//            缺口率 / 级别 / 状态 / 受影响订单数 / 关联采购申请 ID / 关联采购订单 ID /
//            识别时间 / 解除时间
//      状态机：identified → purchase_request → purchase_order → received → resolved
//   2. 新建 material_shortage_threshold_configs 表：持久化预警阈值配置
//      字段：safety_factor / critical_threshold / severe_threshold
//      单行配置（id=1 固定），通过 upsert 更新
//
// 设计依据：V15 审计报告 P0-B15（batch-18 §8.1 缺陷）
// 关联文件：
//   - models/material_shortage.rs（新增 2 Entity）
//   - services/material_shortage_service.rs（save/load/update_status 改为真实持久化）
//   - handlers/material_shortage_handler.rs（update_shortage_status 从 DB 读完整字段）

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
                -- P0-B15：缺料预警持久化（material_shortage_alerts + threshold_configs）
                -- ============================================================

                -- 1. 缺料预警记录表
                CREATE TABLE IF NOT EXISTS "material_shortage_alerts" (
                    "id" BIGSERIAL PRIMARY KEY,
                    -- 缺料单号：MS-YYYYMMDD-NNN（识别时自动生成，全局唯一）
                    "alert_no" VARCHAR(50) NOT NULL UNIQUE,
                    -- 物料 ID（关联 products.id，物料也是 product 的一种）
                    "material_id" INTEGER NOT NULL,
                    "material_name" VARCHAR(200) NOT NULL,
                    "material_code" VARCHAR(100),
                    -- 需求量 / 可用量 / 缺口量 / 缺口率（识别时快照）
                    "required_quantity" DECIMAL(18,4) NOT NULL,
                    "available_quantity" DECIMAL(18,4) NOT NULL,
                    "shortage_quantity" DECIMAL(18,4) NOT NULL,
                    "deficit_rate" DECIMAL(5,2) NOT NULL,
                    -- 级别：Critical / Severe / Warning / Normal
                    "level" VARCHAR(20) NOT NULL,
                    -- 状态机：identified → purchase_request → purchase_order → received → resolved
                    -- identified：已识别（初始状态）
                    -- purchase_request：已生成采购申请
                    -- purchase_order：已生成采购订单
                    -- received：已收货入库
                    -- resolved：已解除（终态）
                    "status" VARCHAR(20) NOT NULL DEFAULT 'identified',
                    -- 受影响订单数（识别时快照）
                    "affected_orders_count" INTEGER NOT NULL DEFAULT 0,
                    -- 关联采购申请 ID（状态推进到 purchase_request 时填入）
                    "purchase_request_id" BIGINT,
                    -- 关联采购订单 ID（状态推进到 purchase_order 时填入）
                    "purchase_order_id" BIGINT,
                    -- 单位
                    "unit" VARCHAR(20),
                    -- 识别时间（首次检测到缺料的时间）
                    "identified_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 解除时间（状态推进到 resolved 时填入）
                    "resolved_at" TIMESTAMPTZ,
                    -- 创建/更新时间
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );

                -- 索引：按物料 / 状态 / 级别 / 识别时间查询
                CREATE INDEX IF NOT EXISTS "idx_material_shortage_alerts_material_id"
                    ON "material_shortage_alerts"("material_id");
                CREATE INDEX IF NOT EXISTS "idx_material_shortage_alerts_status"
                    ON "material_shortage_alerts"("status");
                CREATE INDEX IF NOT EXISTS "idx_material_shortage_alerts_level"
                    ON "material_shortage_alerts"("level");
                CREATE INDEX IF NOT EXISTS "idx_material_shortage_alerts_identified_at"
                    ON "material_shortage_alerts"("identified_at");

                -- CHECK 约束：级别 + 状态 + 数量合法性
                ALTER TABLE "material_shortage_alerts"
                    ADD CONSTRAINT "chk_material_shortage_alerts_level"
                    CHECK ("level" IN ('Critical', 'Severe', 'Warning', 'Normal'));
                ALTER TABLE "material_shortage_alerts"
                    ADD CONSTRAINT "chk_material_shortage_alerts_status"
                    CHECK ("status" IN ('identified', 'purchase_request', 'purchase_order', 'received', 'resolved'));
                ALTER TABLE "material_shortage_alerts"
                    ADD CONSTRAINT "chk_material_shortage_alerts_shortage_nonneg"
                    CHECK ("shortage_quantity" >= 0);
                ALTER TABLE "material_shortage_alerts"
                    ADD CONSTRAINT "chk_material_shortage_alerts_deficit_rate"
                    CHECK ("deficit_rate" >= 0 AND "deficit_rate" <= 100);

                COMMENT ON TABLE "material_shortage_alerts" IS 'P0-B15：缺料预警记录表（持久化缺料单据，支持识别→采购申请→采购订单→入库→解除闭环）';
                COMMENT ON COLUMN "material_shortage_alerts"."alert_no" IS '缺料单号（MS-YYYYMMDD-NNN，识别时自动生成）';
                COMMENT ON COLUMN "material_shortage_alerts"."status" IS '状态机：identified→purchase_request→purchase_order→received→resolved';
                COMMENT ON COLUMN "material_shortage_alerts"."level" IS '级别：Critical（库存为0）/Severe（缺口>50%）/Warning（缺口≤50%）/Normal';
                COMMENT ON COLUMN "material_shortage_alerts"."purchase_request_id" IS '关联采购申请 ID（状态推进到 purchase_request 时填入）';
                COMMENT ON COLUMN "material_shortage_alerts"."purchase_order_id" IS '关联采购订单 ID（状态推进到 purchase_order 时填入）';

                -- 2. 缺料预警阈值配置表（单行配置，id=1 固定）
                CREATE TABLE IF NOT EXISTS "material_shortage_threshold_configs" (
                    "id" BIGINT PRIMARY KEY DEFAULT 1,
                    -- 安全库存倍率（低于安全库存 * 此倍率时触发预警）
                    "safety_factor" DECIMAL(5,2) NOT NULL DEFAULT 1.00,
                    -- 紧急阈值：缺口百分比 >= 此值为 Critical
                    "critical_threshold" DECIMAL(5,2) NOT NULL DEFAULT 100.00,
                    -- 严重阈值：缺口百分比 >= 此值为 Severe
                    "severe_threshold" DECIMAL(5,2) NOT NULL DEFAULT 50.00,
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 强制单行（id 必须为 1）
                    CONSTRAINT "chk_threshold_configs_id_fixed" CHECK ("id" = 1)
                );

                -- 初始化默认配置行（id=1）
                INSERT INTO "material_shortage_threshold_configs" ("id", "safety_factor", "critical_threshold", "severe_threshold")
                VALUES (1, 1.00, 100.00, 50.00)
                ON CONFLICT ("id") DO NOTHING;

                COMMENT ON TABLE "material_shortage_threshold_configs" IS 'P0-B15：缺料预警阈值配置表（单行配置，id=1 固定，通过 upsert 更新）';
                COMMENT ON COLUMN "material_shortage_threshold_configs"."safety_factor" IS '安全库存倍率（默认 1.00）';
                COMMENT ON COLUMN "material_shortage_threshold_configs"."critical_threshold" IS '紧急阈值（缺口百分比 >= 此值为 Critical，默认 100）';
                COMMENT ON COLUMN "material_shortage_threshold_configs"."severe_threshold" IS '严重阈值（缺口百分比 >= 此值为 Severe，默认 50）';
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
                DROP TABLE IF EXISTS "material_shortage_threshold_configs";
                DROP TABLE IF EXISTS "material_shortage_alerts";
                "#,
            )
            .await?;
        Ok(())
    }
}
