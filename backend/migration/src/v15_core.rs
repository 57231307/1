//! V15 核心功能
//!
//! 合并自: 15 个迁移文件

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // === m0061_create_bad_debt_provisions.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- 坏账准备计提表（V15 P0-B01 创建）
                -- 按客户+账龄桶+期间记录每期计提/转回，关联凭证
                CREATE TABLE IF NOT EXISTS "bad_debt_provisions" (
                    "id" BIGSERIAL PRIMARY KEY,
                    -- 业务关联
                    "customer_id" BIGINT NOT NULL REFERENCES "customers"("id") ON DELETE RESTRICT,
                    "customer_name" VARCHAR(200),
                    -- 期间
                    "period_year" INTEGER NOT NULL,
                    "period_month" INTEGER NOT NULL CHECK ("period_month" BETWEEN 1 AND 12),
                    -- 账龄桶（按 ar_invoice.due_date 计算）
                    "aging_bucket" VARCHAR(20) NOT NULL,
                    -- 计提基数与比例
                    "base_amount" DECIMAL(15,2) NOT NULL CHECK ("base_amount" >= 0),
                    "provision_rate" DECIMAL(5,4) NOT NULL CHECK ("provision_rate" >= 0 AND "provision_rate" <= 1),
                    "provision_amount" DECIMAL(15,2) NOT NULL CHECK ("provision_amount" >= 0),
                    -- 凭证关联
                    "voucher_id" BIGINT,
                    -- 状态：draft（草稿）/ confirmed（已确认计提）/ reversed（已转回）
                    "status" VARCHAR(20) NOT NULL DEFAULT 'draft',
                    -- 操作人
                    "created_by" INTEGER NOT NULL REFERENCES "users"("id") ON DELETE RESTRICT,
                    "confirmed_at" TIMESTAMPTZ,
                    "reversed_at" TIMESTAMPTZ,
                    "reverse_voucher_id" BIGINT,
                    "remark" TEXT,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 约束
                    CONSTRAINT "chk_bdp_aging_bucket" CHECK (
                        "aging_bucket" IN ('within_1y', '1_to_2y', '2_to_3y', 'over_3y')
                    ),
                    CONSTRAINT "chk_bdp_status" CHECK (
                        "status" IN ('draft', 'confirmed', 'reversed')
                    ),
                    CONSTRAINT "chk_bdp_period" CHECK ("period_year" >= 2000 AND "period_year" <= 2100)
                );

                -- 索引（5 个）
                CREATE INDEX IF NOT EXISTS "idx_bdp_customer_id" ON "bad_debt_provisions"("customer_id");
                CREATE INDEX IF NOT EXISTS "idx_bdp_period" ON "bad_debt_provisions"("period_year", "period_month");
                CREATE INDEX IF NOT EXISTS "idx_bdp_status" ON "bad_debt_provisions"("status");
                CREATE INDEX IF NOT EXISTS "idx_bdp_aging_bucket" ON "bad_debt_provisions"("aging_bucket");
                CREATE INDEX IF NOT EXISTS "idx_bdp_voucher_id" ON "bad_debt_provisions"("voucher_id");

                COMMENT ON TABLE "bad_debt_provisions" IS '坏账准备计提表 - 账龄法按客户+期间+账龄桶记录计提与转回';
                COMMENT ON COLUMN "bad_debt_provisions"."aging_bucket" IS '账龄桶：within_1y(1年内5%) / 1_to_2y(1-2年20%) / 2_to_3y(2-3年50%) / over_3y(3年以上100%)';
                COMMENT ON COLUMN "bad_debt_provisions"."provision_rate" IS '计提比例（0~1）：within_1y=0.05 / 1_to_2y=0.20 / 2_to_3y=0.50 / over_3y=1.00';
                COMMENT ON COLUMN "bad_debt_provisions"."status" IS '状态：draft(草稿) / confirmed(已确认计提) / reversed(已转回)';
                "#,
            )
            .await?;

        Ok(())
        // === m0062_create_bad_debt_writeoffs.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- 坏账核销审批表（V15 P0-B02 创建）
                -- 二级审批流：申请人→财务经理（level 1）→总经理（level 2）→核销执行
                CREATE TABLE IF NOT EXISTS "bad_debt_writeoffs" (
                    "id" BIGSERIAL PRIMARY KEY,
                    -- 业务关联
                    "customer_id" BIGINT NOT NULL REFERENCES "customers"("id") ON DELETE RESTRICT,
                    "ar_invoice_id" INTEGER NOT NULL REFERENCES "ar_invoices"("id") ON DELETE RESTRICT,
                    "writeoff_amount" DECIMAL(15,2) NOT NULL CHECK ("writeoff_amount" > 0),
                    "reason" TEXT NOT NULL,
                    -- 申请人
                    "applicant_user_id" INTEGER NOT NULL REFERENCES "users"("id") ON DELETE RESTRICT,
                    "applicant_username" VARCHAR(100) NOT NULL,
                    "applicant_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 审批流（二级）
                    "approval_level" SMALLINT NOT NULL DEFAULT 1 CHECK ("approval_level" BETWEEN 1 AND 2),
                    "approval_status" VARCHAR(20) NOT NULL DEFAULT 'pending',
                    -- 一级审批（财务经理）
                    "finance_manager_id" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    "finance_manager_at" TIMESTAMPTZ,
                    "finance_manager_comment" TEXT,
                    -- 二级审批（总经理）
                    "general_manager_id" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    "general_manager_at" TIMESTAMPTZ,
                    "general_manager_comment" TEXT,
                    -- 核销执行
                    "voucher_id" BIGINT,
                    "completed_at" TIMESTAMPTZ,
                    "cancelled_at" TIMESTAMPTZ,
                    "cancel_reason" TEXT,
                    "remark" TEXT,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 约束
                    CONSTRAINT "chk_bdw_status" CHECK (
                        "approval_status" IN ('pending', 'finance_approved', 'approved', 'rejected', 'cancelled')
                    )
                );

                -- 索引（5 个）
                CREATE INDEX IF NOT EXISTS "idx_bdw_customer_id" ON "bad_debt_writeoffs"("customer_id");
                CREATE INDEX IF NOT EXISTS "idx_bdw_ar_invoice_id" ON "bad_debt_writeoffs"("ar_invoice_id");
                CREATE INDEX IF NOT EXISTS "idx_bdw_approval_status" ON "bad_debt_writeoffs"("approval_status");
                CREATE INDEX IF NOT EXISTS "idx_bdw_applicant_user_id" ON "bad_debt_writeoffs"("applicant_user_id");
                CREATE INDEX IF NOT EXISTS "idx_bdw_voucher_id" ON "bad_debt_writeoffs"("voucher_id");

                COMMENT ON TABLE "bad_debt_writeoffs" IS '坏账核销审批表 - 二级审批流（申请人→财务经理→总经理）';
                COMMENT ON COLUMN "bad_debt_writeoffs"."approval_level" IS '当前审批层级：1=待财务经理审批 / 2=待总经理审批';
                COMMENT ON COLUMN "bad_debt_writeoffs"."approval_status" IS '状态：pending(待审) / finance_approved(财务经理通过,待总经理) / approved(最终通过,已核销) / rejected(拒绝) / cancelled(取消)';
                "#,
            )
            .await?;

        Ok(())
        // === m0063_create_collection_tasks.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- 催收任务表（V15 P0-B03 创建）
                -- 按账龄自动生成催收任务，分配给销售员，记录催收结果
                CREATE TABLE IF NOT EXISTS "collection_tasks" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "task_no" VARCHAR(50) NOT NULL UNIQUE,
                    -- 业务关联
                    "customer_id" BIGINT NOT NULL REFERENCES "customers"("id") ON DELETE RESTRICT,
                    "ar_invoice_id" INTEGER REFERENCES "ar_invoices"("id") ON DELETE SET NULL,
                    -- 任务内容
                    "overdue_amount" DECIMAL(15,2) NOT NULL CHECK ("overdue_amount" >= 0),
                    "overdue_days" INTEGER NOT NULL CHECK ("overdue_days" >= 0),
                    "task_type" VARCHAR(20) NOT NULL,
                    "priority" VARCHAR(20) NOT NULL DEFAULT 'normal',
                    "due_date" DATE NOT NULL,
                    -- 分配
                    "assigned_to" INTEGER NOT NULL REFERENCES "users"("id") ON DELETE RESTRICT,
                    "assigned_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "assigned_by" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    -- 执行
                    "status" VARCHAR(20) NOT NULL DEFAULT 'pending',
                    "contact_result" TEXT,
                    "contact_at" TIMESTAMPTZ,
                    "next_action_date" DATE,
                    "next_action_type" VARCHAR(20),
                    -- 扩展
                    "remark" TEXT,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 约束
                    CONSTRAINT "chk_ct_task_type" CHECK (
                        "task_type" IN ('phone', 'visit', 'email', 'letter')
                    ),
                    CONSTRAINT "chk_ct_priority" CHECK (
                        "priority" IN ('low', 'normal', 'high', 'urgent')
                    ),
                    CONSTRAINT "chk_ct_status" CHECK (
                        "status" IN ('pending', 'in_progress', 'completed', 'cancelled')
                    ),
                    CONSTRAINT "chk_ct_next_action_type" CHECK (
                        "next_action_type" IS NULL OR "next_action_type" IN ('phone', 'visit', 'email', 'letter')
                    )
                );

                -- 索引（6 个）
                CREATE INDEX IF NOT EXISTS "idx_ct_customer_id" ON "collection_tasks"("customer_id");
                CREATE INDEX IF NOT EXISTS "idx_ct_ar_invoice_id" ON "collection_tasks"("ar_invoice_id");
                CREATE INDEX IF NOT EXISTS "idx_ct_assigned_to" ON "collection_tasks"("assigned_to");
                CREATE INDEX IF NOT EXISTS "idx_ct_status" ON "collection_tasks"("status");
                CREATE INDEX IF NOT EXISTS "idx_ct_due_date" ON "collection_tasks"("due_date");
                CREATE INDEX IF NOT EXISTS "idx_ct_priority" ON "collection_tasks"("priority");

                COMMENT ON TABLE "collection_tasks" IS '催收任务表 - 按账龄自动生成,分配销售员,记录催收结果';
                COMMENT ON COLUMN "collection_tasks"."task_type" IS '催收类型：phone(电话) / visit(上门) / email(邮件) / letter(函件)';
                COMMENT ON COLUMN "collection_tasks"."priority" IS '优先级：low / normal / high / urgent（按逾期天数自动评估）';
                COMMENT ON COLUMN "collection_tasks"."status" IS '状态：pending(待处理) / in_progress(处理中) / completed(已完成) / cancelled(已取消)';
                "#,
            )
            .await?;

        Ok(())
        // === m0064_create_finance_alerts.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- 财务预警表（V15 P0-B04 创建）
                -- 4 类预警：ar_overdue(应收超额) / inventory_backlog(库存积压) / cash_flow_shortage(现金流不足) / budget_overrun(预算超支)
                CREATE TABLE IF NOT EXISTS "finance_alerts" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "alert_no" VARCHAR(50) NOT NULL UNIQUE,
                    -- 预警类型与级别
                    "alert_type" VARCHAR(30) NOT NULL,
                    "alert_level" VARCHAR(20) NOT NULL,
                    -- 内容
                    "title" VARCHAR(200) NOT NULL,
                    "content" TEXT NOT NULL,
                    -- 关联目标
                    "target_module" VARCHAR(50),
                    "target_id" BIGINT,
                    -- 阈值与实际值
                    "threshold_value" DECIMAL(15,2),
                    "actual_value" DECIMAL(15,2),
                    "value_unit" VARCHAR(20),
                    -- 触发与处理
                    "triggered_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "triggered_by" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    "status" VARCHAR(20) NOT NULL DEFAULT 'active',
                    "acknowledged_at" TIMESTAMPTZ,
                    "acknowledged_by" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    "resolved_at" TIMESTAMPTZ,
                    "resolved_by" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    "resolve_note" TEXT,
                    "expired_at" TIMESTAMPTZ,
                    -- 通知关联
                    "notification_id" INTEGER,
                    "remark" TEXT,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 约束
                    CONSTRAINT "chk_fa_alert_type" CHECK (
                        "alert_type" IN ('ar_overdue', 'inventory_backlog', 'cash_flow_shortage', 'budget_overrun')
                    ),
                    CONSTRAINT "chk_fa_alert_level" CHECK (
                        "alert_level" IN ('info', 'warning', 'critical')
                    ),
                    CONSTRAINT "chk_fa_status" CHECK (
                        "status" IN ('active', 'acknowledged', 'resolved', 'expired')
                    )
                );

                -- 索引（6 个）
                CREATE INDEX IF NOT EXISTS "idx_fa_alert_type" ON "finance_alerts"("alert_type");
                CREATE INDEX IF NOT EXISTS "idx_fa_alert_level" ON "finance_alerts"("alert_level");
                CREATE INDEX IF NOT EXISTS "idx_fa_status" ON "finance_alerts"("status");
                CREATE INDEX IF NOT EXISTS "idx_fa_triggered_at" ON "finance_alerts"("triggered_at");
                CREATE INDEX IF NOT EXISTS "idx_fa_target" ON "finance_alerts"("target_module", "target_id");
                CREATE INDEX IF NOT EXISTS "idx_fa_notification_id" ON "finance_alerts"("notification_id");

                COMMENT ON TABLE "finance_alerts" IS '财务预警表 - 4 类预警(ar_overdue/inventory_backlog/cash_flow_shortage/budget_overrun)主动通知管理层';
                COMMENT ON COLUMN "finance_alerts"."alert_type" IS '预警类型：ar_overdue(应收超额) / inventory_backlog(库存积压) / cash_flow_shortage(现金流不足) / budget_overrun(预算超支)';
                COMMENT ON COLUMN "finance_alerts"."alert_level" IS '预警级别：info / warning / critical';
                COMMENT ON COLUMN "finance_alerts"."status" IS '状态：active(活跃) / acknowledged(已确认) / resolved(已解决) / expired(已过期)';
                "#,
            )
            .await?;

        Ok(())
        // === m0065_add_custom_order_sample_quotation_fields.rs ===
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
        // === m0066_add_after_sales_quality_issue_id.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- ============================================================
                -- P0-B12：after_sales 表新增 quality_issue_id 关联字段
                -- ============================================================

                -- 关联质量异常 ID（指向 quality_issues.id）
                -- 业务语义：当售后工单 issue_type='complaint'（客诉）时，
                -- 自动或手工关联到生产过程的质量异常记录。
                -- 关联后可启动 8D 流程（quality_8d_report）进行根因分析。
                ALTER TABLE "after_sales"
                    ADD COLUMN IF NOT EXISTS "quality_issue_id" BIGINT;

                -- 索引：按质量异常反查售后工单
                CREATE INDEX IF NOT EXISTS "idx_after_sales_quality_issue_id"
                    ON "after_sales"("quality_issue_id");

                COMMENT ON COLUMN "after_sales"."quality_issue_id" IS '关联质量异常 ID（P0-B12：售后与质量集成锚点，指向 quality_issues.id；客诉类工单关联后可启动 8D 流程进行根因分析）';
                "#,
            )
            .await?;

        Ok(())
        // === m0067_add_logistics_waybill_sign_fields.rs ===
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
        // === m0068_create_material_shortage_tables.rs ===
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
        // === m0069_create_supplier_evaluation_records.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS "supplier_evaluation_records" (
                    "id" SERIAL PRIMARY KEY,
                    "supplier_id" INTEGER NOT NULL,
                    "evaluation_period" VARCHAR(50) NOT NULL,
                    "indicator_id" INTEGER NOT NULL,
                    "score" DECIMAL(10,2) NOT NULL,
                    "max_score" INTEGER,
                    "weighted_score" DECIMAL(10,2),
                    "evaluator_id" INTEGER,
                    "evaluation_date" DATE,
                    "remark" TEXT,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    CONSTRAINT "fk_supplier_evaluation_records_supplier"
                        FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id"),
                    CONSTRAINT "fk_supplier_evaluation_records_indicator"
                        FOREIGN KEY ("indicator_id") REFERENCES "supplier_evaluation_indicators" ("id"),
                    CONSTRAINT "chk_supplier_evaluation_records_score"
                        CHECK ("score" >= 0)
                );

                CREATE INDEX IF NOT EXISTS "idx_supplier_evaluation_records_supplier"
                    ON "supplier_evaluation_records"("supplier_id");
                CREATE INDEX IF NOT EXISTS "idx_supplier_evaluation_records_indicator"
                    ON "supplier_evaluation_records"("indicator_id");
                CREATE INDEX IF NOT EXISTS "idx_supplier_evaluation_records_period"
                    ON "supplier_evaluation_records"("evaluation_period");
                CREATE INDEX IF NOT EXISTS "idx_supplier_evaluation_records_date"
                    ON "supplier_evaluation_records"("evaluation_date");

                COMMENT ON TABLE "supplier_evaluation_records" IS '供应商评估记录表（每次评估的指标得分明细）';
                COMMENT ON COLUMN "supplier_evaluation_records"."evaluation_period" IS '评估周期（如 2024Q1）';
                COMMENT ON COLUMN "supplier_evaluation_records"."weighted_score" IS '加权得分 = score * weight / max_score';
                "#,
            )
            .await?;

        Ok(())
        // === m0070_create_user_role.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS user_role (
                    id BIGSERIAL PRIMARY KEY,
                    user_id INTEGER NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
                    role_id INTEGER NOT NULL REFERENCES "roles"("id") ON DELETE CASCADE,
                    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    assigned_by INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 防止同一对 (user_id, role_id) 重复分配
                    CONSTRAINT uniq_user_role UNIQUE (user_id, role_id)
                );

                CREATE INDEX IF NOT EXISTS idx_user_role_user_id ON user_role (user_id);
                CREATE INDEX IF NOT EXISTS idx_user_role_role_id ON user_role (role_id);

                COMMENT ON TABLE user_role IS 'V15 P1 12.1：用户-角色多对多关联表，支持一个用户多角色';
                COMMENT ON COLUMN user_role.assigned_at IS '角色分配时间，用于审计追溯';
                COMMENT ON COLUMN user_role.assigned_by IS '分配人 user_id（审计用），可能为 NULL（系统初始化）';

                -- 数据迁移：将 users.role_id 现有数据同步到 user_role 表（仅迁移非空记录）
                -- 使用 ON CONFLICT DO NOTHING 保证幂等（重复执行不报错）
                INSERT INTO user_role (user_id, role_id, assigned_at)
                SELECT u.id, u.role_id, NOW()
                FROM "users" u
                WHERE u.role_id IS NOT NULL
                ON CONFLICT (user_id, role_id) DO NOTHING;
                "#,
            )
            .await?;
        Ok(())
        // === m0071_add_sales_order_id_to_color_card_issues.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE "color_card_issues"
                    ADD COLUMN IF NOT EXISTS "sales_order_id" BIGINT;

                CREATE INDEX IF NOT EXISTS "idx_issue_sales_order_id"
                    ON "color_card_issues"("sales_order_id")
                    WHERE "sales_order_id" IS NOT NULL;

                COMMENT ON COLUMN "color_card_issues"."sales_order_id" IS
                    'V15 P1 10.3-1：关联销售订单 ID（NULL=非订单驱动发放，非 NULL=订单驱动发放）';
                "#,
            )
            .await?;
        Ok(())
        // === m0072_create_permission_delegations.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS "permission_delegations" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "delegator_id" INTEGER NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
                    "delegatee_id" INTEGER NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
                    "permission_code" VARCHAR(100) NOT NULL,
                    "valid_from" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "valid_until" TIMESTAMPTZ NOT NULL,
                    "is_chain_allowed" BOOLEAN NOT NULL DEFAULT FALSE,
                    "status" VARCHAR(20) NOT NULL DEFAULT 'active',
                    "reason" VARCHAR(500),
                    "revoked_at" TIMESTAMPTZ,
                    "revoked_by" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    "revoke_reason" VARCHAR(500),
                    "created_by" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 委托人与被委托人不可为同一人
                    CONSTRAINT "chk_no_self_delegation" CHECK ("delegator_id" <> "delegatee_id"),
                    -- valid_until 必须晚于 valid_from
                    CONSTRAINT "chk_valid_until_after_from" CHECK ("valid_until" > "valid_from"),
                    -- status 取值约束
                    CONSTRAINT "chk_delegation_status" CHECK (
                        "status" IN ('pending', 'active', 'expired', 'revoked')
                    )
                );

                CREATE INDEX IF NOT EXISTS "idx_delegation_delegator" ON "permission_delegations"("delegator_id");
                CREATE INDEX IF NOT EXISTS "idx_delegation_delegatee" ON "permission_delegations"("delegatee_id");
                CREATE INDEX IF NOT EXISTS "idx_delegation_status" ON "permission_delegations"("status");
                CREATE INDEX IF NOT EXISTS "idx_delegation_valid_until" ON "permission_delegations"("valid_until");

                COMMENT ON TABLE "permission_delegations" IS
                    'V15 P1 12.6：权限委托表，支持时限化临时委托（禁止链式委托）';
                COMMENT ON COLUMN "permission_delegations"."is_chain_allowed" IS
                    '是否允许被委托人再委托（默认 false，禁止链式委托）';
                COMMENT ON COLUMN "permission_delegations"."permission_code" IS
                    '委托的权限码（如 sales.order.approve），与 role_permissions.resource_type+action 对应';
                "#,
            )
            .await?;
        Ok(())
        // === m0073_create_role_relations.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS "role_relations" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "parent_role_code" VARCHAR(50) NOT NULL,
                    "child_role_code" VARCHAR(50) NOT NULL,
                    "relation_type" VARCHAR(30) NOT NULL,
                    "description" VARCHAR(200),
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 对于 inherit：parent 继承 child 的权限
                    -- 对于 mutual_exclusive：parent 与 child 不可同时持有
                    CONSTRAINT "chk_relation_type" CHECK (
                        "relation_type" IN ('inherit', 'mutual_exclusive')
                    ),
                    -- 防止自引用
                    CONSTRAINT "chk_no_self_relation" CHECK (
                        "parent_role_code" <> "child_role_code"
                    ),
                    -- 同一对关系不可重复
                    CONSTRAINT "uniq_role_relation" UNIQUE ("parent_role_code", "child_role_code", "relation_type")
                );

                CREATE INDEX IF NOT EXISTS "idx_role_relations_parent" ON "role_relations"("parent_role_code");
                CREATE INDEX IF NOT EXISTS "idx_role_relations_child" ON "role_relations"("child_role_code");
                CREATE INDEX IF NOT EXISTS "idx_role_relations_type" ON "role_relations"("relation_type");

                COMMENT ON TABLE "role_relations" IS
                    'V15 P1 12.2：角色关系表，支持角色继承（inherit）与互斥（mutual_exclusive）校验';
                COMMENT ON COLUMN "role_relations"."relation_type" IS
                    '关系类型：inherit（继承权限）/ mutual_exclusive（不可同时持有）';

                -- 预置继承关系：经理角色继承执行角色权限
                INSERT INTO "role_relations" ("parent_role_code", "child_role_code", "relation_type", "description") VALUES
                    ('sales_manager', 'sales_rep', 'inherit', '销售经理继承销售代表权限'),
                    ('purchase_manager', 'purchase_clerk', 'inherit', '采购经理继承采购员权限'),
                    ('inventory_manager', 'warehouse_keeper', 'inherit', '库存经理继承仓库管理员权限'),
                    ('qc_manager', 'quality_inspector', 'inherit', '质量管理经理继承质检员权限'),
                    ('finance_manager', 'accountant', 'inherit', '财务经理继承会计权限'),
                    ('hr_manager', 'hr_specialist', 'inherit', '人事经理继承人事专员权限'),
                    ('crm_manager', 'crm_rep', 'inherit', 'CRM经理继承CRM专员权限')
                ON CONFLICT ("parent_role_code", "child_role_code", "relation_type") DO NOTHING;

                -- 预置互斥关系：业务角色不可同时持有（补充 role_conflicts 表）
                INSERT INTO "role_relations" ("parent_role_code", "child_role_code", "relation_type", "description") VALUES
                    ('sales_rep', 'accountant', 'mutual_exclusive', '销售与会计互斥（防止销售操控账务）'),
                    ('sales_rep', 'cashier', 'mutual_exclusive', '销售与出纳互斥（防止销售收款舞弊）'),
                    ('purchase_clerk', 'accountant', 'mutual_exclusive', '采购与会计互斥（防止采购舞弊）'),
                    ('warehouse_keeper', 'accountant', 'mutual_exclusive', '仓库与会计互斥（防止库存账务舞弊）')
                ON CONFLICT ("parent_role_code", "child_role_code", "relation_type") DO NOTHING;
                "#,
            )
            .await?;
        Ok(())
        // === m0074_v15_p1_integrate_sql_migrations.rs ===
