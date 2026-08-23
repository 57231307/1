//! v15 域聚合迁移

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_v15_domain"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- 坏账准备计提表（V15 P0-B01 创建）
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
                    ADD COLUMN IF NOT EXISTS "signed_at" TIMESTAMPTZ;

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
                DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE constraint_name = 'chk_material_shortage_alerts_level' AND table_name = 'material_shortage_alerts'
    ) THEN
        ALTER TABLE "material_shortage_alerts" ADD CONSTRAINT "chk_material_shortage_alerts_level" CHECK ("level" IN ('Critical', 'Severe', 'Warning', 'Normal'));
    END IF;
END $$;
                DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE constraint_name = 'chk_material_shortage_alerts_status' AND table_name = 'material_shortage_alerts'
    ) THEN
        ALTER TABLE "material_shortage_alerts" ADD CONSTRAINT "chk_material_shortage_alerts_status" CHECK ("status" IN ('identified', 'purchase_request', 'purchase_order', 'received', 'resolved'));
    END IF;
END $$;
                DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE constraint_name = 'chk_material_shortage_alerts_shortage_nonneg' AND table_name = 'material_shortage_alerts'
    ) THEN
        ALTER TABLE "material_shortage_alerts" ADD CONSTRAINT "chk_material_shortage_alerts_shortage_nonneg" CHECK ("shortage_quantity" >= 0);
    END IF;
END $$;
                DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE constraint_name = 'chk_material_shortage_alerts_deficit_rate' AND table_name = 'material_shortage_alerts'
    ) THEN
        ALTER TABLE "material_shortage_alerts" ADD CONSTRAINT "chk_material_shortage_alerts_deficit_rate" CHECK ("deficit_rate" >= 0 AND "deficit_rate" <= 100);
    END IF;
END $$;

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

ALTER TABLE "color_card_issues"
                    ADD COLUMN IF NOT EXISTS "sales_order_id" BIGINT;

                CREATE INDEX IF NOT EXISTS "idx_issue_sales_order_id"
                    ON "color_card_issues"("sales_order_id")
                    WHERE "sales_order_id" IS NOT NULL;

                COMMENT ON COLUMN "color_card_issues"."sales_order_id" IS
                    'V15 P1 10.3-1：关联销售订单 ID（NULL=非订单驱动发放，非 NULL=订单驱动发放）';

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

-- ============================================================
                -- P1 batch-16 缺陷 6.1/6.2/6.3：邮件异步队列 + 重试 + 附件
                -- ============================================================

                -- 1. next_retry_at：下次重试时间（指数退避调度使用）
                ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "next_retry_at" TIMESTAMPTZ;

                -- 2. attachments：附件 JSON 数组
                --    格式：[{"filename": "report.pdf", "content_base64": "...", "content_type": "application/pdf"}]
                ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "attachments" JSONB;

                -- 3. html_content / text_content：区分 HTML 与纯文本正文（原 body 字段保留兼容）
                ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "html_content" TEXT;
                ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "text_content" TEXT;

                -- 索引：扫描 PENDING + next_retry_at 邮件的高频查询
                CREATE INDEX IF NOT EXISTS "idx_email_logs_pending_retry"
                    ON "email_logs"("status", "next_retry_at", "retry_count")
                    WHERE "status" = 'PENDING';

                COMMENT ON COLUMN "email_logs"."next_retry_at" IS '下次重试时间（指数退避：1min/5min/30min，NULL 表示立即可重试）';
                COMMENT ON COLUMN "email_logs"."attachments" IS '附件 JSON 数组：[{filename, content_base64, content_type}]';
                COMMENT ON COLUMN "email_logs"."html_content" IS 'HTML 正文（与 body 区分，body 保留为兼容字段）';
                COMMENT ON COLUMN "email_logs"."text_content" IS '纯文本正文';

-- ============================================================
                -- V15 P1-3-3：audit_logs 表导出专属字段
                -- ============================================================

                ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_record_count" INTEGER;
                ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_query_filter" TEXT;
                ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_file_format" VARCHAR(20);
                ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_approval_token" VARCHAR(128);
                ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_watermark_user" VARCHAR(100);

                -- 索引：按导出条数筛选大批量导出（合规审查常用）
                CREATE INDEX IF NOT EXISTS "idx_audit_log_export_count"
                    ON "audit_logs"("export_record_count");
                -- 索引：按审批 token 查询敏感数据导出追溯
                CREATE INDEX IF NOT EXISTS "idx_audit_log_approval_token"
                    ON "audit_logs"("export_approval_token");

                COMMENT ON COLUMN "audit_logs"."export_record_count" IS 'V15 P1-3-3：导出数据行数，用于大批量导出识别（>80% 上限触发告警）';
                COMMENT ON COLUMN "audit_logs"."export_query_filter" IS 'V15 P1-3-3：导出时的筛选条件 JSON，用于追溯导出数据范围';
                COMMENT ON COLUMN "audit_logs"."export_file_format" IS 'V15 P1-3-3：导出文件格式（xlsx/csv/pdf），格式合规审计';
                COMMENT ON COLUMN "audit_logs"."export_approval_token" IS 'V15 P1-3-3：二级审批 token（敏感数据导出），10 分钟有效期';
                COMMENT ON COLUMN "audit_logs"."export_watermark_user" IS 'V15 P1-3-3：导出文件水印中的用户名，二次泄露追溯';

-- ============================================================
                -- P1 batch-16 缺陷 7.2：OA 公告可见性控制
                -- ============================================================

                -- visibility_scope：可见性范围枚举
                --   ALL=全员可见（默认）
                --   DEPT=指定部门可见（visible_scope_config = {"department_ids": [1,2,3]}）
                --   ROLE=指定角色可见（visible_scope_config = {"role_ids": [1,2,3]}）
                --   CUSTOM=自定义用户列表（visible_scope_config = {"user_ids": [1,2,3]}）
                ALTER TABLE "oa_announcement" ADD COLUMN IF NOT EXISTS "visibility_scope" VARCHAR(20) NOT NULL DEFAULT 'ALL';
                ALTER TABLE "oa_announcement" ADD COLUMN IF NOT EXISTS "visible_scope_config" JSONB;

                COMMENT ON COLUMN "oa_announcement"."visibility_scope" IS '可见性范围：ALL=全员/DEPT=指定部门/ROLE=指定角色/CUSTOM=自定义用户';
                COMMENT ON COLUMN "oa_announcement"."visible_scope_config" IS '可见性配置 JSON：{"department_ids":[...]}/{"role_ids":[...]}/{"user_ids":[...]}';

                -- ============================================================
                -- P1 batch-16 缺陷 7.3：用户行为采集隐私合规
                -- ============================================================

                -- user_consents：用户隐私同意记录表
                -- 每次 consent 变更新增一条记录，保留审计轨迹
                CREATE TABLE IF NOT EXISTS "user_consents" (
                    "id" SERIAL PRIMARY KEY,
                    "user_id" INTEGER NOT NULL,
                    "consent_type" VARCHAR(50) NOT NULL,
                    "consent_given" BOOLEAN NOT NULL,
                    "consent_text_version" VARCHAR(20),
                    "consented_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "revoked_at" TIMESTAMPTZ,
                    "ip_address" VARCHAR(64),
                    "user_agent" VARCHAR(512),
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );

                -- 单用户单类型当前最新状态查询索引
                CREATE INDEX IF NOT EXISTS "idx_user_consents_user_type"
                    ON "user_consents"("user_id", "consent_type", "consented_at" DESC);

                -- 约束：consent_type 必须为预定义类型
                ALTER TABLE "user_consents" DROP CONSTRAINT IF EXISTS "chk_user_consents_consent_type";
                DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE constraint_name = 'chk_user_consents_consent_type' AND table_name = 'user_consents'
    ) THEN
        ALTER TABLE "user_consents" ADD CONSTRAINT "chk_user_consents_consent_type" CHECK ("consent_type" IN ('behavior_tracking', 'page_view_tracking', 'cookie_usage', 'marketing_email'));
    END IF;