let db = manager.get_connection();

        // 0. 创建缺失的基础表（从 SQL 迁移整合）
        db.execute_unprepared(r#"
            CREATE TABLE IF NOT EXISTS "fabric_inspection_record" (
                "id" SERIAL PRIMARY KEY,
                "inspection_no" VARCHAR(32) NOT NULL,
                "flow_card_id" INTEGER,
                "dye_lot_no" VARCHAR(64),
                "product_id" INTEGER,
                "product_name" VARCHAR(128),
                "color_no" VARCHAR(64),
                "inspection_date" DATE NOT NULL,
                "inspector_id" INTEGER,
                "inspector_name" VARCHAR(64),
                "machine_no" VARCHAR(32),
                "scoring_system" VARCHAR(16) NOT NULL DEFAULT 'four_point',
                "inspected_yards" NUMERIC(12,2) NOT NULL DEFAULT 0,
                "fabric_width_inches" NUMERIC(8,2),
                "total_defect_points" INTEGER NOT NULL DEFAULT 0,
                "points_per_100_sq_yards" NUMERIC(10,2),
                "grade" VARCHAR(16),
                "qualification_rate" NUMERIC(5,2),
                "abc_grade" VARCHAR(4),
                "total_rolls" INTEGER NOT NULL DEFAULT 0,
                "total_roll_length" NUMERIC(12,2) NOT NULL DEFAULT 0,
                "total_roll_weight" NUMERIC(12,2) NOT NULL DEFAULT 0,
                "status" VARCHAR(16) NOT NULL DEFAULT 'pending',
                "remarks" VARCHAR(256),
                "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

            CREATE TABLE IF NOT EXISTS "dye_batch_state_rule" (
                "id" SERIAL PRIMARY KEY,
                "from_status" VARCHAR(50) NOT NULL,
                "to_status" VARCHAR(50) NOT NULL,
                "transition_code" VARCHAR(50) NOT NULL,
                "transition_name" VARCHAR(100) NOT NULL,
                "is_allowed" BOOLEAN NOT NULL DEFAULT TRUE,
                "require_operator" BOOLEAN NOT NULL DEFAULT TRUE,
                "require_equipment" BOOLEAN NOT NULL DEFAULT FALSE,
                "require_remarks" BOOLEAN NOT NULL DEFAULT FALSE,
                "validation_logic" JSONB,
                "description" TEXT,
                "is_active" BOOLEAN NOT NULL DEFAULT TRUE,
                "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE UNIQUE INDEX IF NOT EXISTS "uk_dye_batch_state_rule_trans" ON "dye_batch_state_rule" ("from_status", "to_status", "transition_code");

            CREATE TABLE IF NOT EXISTS "wage_record_detail" (
                "id" SERIAL PRIMARY KEY,
                "wage_record_id" INTEGER NOT NULL,
                "step_record_id" INTEGER NOT NULL,
                "flow_card_id" INTEGER,
                "dye_lot_no" VARCHAR(64),
                "process_route_id" INTEGER,
                "route_code" VARCHAR(32),
                "route_name" VARCHAR(64),
                "process_type" VARCHAR(32),
                "worker_id" INTEGER NOT NULL,
                "worker_name" VARCHAR(128),
                "equipment_id" INTEGER,
                "equipment_name" VARCHAR(128),
                "wage_type" VARCHAR(16) NOT NULL,
                "grade" VARCHAR(2) NOT NULL,
                "actual_quantity" DECIMAL(12,2) NOT NULL DEFAULT 0,
                "qualified_quantity" DECIMAL(12,2) NOT NULL DEFAULT 0,
                "qualification_rate" DECIMAL(6,2) NOT NULL DEFAULT 0,
                "piece_price" DECIMAL(12,4) NOT NULL DEFAULT 0,
                "time_price" DECIMAL(12,4) NOT NULL DEFAULT 0,
                "duration_minutes" INTEGER NOT NULL DEFAULT 0,
                "base_wage" DECIMAL(12,2) NOT NULL DEFAULT 0,
                "quality_bonus" DECIMAL(12,2) NOT NULL DEFAULT 0,
                "final_wage" DECIMAL(12,2) NOT NULL DEFAULT 0,
                "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
                "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

            CREATE TABLE IF NOT EXISTS "outsourcing_order_item" (
                "id" SERIAL PRIMARY KEY,
                "outsourcing_order_id" INTEGER NOT NULL,
                "product_id" INTEGER NOT NULL,
                "color_no" VARCHAR(64),
                "dye_lot_no" VARCHAR(64),
                "batch_no" VARCHAR(64),
                "warehouse_id" INTEGER,
                "quantity" DECIMAL(14,4) NOT NULL DEFAULT 0,
                "unit" VARCHAR(16) NOT NULL DEFAULT 'kg',
                "unit_cost" DECIMAL(14,4) NOT NULL DEFAULT 0,
                "total_cost" DECIMAL(14,4) NOT NULL DEFAULT 0,
                "inventory_transaction_id" INTEGER,
                "remarks" TEXT,
                "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

            CREATE TABLE IF NOT EXISTS "outsourcing_receipt" (
                "id" SERIAL PRIMARY KEY,
                "receipt_no" VARCHAR(64) NOT NULL,
                "outsourcing_order_id" INTEGER NOT NULL,
                "receipt_date" DATE NOT NULL,
                "product_id" INTEGER NOT NULL,
                "color_no" VARCHAR(64),
                "dye_lot_no" VARCHAR(64),
                "batch_no" VARCHAR(64),
                "warehouse_id" INTEGER,
                "return_quantity" DECIMAL(14,4) NOT NULL DEFAULT 0,
                "loss_quantity" DECIMAL(14,4) NOT NULL DEFAULT 0,
                "loss_type" VARCHAR(16),
                "loss_rate" DECIMAL(8,4),
                "is_loss_normal" BOOLEAN NOT NULL DEFAULT TRUE,
                "unit_cost" DECIMAL(14,4) NOT NULL DEFAULT 0,
                "total_cost" DECIMAL(14,4) NOT NULL DEFAULT 0,
                "abnormal_loss_amount" DECIMAL(14,4) NOT NULL DEFAULT 0,
                "quality_status" VARCHAR(16),
                "grade" VARCHAR(8),
                "inventory_transaction_id" INTEGER,
                "status" VARCHAR(16) NOT NULL DEFAULT 'draft',
                "remarks" TEXT,
                "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

            CREATE TABLE IF NOT EXISTS "dye_batch_rework" (
                "id" SERIAL PRIMARY KEY,
                "original_batch_id" INTEGER NOT NULL,
                "original_batch_no" VARCHAR(100) NOT NULL,
                "rework_batch_id" INTEGER,
                "rework_batch_no" VARCHAR(100),
                "rework_type" VARCHAR(50) NOT NULL,
                "rework_reason" TEXT NOT NULL,
                "original_status" VARCHAR(50) NOT NULL,
                "approved_by" INTEGER,
                "approved_at" TIMESTAMPTZ,
                "status" VARCHAR(30) NOT NULL DEFAULT 'draft',
                "started_at" TIMESTAMPTZ,
                "completed_at" TIMESTAMPTZ,
                "remarks" TEXT,
                "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
                "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
        "#).await?;

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
        // === m0075_add_email_queue_fields.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- ============================================================
                -- P1 batch-16 缺陷 6.1/6.2/6.3：邮件异步队列 + 重试 + 附件
                -- ============================================================

                -- 1. next_retry_at：下次重试时间（指数退避调度使用）
                ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "next_retry_at" TIMESTAMP;

                -- 2. attachments：附件 JSON 数组
                --    格式：[{"filename": "report.pdf", "content_base64": "...", "content_type": "application/pdf"
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