END $$;

                COMMENT ON TABLE "user_consents" IS '用户隐私同意记录表（GDPR/个人信息保护法合规）';
                COMMENT ON COLUMN "user_consents"."consent_type" IS '同意类型：behavior_tracking/page_view_tracking/cookie_usage/marketing_email';
                COMMENT ON COLUMN "user_consents"."consent_given" IS '是否同意：true=同意采集，false=退出';
                COMMENT ON COLUMN "user_consents"."consent_text_version" IS '隐私政策文本版本号（如 v1.0）';

                -- ============================================================
                -- P1 batch-16 缺陷 8.3/8.4：用户行为日志 90 天保留策略归档表
                -- ============================================================

                -- page_view_daily_summary：按 path + date 聚合的页面访问汇总
                CREATE TABLE IF NOT EXISTS "page_view_daily_summary" (
                    "stat_date" DATE NOT NULL,
                    "path" VARCHAR(2048) NOT NULL,
                    "total_views" BIGINT NOT NULL DEFAULT 0,
                    "unique_sessions" BIGINT NOT NULL DEFAULT 0,
                    "unique_users" BIGINT NOT NULL DEFAULT 0,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    PRIMARY KEY ("stat_date", "path")
                );

                COMMENT ON TABLE "page_view_daily_summary" IS '页面访问日聚合表（page_views 90 天归档汇总目标）';

                -- user_behavior_daily_summary：按 event_type + date 聚合的行为汇总
                CREATE TABLE IF NOT EXISTS "user_behavior_daily_summary" (
                    "stat_date" DATE NOT NULL,
                    "event_type" VARCHAR(128) NOT NULL,
                    "total_count" BIGINT NOT NULL DEFAULT 0,
                    "unique_users" BIGINT NOT NULL DEFAULT 0,
                    "unique_sessions" BIGINT NOT NULL DEFAULT 0,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    PRIMARY KEY ("stat_date", "event_type")
                );

                COMMENT ON TABLE "user_behavior_daily_summary" IS '用户行为日聚合表（user_behaviors 90 天归档汇总目标）';

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
                ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "permanent_action_completed_at" TIMESTAMPTZ;
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
                ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "approved_at_fin" TIMESTAMPTZ;
                ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "approved_at_gm" TIMESTAMPTZ;
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
                ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "approval_level" VARCHAR(20);
                ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "approved_by_role" VARCHAR(50);
                ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "total_amount" DECIMAL(14,2) DEFAULT 0;
                COMMENT ON COLUMN "inventory_transfers"."approval_level" IS '审批层级：L1=常规/L2=经理/L3=总监';
                COMMENT ON COLUMN "inventory_transfers"."approved_by_role" IS '审批人角色记录';
                COMMENT ON COLUMN "inventory_transfers"."total_amount" IS '调拨总金额（数量×unit_cost 累计）用于分级审批';

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
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_work_center_equipment_wc" ON "work_center_equipment"("work_center_id");

                -- 工作中心-人员关联表（含多技能）
                CREATE TABLE IF NOT EXISTS "work_center_worker" (
                    "id" SERIAL PRIMARY KEY,
                    "work_center_id" INTEGER NOT NULL,
                    "user_id" INTEGER NOT NULL,
                    "skills" JSONB,
                    "is_primary" BOOLEAN NOT NULL DEFAULT FALSE,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
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
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_work_center_shift_wc" ON "work_center_shift"("work_center_id");

                -- ============================================================
                -- P1 batch-18 缺陷 3.3：piece_mapping 表删除（改用 inventory_piece.parent_piece_id）
                -- ============================================================
                DROP TABLE IF EXISTS "piece_mapping";

-- ============================================================
                -- 缺陷 10：销售合同电子签章字段（《电子签名法》合规）
                -- ============================================================
                ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "signed_at" TIMESTAMPTZ;
                ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "signed_by_user_id" INTEGER;
                ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "signature_hash" VARCHAR(64);
                ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "signature_image_url" VARCHAR(500);
                ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "signature_certificate" TEXT;
                COMMENT ON COLUMN "sales_contracts"."signed_at" IS '电子签章时间';
                COMMENT ON COLUMN "sales_contracts"."signed_by_user_id" IS '签章人用户ID';
                COMMENT ON COLUMN "sales_contracts"."signature_hash" IS '合同内容哈希（SHA-256，防篡改）';
                COMMENT ON COLUMN "sales_contracts"."signature_image_url" IS '电子签章图片URL';
                COMMENT ON COLUMN "sales_contracts"."signature_certificate" IS 'CA证书内容（PEM格式）';

                -- ============================================================
                -- 缺陷 13：委外凭证进项税转出字段（增值税合规）
                -- ============================================================
                ALTER TABLE "outsourcing_voucher" ADD COLUMN IF NOT EXISTS "tax_transfer_amount" DECIMAL(14,4) NOT NULL DEFAULT 0;
                COMMENT ON COLUMN "outsourcing_voucher"."tax_transfer_amount" IS '进项税转出金额（非正常损耗对应的已抵扣进项税转出）';

                -- ============================================================
                -- 缺陷 14：出口退税（免抵退）核算表
                -- 依据：财税[2012]39号 出口货物劳务增值税和消费税政策
                -- ============================================================

                -- 出口报关单
                CREATE TABLE IF NOT EXISTS "export_customs_declarations" (
                    "id" SERIAL PRIMARY KEY,
                    "declaration_no" VARCHAR(50) NOT NULL UNIQUE,
                    "sales_order_id" INTEGER,
                    "customer_id" INTEGER,
                    "product_id" INTEGER,
                    "export_date" DATE NOT NULL,
                    "destination_country" VARCHAR(100),
                    "currency_code" VARCHAR(10),
                    "total_amount" DECIMAL(14,2) NOT NULL DEFAULT 0,
                    "exchange_rate" DECIMAL(10,4) NOT NULL DEFAULT 1,
                    "customs_code" VARCHAR(50),
                    "status" VARCHAR(20) NOT NULL DEFAULT 'pending',
                    "remarks" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_export_customs_dec_order" ON "export_customs_declarations"("sales_order_id");
                CREATE INDEX IF NOT EXISTS "idx_export_customs_dec_status" ON "export_customs_declarations"("status");

                -- 外汇核销单
                CREATE TABLE IF NOT EXISTS "foreign_exchange_verifications" (
                    "id" SERIAL PRIMARY KEY,
                    "verification_no" VARCHAR(50) NOT NULL UNIQUE,
                    "customs_declaration_id" INTEGER,
                    "sales_order_id" INTEGER,
                    "verification_date" DATE NOT NULL,
                    "foreign_currency_amount" DECIMAL(14,2) NOT NULL DEFAULT 0,
                    "rmb_amount" DECIMAL(14,2) NOT NULL DEFAULT 0,
                    "exchange_rate" DECIMAL(10,4) NOT NULL DEFAULT 1,
                    "bank_code" VARCHAR(50),
                    "status" VARCHAR(20) NOT NULL DEFAULT 'pending',
                    "remarks" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_fx_verification_customs" ON "foreign_exchange_verifications"("customs_declaration_id");

                -- 出口退税申报表（免抵退）
                CREATE TABLE IF NOT EXISTS "export_refund_declarations" (
                    "id" SERIAL PRIMARY KEY,
                    "declaration_no" VARCHAR(50) NOT NULL UNIQUE,
                    "period_year" INTEGER NOT NULL,
                    "period_month" INTEGER NOT NULL,
                    "declaration_date" DATE NOT NULL,
                    "export_sales_amount" DECIMAL(14,2) NOT NULL DEFAULT 0,
                    "refundable_vat_amount" DECIMAL(14,2) NOT NULL DEFAULT 0,
                    "exempt_vat_amount" DECIMAL(14,2) NOT NULL DEFAULT 0,
                    "credit_vat_amount" DECIMAL(14,2) NOT NULL DEFAULT 0,
                    "actual_refund_amount" DECIMAL(14,2) NOT NULL DEFAULT 0,
                    "carryforward_amount" DECIMAL(14,2) NOT NULL DEFAULT 0,
                    "refund_rate" DECIMAL(6,4) NOT NULL DEFAULT 0,
                    "documents_complete" BOOLEAN NOT NULL DEFAULT FALSE,
                    "status" VARCHAR(20) NOT NULL DEFAULT 'draft',
                    "remarks" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_export_refund_period" ON "export_refund_declarations"("period_year", "period_month");
                CREATE INDEX IF NOT EXISTS "idx_export_refund_status" ON "export_refund_declarations"("status");

                -- ============================================================
                -- 缺陷 15：污染物排放记录表（环保税核算基础）
                -- 依据：《环境保护税法》印染企业废水/废气/固废排放
                -- ============================================================
                CREATE TABLE IF NOT EXISTS "pollutant_discharge_records" (
                    "id" SERIAL PRIMARY KEY,
                    "discharge_type" VARCHAR(20) NOT NULL,
                    "pollutant_name" VARCHAR(50) NOT NULL,
                    "discharge_amount" DECIMAL(14,4) NOT NULL DEFAULT 0,
                    "discharge_unit" VARCHAR(20) NOT NULL DEFAULT 'kg',
                    "concentration" DECIMAL(14,4),
                    "concentration_unit" VARCHAR(20),
                    "tax_unit_equivalent" DECIMAL(14,4),
                    "tax_amount" DECIMAL(14,2) NOT NULL DEFAULT 0,
                    "period_year" INTEGER NOT NULL,
                    "period_month" INTEGER NOT NULL,
                    "monitoring_point" VARCHAR(200),
                    "remarks" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_pollutant_discharge_period" ON "pollutant_discharge_records"("period_year", "period_month");
                CREATE INDEX IF NOT EXISTS "idx_pollutant_discharge_type" ON "pollutant_discharge_records"("discharge_type");

                -- ============================================================
                -- 缺陷 18：排污许可证登记表
                -- 依据：《环境保护法》第45条 + 《排污许可管理条例》
                -- ============================================================
                CREATE TABLE IF NOT EXISTS "pollution_permits" (
                    "id" SERIAL PRIMARY KEY,
                    "permit_no" VARCHAR(100) NOT NULL UNIQUE,
                    "permit_type" VARCHAR(30) NOT NULL,
                    "permit_category" VARCHAR(30),
                    "issue_date" DATE NOT NULL,
                    "expiry_date" DATE NOT NULL,
                    "issuing_authority" VARCHAR(200) NOT NULL,
                    "permitted_capacity" DECIMAL(14,4),
                    "capacity_unit" VARCHAR(20),
                    "permitted_pollutants" JSONB,
                    "status" VARCHAR(20) NOT NULL DEFAULT 'active',
                    "remarks" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_pollution_permit_status" ON "pollution_permits"("status");
                CREATE INDEX IF NOT EXISTS "idx_pollution_permit_expiry" ON "pollution_permits"("expiry_date");

                -- ============================================================
                -- 缺陷 19：污染物监测记录表 + 固废处置联单
                -- 依据：《水污染防治法》《大气污染防治法》《固废污染防治法》
                -- ============================================================

                -- 污染物监测记录（实时/定期监测）
                CREATE TABLE IF NOT EXISTS "pollutant_monitoring_records" (
                    "id" SERIAL PRIMARY KEY,
                    "monitoring_type" VARCHAR(20) NOT NULL,
                    "monitoring_point" VARCHAR(200) NOT NULL,
                    "pollutant_name" VARCHAR(50) NOT NULL,
                    "measured_value" DECIMAL(14,4) NOT NULL,
                    "unit" VARCHAR(20) NOT NULL,
                    "limit_value" DECIMAL(14,4) NOT NULL,
                    "is_exceeding" BOOLEAN NOT NULL DEFAULT FALSE,
                    "exceeding_ratio" DECIMAL(10,4),
                    "monitoring_time" TIMESTAMPTZ NOT NULL,
                    "monitoring_method" VARCHAR(50),
                    "equipment_id" INTEGER,
                    "operator_id" INTEGER,
                    "remarks" TEXT,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_pollutant_monitoring_type" ON "pollutant_monitoring_records"("monitoring_type");
                CREATE INDEX IF NOT EXISTS "idx_pollutant_monitoring_time" ON "pollutant_monitoring_records"("monitoring_time");
                CREATE INDEX IF NOT EXISTS "idx_pollutant_monitoring_exceeding" ON "pollutant_monitoring_records"("is_exceeding");

                -- 固废处置联单（危废转移联单制度）
                CREATE TABLE IF NOT EXISTS "solid_waste_disposal_records" (
                    "id" SERIAL PRIMARY KEY,
                    "manifest_no" VARCHAR(50) NOT NULL UNIQUE,
                    "waste_type" VARCHAR(50) NOT NULL,
                    "waste_category" VARCHAR(30) NOT NULL,
                    "waste_amount" DECIMAL(14,4) NOT NULL DEFAULT 0,
                    "waste_unit" VARCHAR(20) NOT NULL DEFAULT 'ton',
                    "generation_date" DATE NOT NULL,
                    "disposal_date" DATE,
                    "disposal_method" VARCHAR(50) NOT NULL,
                    "disposal_vendor_id" INTEGER,
                    "disposal_vendor_name" VARCHAR(200),
                    "transport_license_no" VARCHAR(100),
                    "disposal_license_no" VARCHAR(100),
                    "status" VARCHAR(20) NOT NULL DEFAULT 'pending',
                    "remarks" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_solid_waste_status" ON "solid_waste_disposal_records"("status");
                CREATE INDEX IF NOT EXISTS "idx_solid_waste_type" ON "solid_waste_disposal_records"("waste_type");

                -- ============================================================
                -- 缺陷 21：劳动合同电子化管理
                -- 依据：《劳动法》《劳动合同法》第10/19/20条
                -- ============================================================
                CREATE TABLE IF NOT EXISTS "labor_contracts" (
                    "id" SERIAL PRIMARY KEY,
                    "worker_id" INTEGER NOT NULL,
                    "contract_no" VARCHAR(50) NOT NULL UNIQUE,
                    "contract_type" VARCHAR(30) NOT NULL,
                    "start_date" DATE NOT NULL,
                    "end_date" DATE,
                    "probation_end_date" DATE,
                    "probation_salary" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "regular_salary" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "position" VARCHAR(100),
                    "department" VARCHAR(100),
                    "work_location" VARCHAR(200),
                    "working_hours_system" VARCHAR(30) NOT NULL DEFAULT 'standard',
                    "sign_date" DATE NOT NULL,
                    "status" VARCHAR(20) NOT NULL DEFAULT 'active',
                    "termination_date" DATE,
                    "termination_reason" TEXT,
                    "remarks" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_labor_contract_worker" ON "labor_contracts"("worker_id");
                CREATE INDEX IF NOT EXISTS "idx_labor_contract_status" ON "labor_contracts"("status");
                CREATE INDEX IF NOT EXISTS "idx_labor_contract_end_date" ON "labor_contracts"("end_date");

                -- ============================================================
                -- 缺陷 23：社保公积金缴纳记录
                -- 依据：《社会保险法》第58条 + 《住房公积金管理条例》第14条
                -- ============================================================
                CREATE TABLE IF NOT EXISTS "social_insurance_records" (
                    "id" SERIAL PRIMARY KEY,
                    "worker_id" INTEGER NOT NULL,
                    "period_year" INTEGER NOT NULL,
                    "period_month" INTEGER NOT NULL,
                    "base_amount" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "pension_employer" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "pension_employee" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "medical_employer" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "medical_employee" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "unemployment_employer" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "unemployment_employee" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "work_injury_employer" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "maternity_employer" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "housing_fund_employer" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "housing_fund_employee" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "total_employer" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "total_employee" DECIMAL(12,2) NOT NULL DEFAULT 0,
                    "status" VARCHAR(20) NOT NULL DEFAULT 'pending',
                    "payment_date" DATE,
                    "remarks" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_social_insurance_worker" ON "social_insurance_records"("worker_id");
                CREATE INDEX IF NOT EXISTS "idx_social_insurance_period" ON "social_insurance_records"("period_year", "period_month");

                -- ============================================================
                -- 缺陷 24：职业健康合规
                -- 依据：《职业病防治法》第26/35条 + 《危险化学品安全管理条例》
                -- ============================================================

                -- 职业危害因素检测记录（苯/甲醛/噪声/粉尘）
                CREATE TABLE IF NOT EXISTS "occupational_hazard_monitorings" (
                    "id" SERIAL PRIMARY KEY,
                    "hazard_type" VARCHAR(30) NOT NULL,
                    "hazard_name" VARCHAR(50) NOT NULL,
                    "monitoring_point" VARCHAR(200) NOT NULL,
                    "measured_value" DECIMAL(14,4) NOT NULL,
                    "unit" VARCHAR(20) NOT NULL,
                    "limit_value" DECIMAL(14,4) NOT NULL,
                    "is_exceeding" BOOLEAN NOT NULL DEFAULT FALSE,
                    "exceeding_ratio" DECIMAL(10,4),
                    "monitoring_date" DATE NOT NULL,
                    "monitoring_organization" VARCHAR(200),
                    "monitoring_method" VARCHAR(100),
                    "report_url" VARCHAR(500),
                    "remarks" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_occ_hazard_type" ON "occupational_hazard_monitorings"("hazard_type");
                CREATE INDEX IF NOT EXISTS "idx_occ_hazard_date" ON "occupational_hazard_monitorings"("monitoring_date");
                CREATE INDEX IF NOT EXISTS "idx_occ_hazard_exceeding" ON "occupational_hazard_monitorings"("is_exceeding");

                -- 职业健康体检档案（上岗前/在岗期间/离岗时）
                CREATE TABLE IF NOT EXISTS "occupational_health_exams" (
                    "id" SERIAL PRIMARY KEY,
                    "worker_id" INTEGER NOT NULL,
                    "exam_type" VARCHAR(30) NOT NULL,
                    "exam_date" DATE NOT NULL,
                    "next_exam_date" DATE,
                    "exam_organization" VARCHAR(200),
                    "exam_result" VARCHAR(20) NOT NULL DEFAULT 'normal',
                    "hazard_exposure" JSONB,
                    "contraindications" TEXT,
                    "report_url" VARCHAR(500),
                    "remarks" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_occ_health_exam_worker" ON "occupational_health_exams"("worker_id");
                CREATE INDEX IF NOT EXISTS "idx_occ_health_exam_type" ON "occupational_health_exams"("exam_type");
                CREATE INDEX IF NOT EXISTS "idx_occ_health_exam_next" ON "occupational_health_exams"("next_exam_date");

                -- 防护用品配备记录（PPE）
                CREATE TABLE IF NOT EXISTS "ppe_distribution_records" (
                    "id" SERIAL PRIMARY KEY,
                    "worker_id" INTEGER NOT NULL,
                    "ppe_name" VARCHAR(100) NOT NULL,
                    "ppe_type" VARCHAR(50) NOT NULL,
                    "specification" VARCHAR(100),
                    "quantity" INTEGER NOT NULL DEFAULT 1,
                    "distribution_date" DATE NOT NULL,
                    "expiry_date" DATE,
                    "hazard_type" VARCHAR(30),
                    "status" VARCHAR(20) NOT NULL DEFAULT 'distributed',
                    "remarks" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS "idx_ppe_worker" ON "ppe_distribution_records"("worker_id");
                CREATE INDEX IF NOT EXISTS "idx_ppe_date" ON "ppe_distribution_records"("distribution_date");

-- 催收模板表（V15 P1 17.3-D5 创建）
                -- 按催收类型与逾期阶段配置标准化话术
                CREATE TABLE IF NOT EXISTS "collection_templates" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "name" VARCHAR(100) NOT NULL,
                    "task_type" VARCHAR(20) NOT NULL,
                    "overdue_stage" VARCHAR(20) NOT NULL DEFAULT 'all',
                    "title" VARCHAR(200),
                    "content" TEXT NOT NULL,
                    "is_enabled" BOOLEAN NOT NULL DEFAULT TRUE,
                    "sort_order" INTEGER NOT NULL DEFAULT 0,
                    "remark" TEXT,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    CONSTRAINT "uk_ct_name" UNIQUE ("name"),
                    CONSTRAINT "chk_ct_task_type_ext" CHECK (
                        "task_type" IN ('phone', 'visit', 'email', 'letter')
                    ),
                    CONSTRAINT "chk_ct_overdue_stage" CHECK (
                        "overdue_stage" IN ('early', 'middle', 'late', 'all')
                    )
                );

                -- 索引
                CREATE INDEX IF NOT EXISTS "idx_ctpl_task_type" ON "collection_templates"("task_type");
                CREATE INDEX IF NOT EXISTS "idx_ctpl_overdue_stage" ON "collection_templates"("overdue_stage");
                CREATE INDEX IF NOT EXISTS "idx_ctpl_is_enabled" ON "collection_templates"("is_enabled");

                COMMENT ON TABLE "collection_templates" IS '催收模板表 - 按类型/阶段配置标准化话术';
                COMMENT ON COLUMN "collection_templates"."task_type" IS '催收类型：phone(电话) / visit(上门) / email(邮件) / letter(函件)';
                COMMENT ON COLUMN "collection_templates"."overdue_stage" IS '适用逾期阶段：early(0-30天) / middle(31-90天) / late(90+天) / all(全部)';

                -- 预置 4 套默认模板（按催收类型）
                INSERT INTO "collection_templates" ("name", "task_type", "overdue_stage", "title", "content", "is_enabled", "sort_order", "remark") VALUES
                ('电话催收-早期模板', 'phone', 'early', NULL, '您好，我是XX公司的财务专员，您有一笔账款已逾期{overdue_days}天，金额{overdue_amount}元，请尽快安排付款，谢谢配合。', TRUE, 1, '默认电话催收话术-早期'),
                ('上门催收-中期模板', 'visit', 'middle', NULL, '尊敬的客户，您司账款已逾期{overdue_days}天，累计欠款{overdue_amount}元。我司将安排专人上门沟通，请配合核实并安排付款。', TRUE, 1, '默认上门催收话术-中期'),
                ('邮件催收-通用模板', 'email', 'all', '【账款催收通知】逾期{overdue_days}天 - 金额{overdue_amount}元', '尊敬的客户：\n\n经核对，您司于我司的应收账款已逾期{overdue_days}天，未付金额{overdue_amount}元。\n\n请于收到本邮件后7个工作日内安排付款，如有疑问请及时联系我司财务部。\n\n此致\nXX公司财务部', TRUE, 1, '默认邮件催收话术-通用'),
                ('函件催收-晚期模板', 'letter', 'late', '关于催收逾期账款的函', '致：{customer_name}\n\n经我司财务部门核对，截至发函日，贵司尚欠我司货款{overdue_amount}元，已逾期{overdue_days}天。\n\n鉴于逾期时间较长，特发此函正式催收。请贵司于收到本函后15日内付清上述款项，逾期未付我司将依法采取进一步措施。\n\n特此函告。\n\nXX公司\n{date}', TRUE, 1, '默认函件催收话术-晚期')
                ON CONFLICT ("name") DO NOTHING;

-- 固定资产盘点单表（V15 P1 17.8-D4 创建）
                CREATE TABLE IF NOT EXISTS "fixed_asset_counts" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "count_no" VARCHAR(50) NOT NULL,
                    "plan_name" VARCHAR(200) NOT NULL,
                    "count_date" DATE NOT NULL,
                    "asset_category" VARCHAR(100),
                    "use_location" VARCHAR(200),
                    "status" VARCHAR(20) NOT NULL DEFAULT 'DRAFT',
                    "total_items" INTEGER NOT NULL DEFAULT 0,
                    "counted_items" INTEGER NOT NULL DEFAULT 0,
                    "surplus_items" INTEGER NOT NULL DEFAULT 0,
                    "shortage_items" INTEGER NOT NULL DEFAULT 0,
                    "notes" TEXT,
                    "created_by" INTEGER NOT NULL,
                    "approved_by" INTEGER,
                    "completed_at" TIMESTAMPTZ,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    CONSTRAINT "uk_fac_count_no" UNIQUE ("count_no"),
                    CONSTRAINT "chk_fac_status" CHECK (
                        "status" IN ('DRAFT', 'COUNTING', 'COMPLETED')
                    )
                );

                -- 固定资产盘点明细表
                CREATE TABLE IF NOT EXISTS "fixed_asset_count_items" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "count_id" BIGINT NOT NULL,
                    "asset_id" INTEGER NOT NULL,
                    "asset_no" VARCHAR(50) NOT NULL,
                    "asset_name" VARCHAR(200) NOT NULL,
                    "book_original_value" DECIMAL(18,2) NOT NULL,
                    "book_net_value" DECIMAL(18,2),
                    "book_use_location" VARCHAR(200),
                    "actual_original_value" DECIMAL(18,2),
                    "actual_net_value" DECIMAL(18,2),
                    "actual_use_location" VARCHAR(200),
                    "count_result" VARCHAR(20),
                    "variance_type" VARCHAR(20),
                    "variance_amount" DECIMAL(18,2),
                    "remarks" TEXT,
                    "counted_by" INTEGER,
                    "counted_at" TIMESTAMPTZ,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    CONSTRAINT "fk_fac_count_id" FOREIGN KEY ("count_id")
                        REFERENCES "fixed_asset_counts"("id") ON DELETE CASCADE,
                    CONSTRAINT "uk_fac_count_asset" UNIQUE ("count_id", "asset_id"),
                    CONSTRAINT "chk_fac_count_result" CHECK (
                        "count_result" IN ('consistent', 'surplus', 'shortage', 'damaged')
                    )
                );

                -- 索引
                CREATE INDEX IF NOT EXISTS "idx_fac_status" ON "fixed_asset_counts"("status");
                CREATE INDEX IF NOT EXISTS "idx_fac_count_date" ON "fixed_asset_counts"("count_date");
                CREATE INDEX IF NOT EXISTS "idx_faci_count_id" ON "fixed_asset_count_items"("count_id");
                CREATE INDEX IF NOT EXISTS "idx_faci_asset_id" ON "fixed_asset_count_items"("asset_id");
                CREATE INDEX IF NOT EXISTS "idx_faci_variance_type" ON "fixed_asset_count_items"("variance_type");

                COMMENT ON TABLE "fixed_asset_counts" IS '固定资产盘点单 - 盘点计划-执行-差异闭环';
                COMMENT ON TABLE "fixed_asset_count_items" IS '固定资产盘点明细 - 账实对比记录';

-- =====================================================
                -- 18.4-D2：客户团队成员关联表
                -- =====================================================
                CREATE TABLE IF NOT EXISTS "customer_team_members" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "customer_id" INTEGER NOT NULL,
                    "user_id" INTEGER NOT NULL,
                    "user_name" VARCHAR(100),
                    "team_role" VARCHAR(20) NOT NULL DEFAULT 'member',
                    "is_active" BOOLEAN NOT NULL DEFAULT TRUE,
                    "joined_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "left_at" TIMESTAMPTZ,
                    "notes" TEXT,
                    "created_by" INTEGER,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    CONSTRAINT "fk_ctm_customer_id" FOREIGN KEY ("customer_id")
                        REFERENCES "customers"("id") ON DELETE CASCADE,
                    CONSTRAINT "uk_ctm_customer_user" UNIQUE ("customer_id", "user_id"),
                    CONSTRAINT "chk_ctm_team_role" CHECK (
                        "team_role" IN ('primary', 'member', 'assistant')
                    )
                );

                CREATE INDEX IF NOT EXISTS "idx_ctm_customer_id" ON "customer_team_members"("customer_id");
                CREATE INDEX IF NOT EXISTS "idx_ctm_user_id" ON "customer_team_members"("user_id");
                CREATE INDEX IF NOT EXISTS "idx_ctm_active" ON "customer_team_members"("is_active");

                COMMENT ON TABLE "customer_team_members" IS '客户团队成员关联 - 支持多人协作跟进同一客户（18.4-D2）';
                COMMENT ON COLUMN "customer_team_members"."team_role" IS '团队角色：primary=主负责人，member=团队成员，assistant=协助人员';

                -- =====================================================
                -- 18.4-D3：客户数据共享表（带时效控制）
                -- =====================================================
                CREATE TABLE IF NOT EXISTS "customer_shares" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "customer_id" INTEGER NOT NULL,
                    "shared_by_user_id" INTEGER NOT NULL,
                    "shared_by_user_name" VARCHAR(100),
                    "shared_to_user_id" INTEGER NOT NULL,
                    "shared_to_user_name" VARCHAR(100),
                    "permission" VARCHAR(20) NOT NULL DEFAULT 'view',
                    "status" VARCHAR(20) NOT NULL DEFAULT 'active',
                    "shared_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "expire_at" TIMESTAMPTZ,
                    "revoked_at" TIMESTAMPTZ,
                    "revoked_by" INTEGER,
                    "revoke_reason" TEXT,
                    "share_reason" TEXT,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    CONSTRAINT "fk_cs_customer_id" FOREIGN KEY ("customer_id")
                        REFERENCES "customers"("id") ON DELETE CASCADE,
                    CONSTRAINT "uk_cs_customer_to_user_active" UNIQUE ("customer_id", "shared_to_user_id", "status"),
                    CONSTRAINT "chk_cs_permission" CHECK (
                        "permission" IN ('view', 'edit', 'full')
                    ),
                    CONSTRAINT "chk_cs_status" CHECK (
                        "status" IN ('active', 'expired', 'revoked')
                    )
                );

                CREATE INDEX IF NOT EXISTS "idx_cs_customer_id" ON "customer_shares"("customer_id");
                CREATE INDEX IF NOT EXISTS "idx_cs_shared_to_user" ON "customer_shares"("shared_to_user_id");
                CREATE INDEX IF NOT EXISTS "idx_cs_shared_by_user" ON "customer_shares"("shared_by_user_id");
                CREATE INDEX IF NOT EXISTS "idx_cs_status" ON "customer_shares"("status");
                CREATE INDEX IF NOT EXISTS "idx_cs_expire_at" ON "customer_shares"("expire_at");

                COMMENT ON TABLE "customer_shares" IS '客户数据共享 - 支持时效控制和权限回收（18.4-D3）';
                COMMENT ON COLUMN "customer_shares"."permission" IS '共享权限：view=只读，edit=编辑，full=完全';
                COMMENT ON COLUMN "customer_shares"."status" IS '共享状态：active=生效中，expired=已过期，revoked=已撤销';
                COMMENT ON COLUMN "customer_shares"."expire_at" IS '共享过期时间，NULL 表示永久共享（建议设置时效）';

-- ============================================================
                -- P1 batch-16 缺陷 1.1：报表模板版本管理（report_template_versions）
                -- ============================================================

                CREATE TABLE IF NOT EXISTS "report_template_versions" (
                    "id" SERIAL PRIMARY KEY,
                    "template_id" INTEGER NOT NULL,
                    "version" INTEGER NOT NULL,
                    "name" VARCHAR(100) NOT NULL,
                    "code" VARCHAR(50) NOT NULL,
                    "report_type" VARCHAR(50) NOT NULL,
                    "category" VARCHAR(50),
                    "data_source" VARCHAR(100),
                    "columns" JSONB NOT NULL,
                    "filters" JSONB,
                    "parameters" JSONB,
                    "supported_formats" JSONB,
                    "sort_by" VARCHAR(100),
                    "sort_order" VARCHAR(10),
                    "data_source_sql" TEXT,
                    "description" TEXT,
                    "is_public" BOOLEAN NOT NULL DEFAULT false,
                    "required_permission" VARCHAR(100),
                    "snapshot_by" INTEGER NOT NULL,
                    "snapshot_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );

                CREATE INDEX IF NOT EXISTS "idx_report_template_versions_template_id"
                    ON "report_template_versions"("template_id", "version" DESC);

                COMMENT ON TABLE "report_template_versions" IS '报表模板历史版本表（每次 update 前写入快照，支持回滚）';
                COMMENT ON COLUMN "report_template_versions"."template_id" IS '关联的报表模板 ID';
                COMMENT ON COLUMN "report_template_versions"."version" IS '版本号（与 report_templates.version 对应）';
                COMMENT ON COLUMN "report_template_versions"."snapshot_by" IS '执行 update 的用户 ID（快照创建者）';
                COMMENT ON COLUMN "report_template_versions"."snapshot_at" IS '快照时间（update 前写入）';

                -- ============================================================
                -- P1 batch-16 缺陷 1.1/1.2：补齐 report_templates 表的 version / required_permission 字段
                -- ============================================================
                ALTER TABLE "report_templates"
                    ADD COLUMN IF NOT EXISTS "version" INTEGER NOT NULL DEFAULT 1;
                ALTER TABLE "report_templates"
                    ADD COLUMN IF NOT EXISTS "required_permission" VARCHAR(100);
                COMMENT ON COLUMN "report_templates"."version" IS '模板版本号（每次 update 前递增，支持回滚）';
                COMMENT ON COLUMN "report_templates"."required_permission" IS '必需权限码（如 report:sales:view），为空表示按 is_public/created_by 过滤';

                -- ============================================================
                -- P1 batch-16 缺陷 2.3：补齐 report_subscriptions 表的重试字段
                -- ============================================================
                ALTER TABLE "report_subscriptions"
                    ADD COLUMN IF NOT EXISTS "retry_count" INTEGER NOT NULL DEFAULT 0;
                ALTER TABLE "report_subscriptions"
                    ADD COLUMN IF NOT EXISTS "max_retries" INTEGER NOT NULL DEFAULT 3;
                ALTER TABLE "report_subscriptions"
                    ADD COLUMN IF NOT EXISTS "next_retry_at" TIMESTAMPTZ;
                COMMENT ON COLUMN "report_subscriptions"."retry_count" IS '当前重试次数（成功后清零）';
                COMMENT ON COLUMN "report_subscriptions"."max_retries" IS '最大重试次数（默认 3，超过即转入死信状态）';
                COMMENT ON COLUMN "report_subscriptions"."next_retry_at" IS '下次重试时间（按指数退避：1min/5min/30min）';

                -- ============================================================
                -- P1 batch-16 缺陷 4.1：仪表板自定义卡片持久化（dashboard_layouts）
                -- ============================================================

                CREATE TABLE IF NOT EXISTS "dashboard_layouts" (
                    "id" SERIAL PRIMARY KEY,
                    "user_id" INTEGER NOT NULL UNIQUE,
                    "card_config" JSONB NOT NULL,
                    "is_default" BOOLEAN NOT NULL DEFAULT false,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );

                CREATE INDEX IF NOT EXISTS "idx_dashboard_layouts_user_id"
                    ON "dashboard_layouts"("user_id");

                COMMENT ON TABLE "dashboard_layouts" IS '用户仪表板布局配置表（卡片顺序/可见性/尺寸）';
                COMMENT ON COLUMN "dashboard_layouts"."user_id" IS '用户 ID（每个用户独立布局）';
                COMMENT ON COLUMN "dashboard_layouts"."card_config" IS '卡片配置 JSON（卡片顺序、可见性、尺寸等）';
                COMMENT ON COLUMN "dashboard_layouts"."is_default" IS '是否默认布局（true 时其他用户初始可见）';

-- ============================================================
                -- V15 P1 batch-09 缺陷 10.4-1/10.4-2：
                -- 为非 admin 业务角色补齐 color_card_issue:export 权限
                -- ============================================================
                --
                -- 策略：以 roles.code + roles.is_system=true 为筛选条件，
                --       避免硬编码 role_id；ON CONFLICT 保证幂等。
                --
                -- 销售经理（sales_manager）：导出自己客户的色卡发放记录 + 查看成本字段
                INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
                SELECT r.id, 'color_card_issue', 'export', true, NOW(), NOW()
                FROM roles r
                WHERE r.code = 'sales_manager' AND r.is_system = true
                ON CONFLICT (role_id, resource_type, action) DO NOTHING;

                -- 仓库经理（warehouse_manager）：导出色卡发放全流程记录 + 查看成本字段
                INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
                SELECT r.id, 'color_card_issue', 'export', true, NOW(), NOW()
                FROM roles r
                WHERE r.code = 'warehouse_manager' AND r.is_system = true
                ON CONFLICT (role_id, resource_type, action) DO NOTHING;

                -- 成本会计（cost_accountant）：导出/查看色卡发放成本字段（补偿金额核算）
                INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
                SELECT r.id, 'color_card_issue', 'export', true, NOW(), NOW()
                FROM roles r
                WHERE r.code = 'cost_accountant' AND r.is_system = true
                ON CONFLICT (role_id, resource_type, action) DO NOTHING;

-- 大货批色状态变更历史表（V15 P1-10 创建）
                -- 记录 bulk_color_approval 每次状态变更的全量快照
                CREATE TABLE IF NOT EXISTS "bulk_color_approval_history" (
                    "id" BIGSERIAL PRIMARY KEY,
                    "bulk_color_approval_id" BIGINT NOT NULL REFERENCES "bulk_color_approval"("id") ON DELETE CASCADE,
                    "from_status" VARCHAR(20),
                    "to_status" VARCHAR(20) NOT NULL,
                    "operator_id" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    "reason" TEXT,
                    "snapshot" JSONB,
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );

                CREATE INDEX IF NOT EXISTS "idx_bcah_approval_id" ON "bulk_color_approval_history"("bulk_color_approval_id");
                CREATE INDEX IF NOT EXISTS "idx_bcah_to_status" ON "bulk_color_approval_history"("to_status");
                CREATE INDEX IF NOT EXISTS "idx_bcah_created_at" ON "bulk_color_approval_history"("created_at");

                COMMENT ON TABLE "bulk_color_approval_history" IS '大货批色状态变更历史 - 全量追溯每次状态流转';
                COMMENT ON COLUMN "bulk_color_approval_history"."from_status" IS '变更前状态（首次创建时为 NULL）';
                COMMENT ON COLUMN "bulk_color_approval_history"."to_status" IS '变更后状态';
                COMMENT ON COLUMN "bulk_color_approval_history"."operator_id" IS '操作人用户 ID（客户操作时为关联的业务用户）';
                COMMENT ON COLUMN "bulk_color_approval_history"."reason" IS '变更原因（拒绝/返工/降级/报废时填写）';
                COMMENT ON COLUMN "bulk_color_approval_history"."snapshot" IS '变更后记录完整快照 JSON';

ALTER TABLE "outsourcing_receipt"
                    ADD COLUMN IF NOT EXISTS "inspection_id" INTEGER;

                COMMENT ON COLUMN "outsourcing_receipt"."inspection_id" IS
                    '缺陷 2.2：关联质检记录 ID（确认收回时自动创建质检记录并回写）';

                CREATE INDEX IF NOT EXISTS "idx_outsourcing_receipt_inspection_id"
                    ON "outsourcing_receipt" ("inspection_id")
                    WHERE "inspection_id" IS NOT NULL;

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
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
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
                ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "customer_approved_at" TIMESTAMPTZ;
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
                ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "approved_at" TIMESTAMPTZ;
                ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "rejection_reason" TEXT;
                CREATE INDEX IF NOT EXISTS "idx_custom_orders_approval_instance_id" ON "custom_orders"("approval_instance_id");
                COMMENT ON COLUMN "custom_orders"."approval_instance_id" IS 'V15 P1 batch-19 缺陷 23.2.3：BPM 变更审批实例 ID（非 draft 状态变更走二级审批）';
                COMMENT ON COLUMN "custom_orders"."approved_by" IS 'V15 P1 batch-19 缺陷 23.2.3：审批人 user_id';
                COMMENT ON COLUMN "custom_orders"."approved_at" IS 'V15 P1 batch-19 缺陷 23.2.3：审批时间';
                COMMENT ON COLUMN "custom_orders"."rejection_reason" IS 'V15 P1 batch-19 缺陷 23.2.3：审批拒绝原因';

                -- ============================================================
                -- 缺陷 23.3.2：售后流程闭环（受理+评价）字段
                -- ============================================================
                ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "accepted_at" TIMESTAMPTZ;
                ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "evaluation_score" INTEGER;
                ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "evaluation_comment" TEXT;
                ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "evaluated_at" TIMESTAMPTZ;
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
                    "event_time" TIMESTAMPTZ NOT NULL,
                    "location" VARCHAR(200),
                    "description" VARCHAR(500) NOT NULL,
                    "event_type" VARCHAR(30) NOT NULL,
                    "data_source" VARCHAR(20) NOT NULL DEFAULT 'manual',
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
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

-- V15 缺陷 10-4 修复：审计日志导出二次审计机制（防篡改）
--
-- 背景：
--   原 export_audit_logs handler 仅把"导出操作"写回 audit_logs 表本身，
--   审计员（admin）可查/改自身导出记录，无法满足"审计员不能篡改自身记录"的合规要求
--   （SOC2 / ISO27001 / 中国《数据安全法》第 32 条）。
--
-- 方案：
--   新建独立表 audit_log_export_log，记录每一次审计日志导出操作，
--   通过触发器禁止 UPDATE / DELETE（仅允许 INSERT），实现防篡改。
--   导出文件 SHA256 指纹留存，支持事后比对验证文件未被替换。
--
-- 关联：
--   - .monkeycode/doto.md §0.0.2 打印功能未完成项 缺陷 10-4
--   - .monkeycode/docs/audits/v15/batch-11/audit-report.md

CREATE TABLE IF NOT EXISTS "audit_log_export_log" (
    "id"                       SERIAL PRIMARY KEY,
    "exporter_user_id"         INTEGER NOT NULL,
    "exporter_username"        VARCHAR(255) NOT NULL,
    "export_query_filter"      TEXT,
    "export_record_count"      INTEGER NOT NULL CHECK ("export_record_count" >= 0),
    "export_file_format"       VARCHAR(16) NOT NULL DEFAULT 'xlsx',
    "export_file_hash_sha256" VARCHAR(64),
    "export_file_size_bytes"   BIGINT CHECK ("export_file_size_bytes" IS NULL OR "export_file_size_bytes" >= 0),
    "export_ip_address"        VARCHAR(64),
    "export_user_agent"        TEXT,
    "export_request_id"        VARCHAR(64),
    "exported_at"              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 索引：按导出人 / 导出时间检索
CREATE INDEX IF NOT EXISTS "idx_audit_log_export_log_user_id"
    ON "audit_log_export_log" ("exporter_user_id");
CREATE INDEX IF NOT EXISTS "idx_audit_log_export_log_exported_at"
    ON "audit_log_export_log" ("exported_at" DESC);

-- ============================================================
-- 防篡改触发器：禁止 UPDATE / DELETE，仅允许 INSERT
-- ============================================================
CREATE OR REPLACE FUNCTION "fn_audit_log_export_log_immutable"()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION
        'audit_log_export_log 为防篡改表，禁止 UPDATE / DELETE 操作（导出记录只能追加）'
        USING ERRCODE = 'check_violation';
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS "trg_audit_log_export_log_no_update" ON "audit_log_export_log";
CREATE TRIGGER "trg_audit_log_export_log_no_update"
    BEFORE UPDATE ON "audit_log_export_log"
    FOR EACH ROW
    EXECUTE FUNCTION "fn_audit_log_export_log_immutable"();

DROP TRIGGER IF EXISTS "trg_audit_log_export_log_no_delete" ON "audit_log_export_log";
CREATE TRIGGER "trg_audit_log_export_log_no_delete"
    BEFORE DELETE ON "audit_log_export_log"
    FOR EACH ROW
    EXECUTE FUNCTION "fn_audit_log_export_log_immutable"();

COMMENT ON TABLE "audit_log_export_log" IS
    'V15 缺陷 10-4：审计日志导出二次审计表，防篡改（仅 INSERT，触发器禁止 UPDATE/DELETE）';
COMMENT ON COLUMN "audit_log_export_log.export_file_hash_sha256" IS
    '导出文件 SHA256 指纹，事后比对验证文件未被替换';

-- V15 P2 B05-P2-2：dye_batch_rework 表新增 rework_cost 字段
-- 记录每次回修的成本金额，按 rework_type 分类统计（re_dye 重染 / replenish_dye 补染）
-- 字段可为空（历史数据无此字段，新数据由业务层按需写入）
ALTER TABLE dye_batch_rework
    ADD COLUMN IF NOT EXISTS rework_cost NUMERIC(14, 4);

COMMENT ON COLUMN dye_batch_rework.rework_cost IS '回修成本（V15 P2 B05-P2-2）：按 rework_type 分类统计，re_dye 整缸重染成本高 / replenish_dye 局部补染成本低';

-- V15 P2 B05-P2-6：染缸设备占用/释放记录表
-- 记录染缸设备被缸号占用与释放的全生命周期，支持设备资源调度与产能可视化。
-- 唯一约束：同一 vat_id 同时只能有一条 status='occupied' 的记录（部分唯一索引）。
CREATE TABLE IF NOT EXISTS "dye_vat_occupation" (
    "id"           BIGSERIAL PRIMARY KEY,
    "vat_id"       INTEGER NOT NULL,
    "batch_id"     INTEGER NOT NULL,
    "batch_no"     VARCHAR(64),
    "occupied_at"  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "released_at"  TIMESTAMPTZ,
    "status"       VARCHAR(16) NOT NULL DEFAULT 'occupied',
    "created_at"   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at"   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 索引：按 vat_id 查询当前占用状态
CREATE INDEX IF NOT EXISTS "idx_dye_vat_occupation_vat_id"
    ON "dye_vat_occupation" ("vat_id");

-- 索引：按 batch_id 查询缸号占用的染缸
CREATE INDEX IF NOT EXISTS "idx_dye_vat_occupation_batch_id"
    ON "dye_vat_occupation" ("batch_id");

-- 部分唯一索引：同一 vat_id 同时只能有一条 status='occupied' 的记录（防重复占用）
CREATE UNIQUE INDEX IF NOT EXISTS "uq_dye_vat_occupation_vat_occupied"
    ON "dye_vat_occupation" ("vat_id")
    WHERE "status" = 'occupied';

COMMENT ON TABLE "dye_vat_occupation" IS
    'V15 P2 B05-P2-6：染缸占用记录表，缸号进入 dyeing 占用 / 离开 dyeing 释放';
COMMENT ON COLUMN "dye_vat_occupation.status" IS
    '占用状态：occupied（已占用）/ released（已释放）';

-- V15 P2 B05-P2-7：PDA / 工控终端连接资源管理表
-- 记录车间设备（PDA / 工控终端 / 扫码枪）与服务端的连接资源状态，
-- 支持注册 / 心跳 / 下线 / 超时清理的生命周期闭环。
-- 状态机：online（在线）→ offline（主动下线）/ timeout（心跳超时）
-- 唯一约束：device_id 一台设备一条记录，重复注册走应用层 upsert 路径
CREATE TABLE IF NOT EXISTS "device_connection" (
    "id"                BIGSERIAL PRIMARY KEY,
    "device_id"         VARCHAR(128) NOT NULL,
    "device_name"       VARCHAR(128),
    "device_type"       VARCHAR(32) NOT NULL DEFAULT 'other',
    "user_id"           INTEGER,
    "username"          VARCHAR(128),
    "workshop"          VARCHAR(64),
    "ip_address"        VARCHAR(64),
    "session_token"     VARCHAR(255),
    "status"            VARCHAR(16) NOT NULL DEFAULT 'online',
    "last_heartbeat_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "connected_at"      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "disconnected_at"   TIMESTAMPTZ,
    "metadata"          JSONB,
    "created_at"        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at"        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一索引：device_id 一台设备一条记录（upsert 依据）
CREATE UNIQUE INDEX IF NOT EXISTS "uq_device_connection_device_id"
    ON "device_connection" ("device_id");

-- 索引：按状态查询在线设备列表（高频查询路径）
CREATE INDEX IF NOT EXISTS "idx_device_connection_status"
    ON "device_connection" ("status");

-- 索引：按车间汇总在线设备数（看板高频查询）
CREATE INDEX IF NOT EXISTS "idx_device_connection_workshop"
    ON "device_connection" ("workshop");

-- 索引：定时任务按 last_heartbeat_at 扫描超时设备
CREATE INDEX IF NOT EXISTS "idx_device_connection_last_heartbeat"
    ON "device_connection" ("last_heartbeat_at");

COMMENT ON TABLE "device_connection" IS
    'V15 P2 B05-P2-7：PDA/工控终端连接资源管理表，注册/心跳/下线/超时清理全生命周期';
COMMENT ON COLUMN "device_connection.status" IS
    '连接状态：online（在线）/ offline（主动下线）/ timeout（心跳超时）';
COMMENT ON COLUMN "device_connection.device_type" IS
    '设备类型：pda / industrial_terminal / scanner / other';

-- V15 P2 B05-P2-10：期末调整记录表（暂估 / 摊销 / 预提）
-- 依据企业会计准则权责发生制，期末对已发生尚未入账业务做调整分录。
-- 状态机：draft(草稿) → confirmed(已确认，生成凭证) → reversed(已冲销，红字凭证) / cancelled(已取消)
-- 蓝绿部署兼容：所有字段均 NULLABLE 或带 DEFAULT，新增约束仅在事务内执行
CREATE TABLE IF NOT EXISTS "period_adjustment_record" (
    "id"                    BIGSERIAL PRIMARY KEY,
    "adjustment_no"         VARCHAR(64) NOT NULL,
    "adjustment_type"       VARCHAR(32) NOT NULL,
    "period"                VARCHAR(16) NOT NULL,
    "description"           VARCHAR(255) NOT NULL DEFAULT '',
    "debit_subject_code"    VARCHAR(32) NOT NULL,
    "debit_subject_name"    VARCHAR(128) NOT NULL,
    "credit_subject_code"   VARCHAR(32) NOT NULL,
    "credit_subject_name"   VARCHAR(128) NOT NULL,
    "amount"                DECIMAL(14, 2) NOT NULL DEFAULT 0,
    "source_type"           VARCHAR(64),
    "source_bill_id"        INTEGER,
    "source_bill_no"        VARCHAR(64),
    "voucher_id"            INTEGER,
    "reverse_voucher_id"    INTEGER,
    "status"                VARCHAR(16) NOT NULL DEFAULT 'draft',
    "confirmed_by"          INTEGER,
    "confirmed_at"          TIMESTAMPTZ,
    "reversed_by"           INTEGER,
    "reversed_at"           TIMESTAMPTZ,
    "remarks"               VARCHAR(500),
    "is_deleted"            BOOLEAN NOT NULL DEFAULT FALSE,
    "created_by"            INTEGER,
    "created_at"            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at"            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一索引：调整单号全局唯一
CREATE UNIQUE INDEX IF NOT EXISTS "uq_period_adjustment_record_no"
    ON "period_adjustment_record" ("adjustment_no");

-- 索引：按期间查询期末调整（结账时批量确认）
CREATE INDEX IF NOT EXISTS "idx_period_adjustment_record_period"
    ON "period_adjustment_record" ("period");

-- 索引：按状态过滤待确认/待冲销记录
CREATE INDEX IF NOT EXISTS "idx_period_adjustment_record_status"
    ON "period_adjustment_record" ("status");

-- 索引：按类型分类统计（暂估/摊销/预提）
CREATE INDEX IF NOT EXISTS "idx_period_adjustment_record_type"
    ON "period_adjustment_record" ("adjustment_type");

COMMENT ON TABLE "period_adjustment_record" IS
    'V15 P2 B05-P2-10：期末调整记录表，支持暂估/摊销/预提三类调整，确认生成凭证，暂估类可红字冲销';
COMMENT ON COLUMN "period_adjustment_record.adjustment_type" IS
    '调整类型：estimate(暂估) / amortization(摊销) / provision(预提)';
COMMENT ON COLUMN "period_adjustment_record.status" IS
    '状态：draft(草稿) / confirmed(已确认) / reversed(已冲销) / cancelled(已取消)';

ALTER TABLE quality_inspection_records
                    ADD COLUMN IF NOT EXISTS defect_type VARCHAR(50);

                COMMENT ON COLUMN quality_inspection_records.defect_type IS '结构化缺陷类型：color_diff(色差)/color_fastness(色牢度)/spec(规格不符)/damage(破损)/other';

DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE constraint_name = 'uk_products_code' AND table_name = 'products'
    ) THEN
        ALTER TABLE "products" ADD CONSTRAINT "uk_products_code" UNIQUE (code);
    END IF;
END $$;

-- 序列同步（INSERT 后重置序列，防止主键冲突）
SELECT setval('collection_templates_id_seq', COALESCE((SELECT MAX(id) FROM "collection_templates"), 0) + 1, false);
SELECT setval('role_relations_id_seq', COALESCE((SELECT MAX(id) FROM "role_relations"), 0) + 1, false);
SELECT setval('dye_batch_state_rule_id_seq', COALESCE((SELECT MAX(id) FROM "dye_batch_state_rule"), 0) + 1, false);
SELECT setval('user_role_id_seq', COALESCE((SELECT MAX(id) FROM "user_role"), 0) + 1, false);


-- === 从旧迁移恢复的 ALTER ADD COLUMN（确保迁移表结构与 Model 一致）===
ALTER TABLE "dye_batch_rework" ADD COLUMN IF NOT EXISTS "production_order_id" INTEGER;
ALTER TABLE "dye_batch_rework" ADD COLUMN IF NOT EXISTS "rework_cost" NUMERIC(14, 4);
ALTER TABLE "outsourcing_order_item" ADD COLUMN IF NOT EXISTS "greige_fabric_id" INTEGER;
ALTER TABLE "wage_record_detail" ADD COLUMN IF NOT EXISTS "weekday_overtime_minutes" INTEGER;
ALTER TABLE "wage_record_detail" ADD COLUMN IF NOT EXISTS "weekend_overtime_minutes" INTEGER;
ALTER TABLE "wage_record_detail" ADD COLUMN IF NOT EXISTS "overtime_pay" DECIMAL(12,2);
ALTER TABLE "wage_record_detail" ADD COLUMN IF NOT EXISTS "holiday_overtime_minutes" INTEGER;
ALTER TABLE "outsourcing_receipt" ADD COLUMN IF NOT EXISTS "inspection_id" INTEGER;


-- === 从 Model 推断补全 ALTER ADD COLUMN（确保迁移与 Model 字段一致）===
ALTER TABLE "fabric_inspection_record" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "fabric_inspection_record" ADD COLUMN IF NOT EXISTS "is_deleted" BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE "material_shortage_alerts" ADD COLUMN IF NOT EXISTS "critical_threshold" DECIMAL(18,4) NOT NULL DEFAULT 0;
ALTER TABLE "material_shortage_alerts" ADD COLUMN IF NOT EXISTS "safety_factor" DECIMAL(18,4) NOT NULL DEFAULT 0;
ALTER TABLE "material_shortage_alerts" ADD COLUMN IF NOT EXISTS "severe_threshold" DECIMAL(18,4) NOT NULL DEFAULT 0;
ALTER TABLE "outsourcing_order_item" ADD COLUMN IF NOT EXISTS "freight_fee" DECIMAL(18,4) NOT NULL DEFAULT 0;
ALTER TABLE "outsourcing_order_item" ADD COLUMN IF NOT EXISTS "processing_fee" DECIMAL(18,4) NOT NULL DEFAULT 0;
ALTER TABLE "outsourcing_receipt" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "outsourcing_receipt" ADD COLUMN IF NOT EXISTS "is_deleted" BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE "outsourcing_voucher" ADD COLUMN IF NOT EXISTS "amount" DECIMAL(18,4) NOT NULL DEFAULT 0;
ALTER TABLE "outsourcing_voucher" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW();
ALTER TABLE "outsourcing_voucher" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "outsourcing_voucher" ADD COLUMN IF NOT EXISTS "credit_account" VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE "outsourcing_voucher" ADD COLUMN IF NOT EXISTS "debit_account" VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE "outsourcing_voucher" ADD COLUMN IF NOT EXISTS "is_posted" BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE "outsourcing_voucher" ADD COLUMN IF NOT EXISTS "outsourcing_order_id" INTEGER NOT NULL DEFAULT 0;
ALTER TABLE "outsourcing_voucher" ADD COLUMN IF NOT EXISTS "posted_at" TIMESTAMPTZ;
ALTER TABLE "outsourcing_voucher" ADD COLUMN IF NOT EXISTS "remarks" VARCHAR(255);
ALTER TABLE "outsourcing_voucher" ADD COLUMN IF NOT EXISTS "tax_amount" DECIMAL(18,4) NOT NULL DEFAULT 0;
ALTER TABLE "outsourcing_voucher" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW();
ALTER TABLE "outsourcing_voucher" ADD COLUMN IF NOT EXISTS "voucher_date" DATE NOT NULL DEFAULT '';
ALTER TABLE "outsourcing_voucher" ADD COLUMN IF NOT EXISTS "voucher_no" VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE "outsourcing_voucher" ADD COLUMN IF NOT EXISTS "voucher_type" VARCHAR(255) NOT NULL DEFAULT '';
ALTER TABLE "wage_record_detail" ADD COLUMN IF NOT EXISTS "grade_ratio" DECIMAL(18,4) NOT NULL DEFAULT 0;
ALTER TABLE "wage_record_detail" ADD COLUMN IF NOT EXISTS "id_card_no" VARCHAR(255);
ALTER TABLE "wage_record_detail" ADD COLUMN IF NOT EXISTS "individual_income_tax" DECIMAL(18,4);
ALTER TABLE "wage_record_detail" ADD COLUMN IF NOT EXISTS "piece_wage" DECIMAL(18,4) NOT NULL DEFAULT 0;
ALTER TABLE "wage_record_detail" ADD COLUMN IF NOT EXISTS "remarks" VARCHAR(255);
ALTER TABLE "wage_record_detail" ADD COLUMN IF NOT EXISTS "time_wage" DECIMAL(18,4) NOT NULL DEFAULT 0;
ALTER TABLE "wage_record_detail" ADD COLUMN IF NOT EXISTS "wage_amount" DECIMAL(18,4) NOT NULL DEFAULT 0;
CREATE TABLE IF NOT EXISTS "aging_alert_rules" (
    "id" SERIAL PRIMARY KEY,
    "rule_name" VARCHAR(255) NOT NULL,
    "rule_code" VARCHAR(255) NOT NULL,
    "aging_bucket" VARCHAR(255) NOT NULL,
    "threshold_days" INTEGER NOT NULL,
    "threshold_amount" DECIMAL(15,2),
    "alert_level" VARCHAR(255) NOT NULL,
    "notify_method" VARCHAR(255) NOT NULL,
    "notify_roles" JSONB,
    "is_active" BOOLEAN NOT NULL,
    "remarks" VARCHAR(255),
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "aging_grade_configs" (
    "id" BIGSERIAL PRIMARY KEY,
    "grade_name" VARCHAR(255) NOT NULL,
    "min_days" INTEGER NOT NULL,
    "max_days" INTEGER NOT NULL,
    "sort_order" INTEGER NOT NULL,
    "is_active" BOOLEAN NOT NULL,
    "remark" VARCHAR(255),
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "ai_decision_logs" (
    "id" BIGSERIAL PRIMARY KEY,
    "decision_type" VARCHAR(255) NOT NULL,
    "model_version_id" INTEGER,
    "input_json" JSONB,
    "output_json" JSONB,
    "user_id" INTEGER,
    "ip_address" VARCHAR(255),
    "latency_ms" INTEGER,
    "confidence" DECIMAL(18,4),
    "source" VARCHAR(255),
    "degraded" BOOLEAN NOT NULL,
    "sensitivity_level" VARCHAR(255) DEFAULT 'low',
    "operation_category" VARCHAR(255) DEFAULT 'inference',
    "created_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "ai_model_evaluations" (
    "id" SERIAL PRIMARY KEY,
    "model_version_id" INTEGER NOT NULL,
    "evaluation_date" TIMESTAMPTZ NOT NULL,
    "accuracy" DECIMAL(18,4),
    "precision" DECIMAL(18,4),
    "recall" DECIMAL(18,4),
    "f1_score" DECIMAL(18,4),
    "sample_count" INTEGER NOT NULL,
    "evaluation_report" VARCHAR(255),
    "created_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "ai_model_versions" (
    "id" SERIAL PRIMARY KEY,
    "model_name" VARCHAR(255) NOT NULL,
    "version" VARCHAR(255) NOT NULL,
    "algorithm" VARCHAR(255) NOT NULL,
    "parameters_json" JSONB,
    "training_date" DATE,
    "training_dataset_size" INTEGER,
    "accuracy_metrics_json" JSONB,
    "status" VARCHAR(255) NOT NULL,
    "changed_by" INTEGER,
    "change_reason" VARCHAR(255),
    "approval_status" VARCHAR(255) NOT NULL,
    "approved_by" INTEGER,
    "approved_at" TIMESTAMPTZ,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "ai_quality_accuracy_reports" (
    "id" SERIAL PRIMARY KEY,
    "report_period" VARCHAR(255) NOT NULL,
    "total_predictions" INTEGER NOT NULL,
    "correct_predictions" INTEGER NOT NULL,
    "accuracy_rate" DECIMAL(18,4),
    "precision_score" DECIMAL(18,4),
    "recall_score" DECIMAL(18,4),
    "f1_score" DECIMAL(18,4),
    "mismatch_cases_json" JSONB,
    "generated_at" TIMESTAMPTZ NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "asset_categories" (
    "id" SERIAL PRIMARY KEY,
    "category_code" VARCHAR(255) NOT NULL,
    "category_name" VARCHAR(255) NOT NULL,
    "parent_id" INTEGER,
    "default_useful_life" INTEGER,
    "default_depreciation_method" VARCHAR(255),
    "default_salvage_rate" DECIMAL(5,4),
    "description" VARCHAR(255),
    "is_active" BOOLEAN NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "asset_impairment_tests" (
    "id" SERIAL PRIMARY KEY,
    "asset_id" INTEGER NOT NULL,
    "test_date" DATE NOT NULL,
    "carrying_amount" DECIMAL(14,2) NOT NULL,
    "recoverable_amount" DECIMAL(14,2) NOT NULL,
    "impairment_loss" DECIMAL(14,2) NOT NULL,
    "test_basis" VARCHAR(255) NOT NULL,
    "notes" VARCHAR(255),
    "status" VARCHAR(255) NOT NULL,
    "reviewed_by" INTEGER,
    "reviewed_at" TIMESTAMPTZ,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "budget_versions" (
    "id" SERIAL PRIMARY KEY,
    "plan_id" INTEGER NOT NULL,
    "version_no" VARCHAR(255) NOT NULL,
    "version_name" VARCHAR(255) NOT NULL,
    "total_amount" DECIMAL(14,2) NOT NULL,
    "status" VARCHAR(255) NOT NULL,
    "change_reason" VARCHAR(255),
    "approved_by" INTEGER,
    "approved_at" TIMESTAMPTZ,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "business_mode_config" (
    "id" SERIAL PRIMARY KEY,
    "mode_code" VARCHAR(255) NOT NULL,
    "mode_name" VARCHAR(255) NOT NULL,
    "description" VARCHAR(255),
    "is_active" BOOLEAN NOT NULL,
    "is_default" BOOLEAN NOT NULL,
    "process_chain" JSONB NOT NULL,
    "material_source" VARCHAR(255) NOT NULL,
    "settlement_method" VARCHAR(255) NOT NULL,
    "inventory_type" VARCHAR(255) NOT NULL,
    "cost_method" VARCHAR(255) NOT NULL,
    "require_purchase" BOOLEAN NOT NULL,
    "require_production" BOOLEAN NOT NULL,
    "require_outsourcing" BOOLEAN NOT NULL,
    "require_sales" BOOLEAN NOT NULL,
    "mode_category" VARCHAR(255) NOT NULL,
    "remarks" VARCHAR(255),
    "is_deleted" BOOLEAN NOT NULL,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "business_mode_flow_step" (
    "id" SERIAL PRIMARY KEY,
    "mode_id" INTEGER NOT NULL,
    "step_no" INTEGER NOT NULL,
    "step_code" VARCHAR(255) NOT NULL,
    "step_name" VARCHAR(255) NOT NULL,
    "module_name" VARCHAR(255) NOT NULL,
    "is_required" BOOLEAN NOT NULL,
    "description" VARCHAR(255),
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "business_mode_order_link" (
    "id" SERIAL PRIMARY KEY,
    "mode_id" INTEGER NOT NULL,
    "document_type" VARCHAR(255) NOT NULL,
    "document_id" INTEGER NOT NULL,
    "document_no" VARCHAR(255) NOT NULL,
    "mode_snapshot" JSONB,
    "remarks" VARCHAR(255),
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "business_mode_rule" (
    "id" SERIAL PRIMARY KEY,
    "mode_id" INTEGER NOT NULL,
    "rule_code" VARCHAR(255) NOT NULL,
    "rule_name" VARCHAR(255) NOT NULL,
    "rule_type" VARCHAR(255) NOT NULL,
    "module_name" VARCHAR(255) NOT NULL,
    "validation_logic" JSONB,
    "description" VARCHAR(255),
    "is_active" BOOLEAN NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "certificate_of_origin" (
    "id" SERIAL PRIMARY KEY,
    "certificate_no" VARCHAR(255) NOT NULL,
    "inspection_id" INTEGER,
    "product_name" VARCHAR(255) NOT NULL,
    "hs_code" VARCHAR(255) NOT NULL,
    "origin_country" VARCHAR(255) NOT NULL,
    "destination_country" VARCHAR(255) NOT NULL,
    "quantity" DECIMAL(15,2) NOT NULL,
    "unit" VARCHAR(255) NOT NULL,
    "invoice_amount" DECIMAL(15,2),
    "certificate_type" VARCHAR(255) NOT NULL,
    "issue_date" DATE NOT NULL,
    "expiry_date" DATE,
    "status" VARCHAR(255) NOT NULL,
    "remarks" VARCHAR(255),
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "chemical_category" (
    "id" SERIAL PRIMARY KEY,
    "category_code" VARCHAR(255) NOT NULL,
    "category_name" VARCHAR(255) NOT NULL,
    "parent_id" INTEGER,
    "category_type" VARCHAR(255) NOT NULL,
    "description" VARCHAR(255),
    "sort_order" INTEGER NOT NULL,
    "is_active" BOOLEAN NOT NULL,
    "is_deleted" BOOLEAN NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "chemical_lot" (
    "id" SERIAL PRIMARY KEY,
    "lot_no" VARCHAR(255) NOT NULL,
    "chemical_id" INTEGER NOT NULL,
    "supplier_id" INTEGER,
    "supplier_lot_no" VARCHAR(255),
    "production_date" DATE,
    "expiry_date" DATE,
    "received_date" DATE,
    "quantity_received" DECIMAL(14,4) NOT NULL,
    "quantity_available" DECIMAL(14,4) NOT NULL,
    "quantity_reserved" DECIMAL(14,4) NOT NULL,
    "inspection_status" VARCHAR(255) NOT NULL,
    "inspection_report_url" VARCHAR(255),
    "unit_cost" DECIMAL(14,4) NOT NULL,
    "total_cost" DECIMAL(14,4) NOT NULL,
    "warehouse_id" INTEGER,
    "storage_zone" VARCHAR(255),
    "status" VARCHAR(255) NOT NULL,
    "remarks" VARCHAR(255),
    "is_deleted" BOOLEAN NOT NULL,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "chemical_master" (
    "id" SERIAL PRIMARY KEY,
    "chemical_code" VARCHAR(255) NOT NULL,
    "chemical_name" VARCHAR(255) NOT NULL,
    "chemical_name_en" VARCHAR(255),
    "chemical_type" VARCHAR(255) NOT NULL,
    "category_id" INTEGER,
    "dye_category" VARCHAR(255),
    "color_index" VARCHAR(255),
    "auxiliary_category" VARCHAR(255),
    "cas_number" VARCHAR(255),
    "molecular_formula" VARCHAR(255),
    "molecular_weight" DECIMAL(14,4),
    "specification" VARCHAR(255),
    "unit" VARCHAR(255) NOT NULL,
    "standard_price" DECIMAL(14,4) NOT NULL,
    "cost_price" DECIMAL(14,4) NOT NULL,
    "ghs_classification" VARCHAR(255),
    "un_number" VARCHAR(255),
    "hazard_class" VARCHAR(255),
    "hazard_pictogram" VARCHAR(255),
    "signal_word" VARCHAR(255),
    "msds_url" VARCHAR(255),
    "msds_version" VARCHAR(255),
    "msds_updated_at" TIMESTAMPTZ,
    "shelf_life_days" INTEGER,
    "storage_condition" VARCHAR(255),
    "storage_temperature" VARCHAR(255),
    "safety_stock" DECIMAL(14,4) NOT NULL,
    "reorder_point" DECIMAL(14,4) NOT NULL,
    "reorder_quantity" DECIMAL(14,4) NOT NULL,
    "package_unit" VARCHAR(255),
    "package_capacity" DECIMAL(14,4),
    "packages_per_pallet" INTEGER,
    "supplier_id" INTEGER,
    "supplier_product_code" VARCHAR(255),
    "fastness_light" VARCHAR(255),
    "fastness_washing" VARCHAR(255),
    "active_ingredient" VARCHAR(255),
    "concentration" DECIMAL(8,4),
    "status" VARCHAR(255) NOT NULL,
    "remarks" VARCHAR(255),
    "is_deleted" BOOLEAN NOT NULL,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "chemical_requisition" (
    "id" SERIAL PRIMARY KEY,
    "requisition_no" VARCHAR(255) NOT NULL,
    "requisition_type" VARCHAR(255) NOT NULL,
    "department_id" INTEGER,
    "requisition_date" DATE NOT NULL,
    "required_date" DATE,
    "dye_batch_id" INTEGER,
    "production_order_id" INTEGER,
    "status" VARCHAR(255) NOT NULL,
    "total_amount" DECIMAL(14,4) NOT NULL,
    "remarks" VARCHAR(255),
    "is_deleted" BOOLEAN NOT NULL,
    "created_by" INTEGER,
    "approved_by" INTEGER,
    "issued_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "competitor" (
    "id" SERIAL PRIMARY KEY,
    "name" VARCHAR(255) NOT NULL,
    "strengths" VARCHAR(255),
    "weaknesses" VARCHAR(255),
    "website" VARCHAR(255),
    "notes" VARCHAR(255),
    "created_at" TIMESTAMPTZ,
    "updated_at" TIMESTAMPTZ
);


CREATE TABLE IF NOT EXISTS "customer_addresses" (
    "id" BIGSERIAL PRIMARY KEY,
    "customer_id" INTEGER NOT NULL,
    "contact_name" VARCHAR(255) NOT NULL,
    "contact_phone" VARCHAR(255) NOT NULL,
    "province" VARCHAR(255),
    "city" VARCHAR(255),
    "district" VARCHAR(255),
    "address" VARCHAR(255) NOT NULL,
    "postal_code" VARCHAR(255),
    "is_default" BOOLEAN NOT NULL,
    "remark" VARCHAR(255),
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "customer_audit_log" (
    "id" SERIAL PRIMARY KEY,
    "customer_id" INTEGER NOT NULL,
    "operation" VARCHAR(255) NOT NULL,
    "field_name" VARCHAR(255),
    "old_value" VARCHAR(255),
    "new_value" VARCHAR(255),
    "user_id" INTEGER NOT NULL,
    "user_name" VARCHAR(255) NOT NULL,
    "ip_address" VARCHAR(255),
    "user_agent" VARCHAR(255),
    "created_at" TIMESTAMPTZ
);


CREATE TABLE IF NOT EXISTS "customer_field_permission" (
    "id" SERIAL PRIMARY KEY,
    "role_id" INTEGER NOT NULL,
    "field_name" VARCHAR(255) NOT NULL,
    "permission" VARCHAR(255) NOT NULL,
    "mask_pattern" VARCHAR(255),
    "created_at" TIMESTAMPTZ,
    "updated_at" TIMESTAMPTZ
);


CREATE TABLE IF NOT EXISTS "customer_followup" (
    "id" TEXT PRIMARY KEY,
    "customer_id" INTEGER NOT NULL,
    "follow_up_type" VARCHAR(255) NOT NULL,
    "content" VARCHAR(255) NOT NULL,
    "follow_up_at" TIMESTAMPTZ NOT NULL,
    "next_follow_up_at" TIMESTAMPTZ,
    "notes" VARCHAR(255),
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "customer_lifetime_value" (
    "id" SERIAL PRIMARY KEY,
    "customer_id" INTEGER NOT NULL,
    "total_orders" INTEGER NOT NULL,
    "total_revenue" DECIMAL(18,4) NOT NULL,
    "avg_order_value" DECIMAL(18,4) NOT NULL,
    "first_order_date" DATE,
    "last_order_date" DATE,
    "customer_lifespan_days" INTEGER NOT NULL,
    "purchase_frequency" DECIMAL(18,4) NOT NULL,
    "clv_score" DECIMAL(18,4) NOT NULL,
    "segment" VARCHAR(255),
    "calculated_at" TIMESTAMPTZ
);


CREATE TABLE IF NOT EXISTS "customer_pool_rules" (
    "id" SERIAL PRIMARY KEY,
    "name" VARCHAR(255) NOT NULL,
    "rule_type" VARCHAR(255) NOT NULL,
    "rule_value" INTEGER NOT NULL,
    "customer_type" VARCHAR(255) NOT NULL,
    "is_enabled" BOOLEAN NOT NULL,
    "notes" VARCHAR(255),
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "customer_transfer_approvals" (
    "id" SERIAL PRIMARY KEY,
    "approval_no" VARCHAR(255) NOT NULL,
    "lead_id" INTEGER NOT NULL,
    "company_name" VARCHAR(255),
    "from_user_id" INTEGER NOT NULL,
    "from_user_name" VARCHAR(255),
    "to_user_id" INTEGER NOT NULL,
    "to_user_name" VARCHAR(255),
    "applicant_id" INTEGER NOT NULL,
    "reason" VARCHAR(255) NOT NULL,
    "is_large_customer" BOOLEAN NOT NULL,
    "approval_status" VARCHAR(255) NOT NULL,
    "current_level" INTEGER NOT NULL,
    "max_level" INTEGER NOT NULL,
    "manager_approver_id" INTEGER,
    "manager_comment" VARCHAR(255),
    "manager_approved_at" TIMESTAMPTZ,
    "director_approver_id" INTEGER,
    "director_comment" VARCHAR(255),
    "director_approved_at" TIMESTAMPTZ,
    "completed_at" TIMESTAMPTZ,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "depreciation_policy_changes" (
    "id" SERIAL PRIMARY KEY,
    "asset_id" INTEGER NOT NULL,
    "change_date" DATE NOT NULL,
    "old_method" VARCHAR(255) NOT NULL,
    "new_method" VARCHAR(255) NOT NULL,
    "old_useful_life" INTEGER,
    "new_useful_life" INTEGER,
    "old_salvage_rate" DECIMAL(5,4),
    "new_salvage_rate" DECIMAL(5,4),
    "reason" VARCHAR(255) NOT NULL,
    "approved_by" INTEGER,
    "approved_at" TIMESTAMPTZ,
    "status" VARCHAR(255) NOT NULL,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "dye_batch_lifecycle_log" (
    "id" SERIAL PRIMARY KEY,
    "batch_id" INTEGER NOT NULL,
    "batch_no" VARCHAR(255) NOT NULL,
    "from_status" VARCHAR(255),
    "to_status" VARCHAR(255) NOT NULL,
    "transition_code" VARCHAR(255) NOT NULL,
    "transition_name" VARCHAR(255) NOT NULL,
    "operator_id" INTEGER,
    "operator_name" VARCHAR(255),
    "equipment_id" INTEGER,
    "equipment_name" VARCHAR(255),
    "work_shift" VARCHAR(255),
    "captured_params" JSONB,
    "remarks" VARCHAR(255),
    "transition_at" TIMESTAMPTZ NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "dye_batch_operation" (
    "id" SERIAL PRIMARY KEY,
    "operation_type" VARCHAR(255) NOT NULL,
    "operation_name" VARCHAR(255) NOT NULL,
    "target_batch_id" INTEGER NOT NULL,
    "target_batch_no" VARCHAR(255) NOT NULL,
    "source_batch_ids" JSONB,
    "source_batch_nos" JSONB,
    "operation_data" JSONB,
    "operator_id" INTEGER,
    "operator_name" VARCHAR(255),
    "operation_at" TIMESTAMPTZ NOT NULL,
    "remarks" VARCHAR(255),
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "energy_allocation_record" (
    "id" SERIAL PRIMARY KEY,
    "allocation_no" VARCHAR(255) NOT NULL,
    "period_start" TIMESTAMPTZ NOT NULL,
    "period_end" TIMESTAMPTZ NOT NULL,
    "meter_type" VARCHAR(255) NOT NULL,
    "workshop" VARCHAR(255),
    "allocation_rule_id" INTEGER,
    "allocation_basis" VARCHAR(255) NOT NULL,
    "total_consumption" DECIMAL(14,2) NOT NULL,
    "total_cost" DECIMAL(14,2) NOT NULL,
    "dye_lot_no" VARCHAR(255),
    "production_order_id" INTEGER,
    "production_order_no" VARCHAR(255),
    "process_route_id" INTEGER,
    "route_code" VARCHAR(255),
    "flow_card_id" INTEGER,
    "allocation_basis_value" DECIMAL(14,2) NOT NULL,
    "allocation_ratio" DECIMAL(8,4) NOT NULL,
    "allocated_consumption" DECIMAL(14,2) NOT NULL,
    "allocated_cost" DECIMAL(14,2) NOT NULL,
    "output_quantity" DECIMAL(14,2),
    "unit_consumption" DECIMAL(14,4),
    "cost_collection_id" INTEGER,
    "status" VARCHAR(255) NOT NULL,
    "confirmed_by" INTEGER,
    "confirmed_at" TIMESTAMPTZ,
    "remarks" VARCHAR(255),
    "is_deleted" BOOLEAN NOT NULL,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "energy_allocation_rule" (
    "id" SERIAL PRIMARY KEY,
    "rule_no" VARCHAR(255) NOT NULL,
    "rule_name" VARCHAR(255) NOT NULL,
    "meter_type" VARCHAR(255) NOT NULL,
    "allocation_basis" VARCHAR(255) NOT NULL,
    "workshop" VARCHAR(255),
    "process_route_id" INTEGER,
    "route_code" VARCHAR(255),
    "effective_date" DATE NOT NULL,
    "expiry_date" DATE,
    "standard_consumption_per_unit" DECIMAL(14,4) NOT NULL,
    "standard_unit" VARCHAR(255),
    "status" VARCHAR(255) NOT NULL,
    "remarks" VARCHAR(255),
    "is_deleted" BOOLEAN NOT NULL,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "energy_consumption_record" (
    "id" SERIAL PRIMARY KEY,
    "record_no" VARCHAR(255) NOT NULL,
    "meter_id" INTEGER,
    "meter_type" VARCHAR(255) NOT NULL,
    "workshop" VARCHAR(255),
    "unit" VARCHAR(255) NOT NULL,
    "previous_reading" DECIMAL(14,2) NOT NULL,
    "current_reading" DECIMAL(14,2) NOT NULL,
    "consumption" DECIMAL(14,2) NOT NULL,
    "unit_price" DECIMAL(12,4) NOT NULL,
    "total_cost" DECIMAL(14,2) NOT NULL,
    "period_start" TIMESTAMPTZ NOT NULL,
    "period_end" TIMESTAMPTZ NOT NULL,
    "recording_method" VARCHAR(255) NOT NULL,
    "dye_lot_no" VARCHAR(255),
    "process_route_id" INTEGER,
    "route_code" VARCHAR(255),
    "equipment_id" INTEGER,
    "equipment_name" VARCHAR(255),
    "operator_id" INTEGER,
    "recorded_at" TIMESTAMPTZ NOT NULL,
    "status" VARCHAR(255) NOT NULL,
    "remarks" VARCHAR(255),
    "is_deleted" BOOLEAN NOT NULL,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "energy_meter" (
    "id" SERIAL PRIMARY KEY,
    "meter_no" VARCHAR(255) NOT NULL,
    "meter_name" VARCHAR(255) NOT NULL,
    "meter_type" VARCHAR(255) NOT NULL,
    "workshop" VARCHAR(255),
    "equipment_id" INTEGER,
    "equipment_name" VARCHAR(255),
    "location" VARCHAR(255),
    "iot_device_id" VARCHAR(255),
    "unit" VARCHAR(255) NOT NULL,
    "current_reading" DECIMAL(14,2) NOT NULL,
    "previous_reading" DECIMAL(14,2) NOT NULL,
    "last_reading_at" TIMESTAMPTZ,
    "unit_price" DECIMAL(12,4) NOT NULL,
    "status" VARCHAR(255) NOT NULL,
    "remarks" VARCHAR(255),
    "is_deleted" BOOLEAN NOT NULL,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "environmental_assessment" (
    "id" SERIAL PRIMARY KEY,
    "doc_type" VARCHAR(255) NOT NULL,
    "doc_name" VARCHAR(255) NOT NULL,
    "doc_url" VARCHAR(255) NOT NULL,
    "approval_date" DATE,
    "approval_authority" VARCHAR(255),
    "remarks" VARCHAR(255),
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "export_inspection" (
    "id" SERIAL PRIMARY KEY,
    "inspection_no" VARCHAR(255) NOT NULL,
    "sales_order_id" INTEGER NOT NULL,
    "delivery_id" INTEGER,
    "product_name" VARCHAR(255) NOT NULL,
    "hs_code" VARCHAR(255) NOT NULL,
    "inspection_type" VARCHAR(255) NOT NULL,
    "inspection_agency" VARCHAR(255) NOT NULL,
    "inspection_date" DATE NOT NULL,
    "result" VARCHAR(255) NOT NULL,
    "report_url" VARCHAR(255),
    "certificate_no" VARCHAR(255),
    "certificate_expiry" DATE,
    "remarks" VARCHAR(255),
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "fabric_defect_record" (
    "id" SERIAL PRIMARY KEY,
    "inspection_id" INTEGER NOT NULL,
    "defect_type" VARCHAR(255) NOT NULL,
    "position_yards" DECIMAL(10,2) NOT NULL,
    "defect_length_inches" DECIMAL(8,2) NOT NULL,
    "direction" VARCHAR(255) NOT NULL,
    "is_hole" BOOLEAN NOT NULL,
    "is_continuous" BOOLEAN NOT NULL,
    "is_half_width" BOOLEAN NOT NULL,
    "points" INTEGER NOT NULL,
    "description" VARCHAR(255),
    "created_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "female_worker_protection" (
    "id" SERIAL PRIMARY KEY,
    "worker_id" INTEGER NOT NULL,
    "protection_type" VARCHAR(255) NOT NULL,
    "expected_start_date" DATE,
    "expected_end_date" DATE,
    "actual_start_date" DATE,
    "actual_end_date" DATE,
    "status" VARCHAR(255) NOT NULL,
    "remarks" VARCHAR(255),
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "finance_invoices" (
    "id" SERIAL PRIMARY KEY,
    "invoice_no" VARCHAR(255) NOT NULL,
    "order_id" INTEGER,
    "invoice_date" TIMESTAMPTZ NOT NULL,
    "amount" DECIMAL(18,4) NOT NULL,
    "tax_amount" DECIMAL(18,4) NOT NULL,
    "total_amount" DECIMAL(18,4) NOT NULL,
    "status" VARCHAR(255) NOT NULL,
    "paid_date" TIMESTAMPTZ,
    "payment_method" VARCHAR(255),
    "notes" VARCHAR(255),
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "industry_benchmark_configs" (
    "id" BIGSERIAL PRIMARY KEY,
    "benchmark_name" VARCHAR(255) NOT NULL,
    "industry_type" VARCHAR(255) NOT NULL,
    "metric_name" VARCHAR(255) NOT NULL,
    "metric_value" DECIMAL(14,4) NOT NULL,
    "unit" VARCHAR(255),
    "data_source" VARCHAR(255),
    "data_year" INTEGER,
    "is_active" BOOLEAN NOT NULL,
    "remark" VARCHAR(255),
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "inventory_write_down" (
    "id" SERIAL PRIMARY KEY,
    "product_id" INTEGER NOT NULL,
    "write_down_type" VARCHAR(255) NOT NULL,
    "original_cost" DECIMAL(15,2) NOT NULL,
    "net_realizable_value" DECIMAL(15,2) NOT NULL,
    "write_down_amount" DECIMAL(15,2) NOT NULL,
    "reason" VARCHAR(255),
    "period" DATE NOT NULL,
    "status" VARCHAR(255) NOT NULL,
    "created_by" INTEGER NOT NULL,
    "confirmed_by" INTEGER,
    "confirmed_at" TIMESTAMPTZ,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "lab_dip_request" (
    "id" SERIAL PRIMARY KEY,
    "request_no" VARCHAR(255) NOT NULL,
    "customer_id" INTEGER,
    "customer_color_no" VARCHAR(255),
    "customer_color_name" VARCHAR(255),
    "sample_type" VARCHAR(255),
    "fabric_spec" VARCHAR(255),
    "fabric_component" VARCHAR(255),
    "sample_size" VARCHAR(255),
    "light_source" VARCHAR(255) NOT NULL,
    "secondary_light_source" VARCHAR(255),
    "color_fastness_req" VARCHAR(255),
    "eco_requirement" VARCHAR(255),
    "sample_versions" INTEGER NOT NULL,
    "dye_category" VARCHAR(255),
    "required_date" DATE NOT NULL,
    "expected_days" INTEGER,
    "status" VARCHAR(255) NOT NULL,
    "customer_approved_at" TIMESTAMPTZ,
    "customer_approval_comment" VARCHAR(255),
    "approved_sample_id" INTEGER,
    "production_recipe_id" INTEGER,
    "remarks" VARCHAR(255),
    "is_deleted" BOOLEAN NOT NULL,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "lab_dip_resample" (
    "id" SERIAL PRIMARY KEY,
    "request_id" INTEGER NOT NULL,
    "source_sample_id" INTEGER NOT NULL,
    "resample_no" VARCHAR(255) NOT NULL,
    "workshop_fabric_batch" VARCHAR(255),
    "dye_batch_no" VARCHAR(255),
    "auxiliary_batch_no" VARCHAR(255),
    "production_plan_id" INTEGER,
    "adjusted_formula" VARCHAR(255),
    "adjustment_factor" DECIMAL(5,2),
    "adjusted_temperature" DECIMAL(5,2),
    "adjusted_time_minutes" INTEGER,
    "adjusted_liquor_ratio" VARCHAR(255),
    "color_difference_grade" INTEGER,
    "color_difference_value" DECIMAL(5,2),
    "result" VARCHAR(255) NOT NULL,
    "reviewed_by" INTEGER,
    "reviewed_at" TIMESTAMPTZ,
    "review_comment" VARCHAR(255),
    "production_recipe_id" INTEGER,
    "tech_card_no" VARCHAR(255),
    "tech_card_issued_by" INTEGER,
    "tech_card_issued_at" TIMESTAMPTZ,
    "remarks" VARCHAR(255),
    "is_deleted" BOOLEAN NOT NULL,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "lab_dip_sample" (
    "id" SERIAL PRIMARY KEY,
    "request_id" INTEGER NOT NULL,
    "version_label" VARCHAR(255) NOT NULL,
    "version_seq" INTEGER NOT NULL,
    "recipe_no" VARCHAR(255),
    "dye_recipe_id" INTEGER,
    "formula" VARCHAR(255),
    "formula_detail" JSONB,
    "temperature" DECIMAL(5,2),
    "time_minutes" INTEGER,
    "liquor_ratio" VARCHAR(255),
    "ph_value" DECIMAL(5,2),
    "dyeing_method" VARCHAR(255),
    "dye_cost" DECIMAL(10,4),
    "auxiliary_cost" DECIMAL(10,4),
    "total_cost" DECIMAL(10,4),
    "color_difference_grade" INTEGER,
    "color_difference_value" DECIMAL(5,2),
    "matching_result" VARCHAR(255) NOT NULL,
    "approved_by" INTEGER,
    "approved_at" TIMESTAMPTZ,
    "approval_comment" VARCHAR(255),
    "resample_status" VARCHAR(255),
    "resample_recipe_id" INTEGER,
    "remarks" VARCHAR(255),
    "is_deleted" BOOLEAN NOT NULL,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "lead_allocation_rule" (
    "id" SERIAL PRIMARY KEY,
    "rule_name" VARCHAR(255) NOT NULL,
    "rule_type" VARCHAR(255) NOT NULL,
    "source_filter" VARCHAR(255),
    "industry_filter" VARCHAR(255),
    "region_filter" VARCHAR(255),
    "assigned_user_ids" JSONB,
    "weights" JSONB,
    "daily_limit" INTEGER NOT NULL,
    "priority" INTEGER NOT NULL,
    "is_active" BOOLEAN NOT NULL,
    "created_at" TIMESTAMPTZ,
    "updated_at" TIMESTAMPTZ
);


CREATE TABLE IF NOT EXISTS "lead_nurture_plan" (
    "id" SERIAL PRIMARY KEY,
    "lead_id" INTEGER NOT NULL,
    "plan_name" VARCHAR(255) NOT NULL,
    "nurture_type" VARCHAR(255) NOT NULL,
    "trigger_condition" VARCHAR(255),
    "template_id" VARCHAR(255),
    "scheduled_at" TIMESTAMPTZ,
    "executed_at" TIMESTAMPTZ,
    "status" VARCHAR(255),
    "result" VARCHAR(255),
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ
);


CREATE TABLE IF NOT EXISTS "lead_source_roi" (
    "id" SERIAL PRIMARY KEY,
    "source" VARCHAR(255) NOT NULL,
    "period_start" DATE NOT NULL,
    "period_end" DATE NOT NULL,
    "cost" DECIMAL(18,4) NOT NULL,
    "lead_count" INTEGER NOT NULL,
    "converted_count" INTEGER NOT NULL,
    "opportunity_count" INTEGER NOT NULL,
    "order_count" INTEGER NOT NULL,
    "revenue" DECIMAL(18,4) NOT NULL,
    "conversion_rate" DECIMAL(18,4) NOT NULL,
    "roi" DECIMAL(18,4) NOT NULL,
    "created_at" TIMESTAMPTZ
);


CREATE TABLE IF NOT EXISTS "long_running_tasks" (
    "id" BIGSERIAL PRIMARY KEY,
    "task_type" VARCHAR(255) NOT NULL,
    "status" VARCHAR(255) NOT NULL,
    "params" JSONB,
    "progress" INTEGER NOT NULL,
    "result" JSONB,
    "error_message" VARCHAR(255),
    "started_at" TIMESTAMPTZ,
    "completed_at" TIMESTAMPTZ,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "notification_subscriptions" (
    "id" SERIAL PRIMARY KEY,
    "user_id" INTEGER NOT NULL,
    "name" VARCHAR(255) NOT NULL,
    "business_type" VARCHAR(255) NOT NULL,
    "channel" VARCHAR(255) NOT NULL,
    "is_enabled" BOOLEAN NOT NULL,
    "next_run_at" TIMESTAMPTZ,
    "last_run_at" TIMESTAMPTZ,
    "last_run_status" VARCHAR(255),
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "notification_templates" (
    "id" SERIAL PRIMARY KEY,
    "code" VARCHAR(255) NOT NULL,
    "name" VARCHAR(255) NOT NULL,
    "template_type" VARCHAR(255) NOT NULL,
    "title_template" VARCHAR(255) NOT NULL,
    "content_template" VARCHAR(255) NOT NULL,
    "language" VARCHAR(255) NOT NULL,
    "is_active" BOOLEAN NOT NULL,
    "remarks" VARCHAR(255),
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "operation_certificate" (
    "id" SERIAL PRIMARY KEY,
    "worker_id" INTEGER NOT NULL,
    "certificate_no" VARCHAR(255) NOT NULL,
    "certificate_type" VARCHAR(255) NOT NULL,
    "equipment_name" VARCHAR(255),
    "issue_date" DATE NOT NULL,
    "expiry_date" DATE NOT NULL,
    "issuing_authority" VARCHAR(255),
    "status" VARCHAR(255) NOT NULL,
    "remarks" VARCHAR(255),
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "opportunity_competitor" (
    "id" SERIAL PRIMARY KEY,
    "opportunity_id" INTEGER NOT NULL,
    "competitor_id" INTEGER NOT NULL,
    "threat_level" VARCHAR(255),
    "notes" VARCHAR(255),
    "created_at" TIMESTAMPTZ
);


CREATE TABLE IF NOT EXISTS "opportunity_follow_up" (
    "id" SERIAL PRIMARY KEY,
    "opportunity_id" INTEGER NOT NULL,
    "follow_up_type" VARCHAR(255) NOT NULL,
    "content" VARCHAR(255) NOT NULL,
    "follow_up_time" TIMESTAMPTZ NOT NULL,
    "next_follow_up_date" DATE,
    "user_id" INTEGER NOT NULL,
    "user_name" VARCHAR(255) NOT NULL,
    "created_at" TIMESTAMPTZ
);


CREATE TABLE IF NOT EXISTS "opportunity_stage_history" (
    "id" SERIAL PRIMARY KEY,
    "opportunity_id" INTEGER NOT NULL,
    "from_stage" VARCHAR(255),
    "to_stage" VARCHAR(255) NOT NULL,
    "changed_at" TIMESTAMPTZ NOT NULL,
    "changed_by" INTEGER,
    "duration_days" INTEGER
);


CREATE TABLE IF NOT EXISTS "outsourcing_order" (
    "id" SERIAL PRIMARY KEY,
    "order_no" VARCHAR(255) NOT NULL,
    "order_type" VARCHAR(255) NOT NULL,
    "supplier_id" INTEGER NOT NULL,
    "production_order_id" INTEGER,
    "dye_batch_id" INTEGER,
    "color_no" VARCHAR(255),
    "dye_lot_no" VARCHAR(255),
    "issue_date" DATE NOT NULL,
    "expected_return_date" DATE,
    "actual_return_date" DATE,
    "issue_quantity" DECIMAL(14,4) NOT NULL,
    "issue_unit" VARCHAR(255) NOT NULL,
    "return_quantity" DECIMAL(14,4) NOT NULL,
    "loss_quantity" DECIMAL(14,4) NOT NULL,
    "loss_type" VARCHAR(255),
    "loss_rate" DECIMAL(8,4),
    "standard_loss_rate" DECIMAL(8,4),
    "material_cost" DECIMAL(14,4) NOT NULL,
    "processing_fee" DECIMAL(14,4) NOT NULL,
    "freight_fee" DECIMAL(14,4) NOT NULL,
    "tax_amount" DECIMAL(14,4) NOT NULL,
    "abnormal_loss_amount" DECIMAL(14,4) NOT NULL,
    "total_cost" DECIMAL(14,4) NOT NULL,
    "unit_cost" DECIMAL(14,4) NOT NULL,
    "status" VARCHAR(255) NOT NULL,
    "voucher_no_issue" VARCHAR(255),
    "voucher_no_fee" VARCHAR(255),
    "voucher_no_receipt" VARCHAR(255),
    "remarks" VARCHAR(255),
    "is_deleted" BOOLEAN NOT NULL,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "period_report_snapshots" (
    "id" SERIAL PRIMARY KEY,
    "period_id" INTEGER NOT NULL,
    "report_type" VARCHAR(255) NOT NULL,
    "report_data" JSONB NOT NULL,
    "snapshot_hash" VARCHAR(255) NOT NULL,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "process_quality_feedback" (
    "id" SERIAL PRIMARY KEY,
    "feedback_no" VARCHAR(255) NOT NULL,
    "flow_card_id" INTEGER NOT NULL,
    "step_record_id" INTEGER,
    "feedback_type" VARCHAR(255) NOT NULL,
    "description" VARCHAR(255) NOT NULL,
    "severity" VARCHAR(255) NOT NULL,
    "found_by" INTEGER,
    "found_at" TIMESTAMPTZ NOT NULL,
    "handling_opinion" VARCHAR(255),
    "handled_by" INTEGER,
    "handled_at" TIMESTAMPTZ,
    "handling_result" VARCHAR(255),
    "status" VARCHAR(255) NOT NULL,
    "remarks" VARCHAR(255),
    "is_deleted" BOOLEAN NOT NULL,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "process_route" (
    "id" SERIAL PRIMARY KEY,
    "route_code" VARCHAR(255) NOT NULL,
    "route_name" VARCHAR(255) NOT NULL,
    "seq" INTEGER NOT NULL,
    "process_type" VARCHAR(255) NOT NULL,
    "default_duration_minutes" INTEGER,
    "require_scan" BOOLEAN NOT NULL,
    "is_active" BOOLEAN NOT NULL,
    "remarks" VARCHAR(255),
    "is_deleted" BOOLEAN NOT NULL,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "process_step_record" (
    "id" SERIAL PRIMARY KEY,
    "flow_card_id" INTEGER NOT NULL,
    "process_route_id" INTEGER,
    "step_seq" INTEGER NOT NULL,
    "route_code" VARCHAR(255) NOT NULL,
    "route_name" VARCHAR(255) NOT NULL,
    "process_type" VARCHAR(255) NOT NULL,
    "worker_ids" VARCHAR(255),
    "worker_names" VARCHAR(255),
    "equipment_id" INTEGER,
    "equipment_name" VARCHAR(255),
    "start_at" TIMESTAMPTZ NOT NULL,
    "end_at" TIMESTAMPTZ,
    "duration_minutes" INTEGER,
    "planned_quantity" DECIMAL(12,2),
    "actual_quantity" DECIMAL(12,2),
    "qualified_quantity" DECIMAL(12,2),
    "status" VARCHAR(255) NOT NULL,
    "abnormal_description" VARCHAR(255),
    "handling_opinion" VARCHAR(255),
    "rework_source_id" INTEGER,
    "remarks" VARCHAR(255),
    "is_deleted" BOOLEAN NOT NULL,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "process_wage_rate" (
    "id" SERIAL PRIMARY KEY,
    "rate_no" VARCHAR(255) NOT NULL,
    "process_route_id" INTEGER NOT NULL,
    "route_code" VARCHAR(255) NOT NULL,
    "route_name" VARCHAR(255) NOT NULL,
    "wage_type" VARCHAR(255) NOT NULL,
    "piece_price" DECIMAL(12,4) NOT NULL,
    "time_price" DECIMAL(12,4) NOT NULL,
    "grade_a_ratio" DECIMAL(5,4) NOT NULL,
    "grade_b_ratio" DECIMAL(5,4) NOT NULL,
    "grade_c_ratio" DECIMAL(5,4) NOT NULL,
    "effective_date" DATE NOT NULL,
    "expiry_date" DATE,
    "workshop" VARCHAR(255),
    "status" VARCHAR(255) NOT NULL,
    "remarks" VARCHAR(255),
    "is_deleted" BOOLEAN NOT NULL,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "production_flow_card" (
    "id" SERIAL PRIMARY KEY,
    "card_no" VARCHAR(255) NOT NULL,
    "barcode" VARCHAR(255) NOT NULL,
    "production_order_id" INTEGER NOT NULL,
    "dye_batch_id" INTEGER,
    "dye_lot_no" VARCHAR(255),
    "process_route_id" INTEGER,
    "customer_id" INTEGER,
    "customer_name" VARCHAR(255),
    "order_no" VARCHAR(255),
    "product_id" INTEGER,
    "product_name" VARCHAR(255),
    "color_no" VARCHAR(255),
    "dyeing_requirements" VARCHAR(255),
    "planned_fabric_weight" DECIMAL(12,2),
    "actual_fabric_weight" DECIMAL(12,2),
    "current_step_seq" INTEGER NOT NULL,
    "status" VARCHAR(255) NOT NULL,
    "scheduled_at" TIMESTAMPTZ,
    "prepared_at" TIMESTAMPTZ,
    "dye_start_at" TIMESTAMPTZ,
    "dye_end_at" TIMESTAMPTZ,
    "inspected_at" TIMESTAMPTZ,
    "completed_at" TIMESTAMPTZ,
    "shipped_at" TIMESTAMPTZ,
    "priority" INTEGER NOT NULL,
    "remarks" VARCHAR(255),
    "is_deleted" BOOLEAN NOT NULL,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "production_recipe" (
    "id" SERIAL PRIMARY KEY,
    "recipe_no" VARCHAR(255) NOT NULL,
    "work_order_id" INTEGER,
    "dye_batch_id" INTEGER,
    "source_recipe_id" INTEGER,
    "lab_dip_resample_id" INTEGER,
    "customer_id" INTEGER,
    "color_no" VARCHAR(255),
    "fabric_name" VARCHAR(255),
    "fabric_spec" VARCHAR(255),
    "fabric_width" DECIMAL(10,2),
    "gram_weight" DECIMAL(10,2),
    "fabric_weight" DECIMAL(12,2) NOT NULL,
    "equipment_no" VARCHAR(255),
    "liquor_ratio" VARCHAR(255) NOT NULL,
    "bath_volume" DECIMAL(12,2),
    "adjustment_factor" DECIMAL(5,2),
    "recipe_detail" JSONB,
    "total_dye_cost" DECIMAL(12,4),
    "total_auxiliary_cost" DECIMAL(12,4),
    "status" VARCHAR(255) NOT NULL,
    "approved_by" INTEGER,
    "approved_at" TIMESTAMPTZ,
    "issued_by" INTEGER,
    "printed_count" INTEGER,
    "remarks" VARCHAR(255),
    "is_deleted" BOOLEAN NOT NULL,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "production_recipe_addition" (
    "id" SERIAL PRIMARY KEY,
    "addition_no" VARCHAR(255) NOT NULL,
    "production_recipe_id" INTEGER NOT NULL,
    "work_order_id" INTEGER,
    "dye_batch_id" INTEGER,
    "addition_reason" VARCHAR(255),
    "addition_detail" JSONB,
    "total_cost" DECIMAL(12,4),
    "status" VARCHAR(255) NOT NULL,
    "approved_by" INTEGER,
    "approved_at" TIMESTAMPTZ,
    "issued_by" INTEGER,
    "remarks" VARCHAR(255),
    "is_deleted" BOOLEAN NOT NULL,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "purchase_order_item" (
    "id" SERIAL PRIMARY KEY,
    "order_id" INTEGER NOT NULL,
    "line_no" INTEGER NOT NULL,
    "product_id" INTEGER NOT NULL,
    "quantity" DECIMAL(18,4) NOT NULL,
    "quantity_alt" DECIMAL(18,4) NOT NULL,
    "unit_price" DECIMAL(18,6) NOT NULL,
    "unit_price_foreign" DECIMAL(18,6) NOT NULL,
    "discount_percent" DECIMAL(5,4) NOT NULL,
    "tax_percent" DECIMAL(5,4) NOT NULL,
    "subtotal" DECIMAL(18,2) NOT NULL,
    "tax_amount" DECIMAL(18,2) NOT NULL,
    "discount_amount" DECIMAL(18,2) NOT NULL,
    "total_amount" DECIMAL(18,2) NOT NULL,
    "received_quantity" DECIMAL(18,4) NOT NULL,
    "received_quantity_alt" DECIMAL(18,4) NOT NULL,
    "notes" VARCHAR(255),
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL,
    "color_code" VARCHAR(255),
    "lot_no" VARCHAR(255),
    "batch_no" VARCHAR(255)
);


CREATE TABLE IF NOT EXISTS "role_change_approvals" (
    "id" SERIAL PRIMARY KEY,
    "approval_no" VARCHAR(255) NOT NULL,
    "change_type" VARCHAR(255) NOT NULL,
    "target_user_id" INTEGER,
    "target_role_id" INTEGER NOT NULL,
    "target_role_code" VARCHAR(255) NOT NULL,
    "proposed_permission_id" INTEGER,
    "proposed_resource_type" VARCHAR(255),
    "proposed_action" VARCHAR(255),
    "proposed_allowed" BOOLEAN,
    "applicant_id" INTEGER NOT NULL,
    "applicant_username" VARCHAR(255) NOT NULL,
    "approver1_id" INTEGER,
    "approver1_comment" VARCHAR(255),
    "approver1_at" TIMESTAMPTZ,
    "approver2_id" INTEGER,
    "approver2_comment" VARCHAR(255),
    "approver2_at" TIMESTAMPTZ,
    "status" VARCHAR(255) NOT NULL,
    "current_level" INTEGER NOT NULL,
    "completed_at" TIMESTAMPTZ,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "safety_accident_reports" (
    "id" SERIAL PRIMARY KEY,
    "accident_no" VARCHAR(255) NOT NULL,
    "accident_level" VARCHAR(255) NOT NULL,
    "accident_date" DATE NOT NULL,
    "location" VARCHAR(255),
    "description" VARCHAR(255) NOT NULL,
    "casualties" INTEGER NOT NULL,
    "direct_loss" DECIMAL(18,4),
    "cause" VARCHAR(255),
    "measures" VARCHAR(255),
    "reporter_id" INTEGER,
    "remarks" VARCHAR(255),
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "sales_contract_items" (
    "id" SERIAL PRIMARY KEY,
    "contract_id" INTEGER NOT NULL,
    "product_id" INTEGER,
    "product_name" VARCHAR(255) NOT NULL,
    "product_spec" VARCHAR(255),
    "unit" VARCHAR(255) NOT NULL,
    "quantity" DECIMAL(15,2) NOT NULL,
    "unit_price" DECIMAL(15,4) NOT NULL,
    "amount" DECIMAL(15,2) NOT NULL,
    "delivery_date" DATE,
    "remarks" VARCHAR(255),
    "sort_order" INTEGER NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "security_alert_logs" (
    "id" SERIAL PRIMARY KEY,
    "alert_type" VARCHAR(255) NOT NULL,
    "severity" VARCHAR(255) NOT NULL,
    "message" VARCHAR(255) NOT NULL,
    "source_ip" VARCHAR(255),
    "user_id" INTEGER,
    "details" VARCHAR(255),
    "resolved" BOOLEAN NOT NULL,
    "resolved_at" TIMESTAMPTZ,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);


CREATE TABLE IF NOT EXISTS "wage_record" (
    "id" SERIAL PRIMARY KEY,
    "record_no" VARCHAR(255) NOT NULL,
    "period_start" DATE NOT NULL,
    "period_end" DATE NOT NULL,
    "workshop" VARCHAR(255),
    "total_workers" INTEGER NOT NULL,
    "total_step_records" INTEGER NOT NULL,
    "total_qualified_quantity" DECIMAL(14,2) NOT NULL,
    "total_duration_minutes" INTEGER NOT NULL,
    "total_amount" DECIMAL(14,2) NOT NULL,
    "status" VARCHAR(255) NOT NULL,
    "confirmed_by" INTEGER,
    "confirmed_at" TIMESTAMPTZ,
    "paid_by" INTEGER,
    "paid_at" TIMESTAMPTZ,
    "remarks" VARCHAR(255),
    "is_deleted" BOOLEAN NOT NULL,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL,
    "updated_at" TIMESTAMPTZ NOT NULL
);

ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "closed_at" TIMESTAMPTZ;
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "custom_order_id" BIGINT;
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "customer_id" BIGINT;
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255);
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "issue_type" VARCHAR(255);
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "opened_at" TIMESTAMPTZ;
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "refund_amount" DECIMAL(18,4);
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "resolution" VARCHAR(255);
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "after_sales" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "action" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "after_snapshot" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "before_snapshot" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "condition" TEXT;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "created_at" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "duration_ms" INTEGER;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "ip_address" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "new_value" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "old_value" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "operation_type" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "request_body" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "request_id" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "request_method" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "request_path" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "resource_id" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "resource_name" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "resource_type" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "response_status" INTEGER;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "severity" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "user_agent" VARCHAR(255);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "user_id" INTEGER;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "username" VARCHAR(255);
ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "batch_no" VARCHAR(255);
ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "operated_at" TIMESTAMPTZ;
ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "operated_by" INTEGER;
ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "operation_type" VARCHAR(255);
ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "quantity" DECIMAL(18,4);
ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "quantity_after" DECIMAL(18,4);
ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "quantity_before" DECIMAL(18,4);
ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "remarks" VARCHAR(255);
ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "source_id" INTEGER;
ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "source_no" VARCHAR(255);
ALTER TABLE "batch_trace_log" ADD COLUMN IF NOT EXISTS "source_type" VARCHAR(255);
ALTER TABLE "color_card_issues" ADD COLUMN IF NOT EXISTS "actual_return_date" DATE;
ALTER TABLE "color_card_issues" ADD COLUMN IF NOT EXISTS "color_card_id" BIGINT;
ALTER TABLE "color_card_issues" ADD COLUMN IF NOT EXISTS "compensation_amount" DECIMAL(18,4);
ALTER TABLE "color_card_issues" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "color_card_issues" ADD COLUMN IF NOT EXISTS "customer_id" BIGINT;
ALTER TABLE "color_card_issues" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "color_card_issues" ADD COLUMN IF NOT EXISTS "expected_return_date" DATE;
ALTER TABLE "color_card_issues" ADD COLUMN IF NOT EXISTS "is_deleted" BOOLEAN;
ALTER TABLE "color_card_issues" ADD COLUMN IF NOT EXISTS "issue_qty" INTEGER;
ALTER TABLE "color_card_issues" ADD COLUMN IF NOT EXISTS "issued_at" TIMESTAMPTZ;
ALTER TABLE "color_card_issues" ADD COLUMN IF NOT EXISTS "issued_by" BIGINT;
ALTER TABLE "color_card_issues" ADD COLUMN IF NOT EXISTS "purpose" VARCHAR(255);
ALTER TABLE "color_card_issues" ADD COLUMN IF NOT EXISTS "remark" VARCHAR(255);
ALTER TABLE "color_card_issues" ADD COLUMN IF NOT EXISTS "returned_by" BIGINT;
ALTER TABLE "color_card_issues" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "color_card_issues" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "actual_delivery_date" DATE;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "color_id" BIGINT;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "created_by" BIGINT;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "currency" VARCHAR(255);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "custom_requirements" JSONB;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "customer_id" BIGINT;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "dye_method" VARCHAR(255);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "expected_delivery_date" DATE;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "finishing_method" VARCHAR(255);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "notes" VARCHAR(255);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "order_no" VARCHAR(255);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "product_id" BIGINT;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "quantity" DECIMAL(18,4);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "sales_order_id" BIGINT;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "spec" VARCHAR(255);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "total_amount" DECIMAL(18,4);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "unit" VARCHAR(255);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "yarn_spec" VARCHAR(255);
ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "bcc" VARCHAR(255);
ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "body" VARCHAR(255);
ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "cc" VARCHAR(255);
ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "error_message" VARCHAR(255);
ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "external_message_id" VARCHAR(255);
ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "recipients" VARCHAR(255);
ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "retry_count" INTEGER;
ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "sent_at" TIMESTAMPTZ;
ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "subject" VARCHAR(255);
ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "template_id" INTEGER;
ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "email_logs" ADD COLUMN IF NOT EXISTS "user_id" INTEGER;
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "batch_no" VARCHAR(255);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "color_code" VARCHAR(255);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "composition" VARCHAR(255);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "density" VARCHAR(255);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "fabric_name" VARCHAR(255);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "fabric_no" VARCHAR(255);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "fabric_type" VARCHAR(255);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "gram_weight" DECIMAL(10,2);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "is_deleted" BOOLEAN;
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "length_m" DECIMAL(12,2);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "location" VARCHAR(255);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "product_id" INTEGER;
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "production_date" DATE;
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "purchase_date" DATE;
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "quality_grade" VARCHAR(255);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "quantity_kg" DECIMAL(12,2);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "quantity_meters" DECIMAL(12,2);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "remarks" VARCHAR(255);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "structure" VARCHAR(255);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "supplier_id" INTEGER;
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "warehouse_id" INTEGER;
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "weight_kg" DECIMAL(10,2);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "width" DECIMAL(10,2);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "width_cm" DECIMAL(10,2);
ALTER TABLE "greige_fabric" ADD COLUMN IF NOT EXISTS "yarn_count" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "batch_no" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "bin_location" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "expiry_date" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "grade" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "gram_weight" DECIMAL(10,2);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "last_count_date" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "last_movement_date" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "layer_no" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "location_id" INTEGER;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "max_stock_point" DECIMAL(12,2);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "product_id" INTEGER;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "production_date" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quality_status" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quantity_available" DECIMAL(18,4);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quantity_incoming" DECIMAL(18,4);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quantity_kg" DECIMAL(12,2);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quantity_meters" DECIMAL(12,2);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quantity_on_hand" DECIMAL(18,4);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quantity_reserved" DECIMAL(18,4);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "quantity_shipped" DECIMAL(18,4);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "reorder_point" DECIMAL(18,4);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "reorder_quantity" DECIMAL(18,4);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "shelf_no" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "stock_status" VARCHAR(255);
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "version" INTEGER;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "warehouse_id" INTEGER;
ALTER TABLE "inventory_stocks" ADD COLUMN IF NOT EXISTS "width" DECIMAL(10,2);
ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "approved_at" TIMESTAMPTZ;
ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "approved_by" INTEGER;
ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "from_warehouse_id" INTEGER;
ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "notes" VARCHAR(255);
ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "received_at" TIMESTAMPTZ;
ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "shipped_at" TIMESTAMPTZ;
ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "to_warehouse_id" INTEGER;
ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "total_quantity" DECIMAL(18,4);
ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "transfer_date" TIMESTAMPTZ;
ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "transfer_no" VARCHAR(255);
ALTER TABLE "inventory_transfers" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "actual_arrival" TIMESTAMPTZ;
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "driver_name" VARCHAR(255);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "driver_phone" VARCHAR(255);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "expected_arrival" TIMESTAMPTZ;
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "freight_fee" DECIMAL(18,4);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "logistics_company" VARCHAR(255);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "notes" VARCHAR(255);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "order_id" INTEGER;
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "tracking_number" VARCHAR(255);
ALTER TABLE "logistics_waybills" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "oa_announcement" ADD COLUMN IF NOT EXISTS "announcement_type" VARCHAR(255);
ALTER TABLE "oa_announcement" ADD COLUMN IF NOT EXISTS "attachments" JSONB;
ALTER TABLE "oa_announcement" ADD COLUMN IF NOT EXISTS "content" VARCHAR(255);
ALTER TABLE "oa_announcement" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "oa_announcement" ADD COLUMN IF NOT EXISTS "effective_date" DATE;
ALTER TABLE "oa_announcement" ADD COLUMN IF NOT EXISTS "expiry_date" DATE;
ALTER TABLE "oa_announcement" ADD COLUMN IF NOT EXISTS "is_top" BOOLEAN;
ALTER TABLE "oa_announcement" ADD COLUMN IF NOT EXISTS "publish_date" DATE;
ALTER TABLE "oa_announcement" ADD COLUMN IF NOT EXISTS "publisher_id" INTEGER;
ALTER TABLE "oa_announcement" ADD COLUMN IF NOT EXISTS "remarks" VARCHAR(255);
ALTER TABLE "oa_announcement" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "oa_announcement" ADD COLUMN IF NOT EXISTS "title" VARCHAR(255);
ALTER TABLE "oa_announcement" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "actual_end_date" DATE;
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "actual_quantity" DECIMAL(18,4);
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "actual_start_date" DATE;
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "batch_no" VARCHAR(255);
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "order_no" VARCHAR(255);
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "order_type" VARCHAR(255);
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "original_batch_id" INTEGER;
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "planned_end_date" DATE;
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "planned_quantity" DECIMAL(18,4);
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "planned_start_date" DATE;
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "priority" INTEGER;
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "product_id" INTEGER;
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "remarks" VARCHAR(255);
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "sales_order_id" INTEGER;
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "production_orders" ADD COLUMN IF NOT EXISTS "work_center_id" INTEGER;
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "batch_level" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "category_id" INTEGER;
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "code" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "cost_price" DECIMAL(18,4);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "density" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "fabric_composition" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "finish" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "gram_weight" DECIMAL(10,2);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "is_batch_managed" BOOLEAN;
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "is_deleted" BOOLEAN;
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "lead_time" INTEGER;
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "min_order_quantity" DECIMAL(12,2);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "name" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "product_type" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "specification" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "standard_price" DECIMAL(18,4);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "structure" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "supplier_id" INTEGER;
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "supplier_product_code" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "unit" VARCHAR(255);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "width" DECIMAL(10,2);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "yarn_count" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "auxiliary_type" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "batch_no" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "customer_id" INTEGER;
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "dye_type" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "fabric_source" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "grade" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "inspected_qty" DECIMAL(18,4);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "inspection_date" DATE;
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "inspection_no" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "inspection_result" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "inspection_type" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "inspector_id" INTEGER;
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "product_id" INTEGER;
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "qualification_rate" DECIMAL(18,4);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "qualified_qty" DECIMAL(18,4);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "related_id" INTEGER;
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "related_type" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "remark" VARCHAR(255);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "supplier_id" INTEGER;
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "temperature" DECIMAL(18,4);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "total_qty" DECIMAL(18,4);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "unqualified_qty" DECIMAL(18,4);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "custom_order_id" BIGINT;
ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255);
ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "discovered_at" TIMESTAMPTZ;
ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "issue_type" VARCHAR(255);
ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "process_node_id" BIGINT;
ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "resolution" VARCHAR(255);
ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "resolved_at" TIMESTAMPTZ;
ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "severity" VARCHAR(255);
ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "quality_issues" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "export_format" VARCHAR(255);
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "frequency" VARCHAR(255);
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "is_enabled" BOOLEAN;
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "last_run_at" TIMESTAMPTZ;
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "last_run_error" VARCHAR(255);
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "last_run_status" VARCHAR(255);
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "name" VARCHAR(255);
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "next_run_at" TIMESTAMPTZ;
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "parameters" JSONB;
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "recipients" JSONB;
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "run_count" INTEGER;
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "template_id" INTEGER;
ALTER TABLE "report_subscriptions" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "cache_ttl_seconds" INTEGER;
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "category" VARCHAR(255);
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "code" VARCHAR(255);
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "columns" JSONB;
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "data_source" VARCHAR(255);
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "data_source_sql" VARCHAR(255);
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255);
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "filters" JSONB;
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "is_public" BOOLEAN;
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "name" VARCHAR(255);
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "parameters" JSONB;
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "refresh_strategy" VARCHAR(255);
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "report_type" VARCHAR(255);
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "sort_by" VARCHAR(255);
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "sort_order" VARCHAR(255);
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "supported_formats" JSONB;
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "template_id" VARCHAR(255);
ALTER TABLE "report_templates" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "breach_liability" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "contract_name" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "contract_no" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "contract_type" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "customer_id" INTEGER;
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "customer_name" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "delivery_date" DATE;
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "delivery_location" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "dispute_resolution" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "effective_date" DATE;
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "expiry_date" DATE;
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "payment_method" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "payment_terms" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "performance_period" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "quality_terms" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "signed_date" DATE;
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "stamp_tax_amount" DECIMAL(18,4);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "total_amount" DECIMAL(18,4);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "approval_instance_id" BIGINT;
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "approved_at" TIMESTAMPTZ;
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "approved_by" BIGINT;
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "base_currency" VARCHAR(255);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "converted_at" TIMESTAMPTZ;
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "converted_sales_order_id" BIGINT;
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "created_by" BIGINT;
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "currency" VARCHAR(255);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "customer_id" BIGINT;
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "customer_level" VARCHAR(255);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "exchange_rate" DECIMAL(18,4);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "incoterm_location" VARCHAR(255);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "incoterms_version" VARCHAR(255);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "lead_time_days" INTEGER;
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "moq" DECIMAL(18,4);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "notes" VARCHAR(255);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "price_terms" VARCHAR(255);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "quotation_date" DATE;
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "quotation_no" VARCHAR(255);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "rejection_reason" VARCHAR(255);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "sales_user_id" BIGINT;
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "subtotal" DECIMAL(18,4);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "tax_amount" DECIMAL(18,4);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "tax_inclusive" BOOLEAN;
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "tax_rate" DECIMAL(18,4);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "total_amount" DECIMAL(18,4);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "valid_until" DATE;
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "batch_no" VARCHAR(255);
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "grade" VARCHAR(255);
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "handling_at" TIMESTAMPTZ;
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "handling_by" INTEGER;
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "handling_method" VARCHAR(255);
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "handling_result" VARCHAR(255);
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "handling_status" VARCHAR(255);
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "inspection_id" INTEGER;
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "product_id" INTEGER;
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "remark" VARCHAR(255);
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "unqualified_no" VARCHAR(255);
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "unqualified_qty" DECIMAL(18,4);
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "unqualified_reason" VARCHAR(255);
ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "capacity_unit" VARCHAR(255);
ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "code" VARCHAR(255);
ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "daily_capacity" DECIMAL(18,4);
ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "name" VARCHAR(255);
ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "remarks" VARCHAR(255);
ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "work_centers" ADD COLUMN IF NOT EXISTS "work_center_type" VARCHAR(255);
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
