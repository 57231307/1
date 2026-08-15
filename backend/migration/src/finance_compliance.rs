//! 财务/合规/审计
//!
//! 合并自: 10 个迁移文件

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // === m0051_add_data_scope_to_roles.rs ===
let sql = include_str!("../../migrations/20260716000001_add_data_scope_to_roles/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        // === m0052_create_role_conflicts.rs ===
// 创建 role_conflicts 表
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS role_conflicts (
                    id SERIAL PRIMARY KEY,
                    role_a_code VARCHAR(50) NOT NULL,
                    role_b_code VARCHAR(50) NOT NULL,
                    conflict_type VARCHAR(50) NOT NULL DEFAULT 'sod',
                    description VARCHAR(200),
                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 确保 role_a_code < role_b_code 避免重复对（A-B 和 B-A）
                    CONSTRAINT chk_role_order CHECK (role_a_code < role_b_code),
                    CONSTRAINT uniq_role_pair UNIQUE (role_a_code, role_b_code)
                );

                -- 创建索引加速查询
                CREATE INDEX idx_role_conflicts_a ON role_conflicts (role_a_code);
                CREATE INDEX idx_role_conflicts_b ON role_conflicts (role_b_code);

                -- 预置财务三权分立互斥规则
                INSERT INTO role_conflicts (role_a_code, role_b_code, conflict_type, description) VALUES
                    ('accountant', 'finance_manager', 'sod', '财务制单与审核互斥'),
                    ('accountant', 'cashier', 'sod', '财务制单与出纳互斥'),
                    ('finance_manager', 'cashier', 'sod', '财务审核与出纳互斥'),
                    -- 采购与付款互斥
                    ('purchase_manager', 'cashier', 'sod', '采购审批与付款互斥'),
                    ('purchase_clerk', 'cashier', 'sod', '采购执行与付款互斥'),
                    -- 销售与收款互斥
                    ('sales_manager', 'cashier', 'sod', '销售审批与收款互斥'),
                    -- 生产与质量互斥
                    ('production_manager', 'qc_manager', 'sod', '生产与质量管理互斥'),
                    ('dyeing_master', 'quality_inspector', 'sod', '染色主管与质检员互斥')
                ON CONFLICT (role_a_code, role_b_code) DO NOTHING;
                "#,
            )
            .await?;

        Ok(())
        // === m0053_create_permission_change_audit.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS permission_change_audits (
                    id SERIAL PRIMARY KEY,
                    -- 变更类型：role_permission_assign / role_permission_remove / user_role_change
                    change_type VARCHAR(50) NOT NULL,
                    -- 操作人 ID
                    operator_id INTEGER NOT NULL,
                    -- 受影响角色 ID
                    role_id INTEGER,
                    -- 受影响用户 ID（user_role_change 时有值）
                    user_id INTEGER,
                    -- 资源类型（role_permission 变更时有值）
                    resource_type VARCHAR(100),
                    -- 操作权限码（role_permission 变更时有值）
                    action VARCHAR(50),
                    -- 旧值（如旧 role_id / 旧 allowed）
                    old_value VARCHAR(200),
                    -- 新值（如新 role_id / 新 allowed）
                    new_value VARCHAR(200),
                    -- 变更时间
                    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 客户端 IP
                    client_ip VARCHAR(45),
                    -- 备注
                    remark TEXT
                );

                -- 创建索引加速查询
                CREATE INDEX idx_pca_change_type ON permission_change_audits (change_type);
                CREATE INDEX idx_pca_operator ON permission_change_audits (operator_id);
                CREATE INDEX idx_pca_role ON permission_change_audits (role_id);
                CREATE INDEX idx_pca_user ON permission_change_audits (user_id);
                CREATE INDEX idx_pca_changed_at ON permission_change_audits (changed_at);
                "#,
            )
            .await?;

        Ok(())
        // === m0054_enable_rls_policies.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- 1. customers 表（owner_id NOT NULL，0 表示公海）
                ALTER TABLE customers ENABLE ROW LEVEL SECURITY;

                CREATE POLICY customers_isolation ON customers
                    FOR ALL
                    USING (
                        current_setting('app.user_id', true) IS NULL
                        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
                        OR owner_id = current_setting('app.user_id', true)::int
                        OR owner_id = 0
                    )
                    WITH CHECK (
                        current_setting('app.user_id', true) IS NULL
                        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
                        OR owner_id = current_setting('app.user_id', true)::int
                        OR owner_id = 0
                    );

                -- 2. suppliers 表（created_by 可空，历史数据 NULL）
                ALTER TABLE suppliers ENABLE ROW LEVEL SECURITY;

                CREATE POLICY suppliers_isolation ON suppliers
                    FOR ALL
                    USING (
                        current_setting('app.user_id', true) IS NULL
                        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
                        OR created_by IS NULL
                        OR created_by = current_setting('app.user_id', true)::int
                    )
                    WITH CHECK (
                        current_setting('app.user_id', true) IS NULL
                        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
                        OR created_by IS NULL
                        OR created_by = current_setting('app.user_id', true)::int
                    );

                -- 3. sales_orders 表（created_by 可空，历史数据 NULL）
                ALTER TABLE sales_orders ENABLE ROW LEVEL SECURITY;

                CREATE POLICY sales_orders_isolation ON sales_orders
                    FOR ALL
                    USING (
                        current_setting('app.user_id', true) IS NULL
                        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
                        OR created_by IS NULL
                        OR created_by = current_setting('app.user_id', true)::int
                    )
                    WITH CHECK (
                        current_setting('app.user_id', true) IS NULL
                        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
                        OR created_by IS NULL
                        OR created_by = current_setting('app.user_id', true)::int
                    );

                -- 4. crm_lead 表（owner_id NOT NULL）
                ALTER TABLE crm_lead ENABLE ROW LEVEL SECURITY;

                CREATE POLICY crm_lead_isolation ON crm_lead
                    FOR ALL
                    USING (
                        current_setting('app.user_id', true) IS NULL
                        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
                        OR owner_id = current_setting('app.user_id', true)::int
                    )
                    WITH CHECK (
                        current_setting('app.user_id', true) IS NULL
                        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
                        OR owner_id = current_setting('app.user_id', true)::int
                    );

                -- 5. crm_opportunity 表（owner_id NOT NULL）
                ALTER TABLE crm_opportunity ENABLE ROW LEVEL SECURITY;

                CREATE POLICY crm_opportunity_isolation ON crm_opportunity
                    FOR ALL
                    USING (
                        current_setting('app.user_id', true) IS NULL
                        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
                        OR owner_id = current_setting('app.user_id', true)::int
                    )
                    WITH CHECK (
                        current_setting('app.user_id', true) IS NULL
                        OR current_setting('app.role_code', true) IN ('admin', 'gm', 'deputy_gm')
                        OR owner_id = current_setting('app.user_id', true)::int
                    );
                "#,
            )
            .await?;

        Ok(())
        // === m0055_create_export_approval_request.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE IF NOT EXISTS export_approval_request (
                    id BIGSERIAL PRIMARY KEY,
                    -- 申请人用户 ID
                    applicant_user_id INTEGER NOT NULL,
                    -- 申请人用户名
                    applicant_username VARCHAR(100) NOT NULL,
                    -- 审批人用户 ID（二级审批时填充）
                    approver_user_id INTEGER,
                    -- 审批人用户名
                    approver_username VARCHAR(100),
                    -- 导出资源类型：customer/supplier/dye_recipe/price_list/finance_report 等
                    resource_type VARCHAR(100) NOT NULL,
                    -- 导出参数 JSON（过滤条件/字段选择等）
                    export_params JSONB,
                    -- 预估导出行数
                    estimated_rows BIGINT,
                    -- 文件格式：xlsx/pdf/csv
                    file_format VARCHAR(20) NOT NULL,
                    -- 审批状态：pending/approved/rejected/expired/cancelled
                    status VARCHAR(20) NOT NULL,
                    -- 当前审批层级：1=一级，2=二级
                    approval_level INTEGER NOT NULL,
                    -- 审批人备注
                    approver_comments TEXT,
                    -- 审批通过时间
                    approved_at TIMESTAMPTZ,
                    -- 审批拒绝时间
                    rejected_at TIMESTAMPTZ,
                    -- 临时下载令牌（审批通过后生成，5 分钟有效，防重放攻击）
                    download_token VARCHAR(100),
                    -- token 过期时间（approved_at + 5min）
                    token_expires_at TIMESTAMPTZ,
                    -- 已下载次数
                    download_count INTEGER NOT NULL DEFAULT 0,
                    -- 最大下载次数（默认 1，防重放攻击）
                    max_downloads INTEGER NOT NULL DEFAULT 1,
                    -- 导出文件临时存储路径
                    file_path VARCHAR(500),
                    -- 文件大小（字节）
                    file_size_bytes BIGINT,
                    -- 文件 SHA256 校验值
                    file_checksum VARCHAR(64),
                    -- 申请人 IP
                    applicant_ip VARCHAR(45),
                    -- 审批人 IP
                    approver_ip VARCHAR(45),
                    -- 申请人 User-Agent
                    applicant_user_agent VARCHAR(500),
                    -- 风险等级：low/medium/high/critical
                    risk_level VARCHAR(20) NOT NULL,
                    -- 审批上下文（JSON）
                    context JSONB,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 流程终结时间（下载完成或 token 过期）
                    completed_at TIMESTAMPTZ
                );

                -- 索引：按状态查询待审批/已通过列表
                CREATE INDEX idx_ear_status ON export_approval_request (status);
                -- 索引：按申请人查询（我的申请列表）
                CREATE INDEX idx_ear_applicant ON export_approval_request (applicant_user_id);
                -- 索引：按审批人查询（待我审批列表）
                CREATE INDEX idx_ear_approver ON export_approval_request (approver_user_id);
                -- 索引：按资源类型查询（资源审批历史）
                CREATE INDEX idx_ear_resource_type ON export_approval_request (resource_type);
                -- 索引：按 download_token 查询（下载校验高频查询）
                CREATE UNIQUE INDEX idx_ear_download_token ON export_approval_request (download_token) WHERE download_token IS NOT NULL;
                -- 索引：按风险等级查询（高风险导出监控）
                CREATE INDEX idx_ear_risk_level ON export_approval_request (risk_level);
                "#,
            )
            .await?;

        Ok(())
        // === m0056_add_condition_to_audit_logs.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- audit_logs 表新增 condition 字段（请求条件/查询条件）
                -- 与 request_body 区分：request_body 记录完整请求体，condition 仅记录查询条件（query string）
                -- 用于快速筛选特定条件下的导出/查询审计记录
                ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS condition TEXT;

                -- omni_audit_logs 表新增 condition 字段
                ALTER TABLE omni_audit_logs ADD COLUMN IF NOT EXISTS condition TEXT;
                "#,
            )
            .await?;

        Ok(())
        // === m0057_create_color_card_issues_and_stock_fields.rs ===
// 1. 创建 color_card_issues 表（与 color_card_issue.rs model 字段对齐，17 字段 + 5 索引）
        manager
            .get_connection()
            .execute_unprepared(
                r#"
CREATE TABLE IF NOT EXISTS "color_card_issues" (
    "id" BIGSERIAL PRIMARY KEY,
    "color_card_id" BIGINT NOT NULL REFERENCES "color_cards"("id") ON DELETE RESTRICT,
    "customer_id" BIGINT NOT NULL REFERENCES "customers"("id") ON DELETE RESTRICT,
    "issue_qty" INTEGER NOT NULL CHECK ("issue_qty" > 0),
    "issued_by" BIGINT NOT NULL REFERENCES "users"("id") ON DELETE RESTRICT,
    "issued_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "expected_return_date" DATE,
    "actual_return_date" DATE,
    "status" VARCHAR(20) NOT NULL DEFAULT 'issued',
    "purpose" TEXT,
    "remark" TEXT,
    "compensation_amount" DECIMAL(15,2) CHECK ("compensation_amount" IS NULL OR "compensation_amount" >= 0),
    "returned_by" BIGINT REFERENCES "users"("id") ON DELETE SET NULL,
    "dye_lot_no" VARCHAR(50),
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    CONSTRAINT "chk_issue_status" CHECK ("status" IN ('issued', 'returned', 'lost', 'damaged', 'cancelled'))
);

CREATE INDEX IF NOT EXISTS "idx_issue_card" ON "color_card_issues"("color_card_id");
CREATE INDEX IF NOT EXISTS "idx_issue_customer" ON "color_card_issues"("customer_id");
CREATE INDEX IF NOT EXISTS "idx_issue_status" ON "color_card_issues"("status");
CREATE INDEX IF NOT EXISTS "idx_issue_issued_at" ON "color_card_issues"("issued_at" DESC);
CREATE INDEX IF NOT EXISTS "idx_issue_issued_by" ON "color_card_issues"("issued_by");

COMMENT ON TABLE "color_card_issues" IS '色卡发放记录 - 发放/归还/遗失/损坏/取消全生命周期跟踪（V15 P0-F04 替代旧 color_card_borrow_records）';
COMMENT ON COLUMN "color_card_issues"."status" IS '发放状态：issued(发放中) / returned(已归还) / lost(遗失) / damaged(损坏) / cancelled(已取消)';
COMMENT ON COLUMN "color_card_issues"."issue_qty" IS '发放数量（必须 > 0）';
COMMENT ON COLUMN "color_card_issues"."dye_lot_no" IS '染色批号（lot 概念，防色差混批）';
"#,
            )
            .await?;

        // 2. color_cards 表新增 stock_quantity / issued_quantity 字段（库存联动基础）
        manager
            .get_connection()
            .execute_unprepared(
                r#"
ALTER TABLE "color_cards"
    ADD COLUMN IF NOT EXISTS "stock_quantity" INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS "issued_quantity" INTEGER NOT NULL DEFAULT 0;

COMMENT ON COLUMN "color_cards"."stock_quantity" IS '色卡总库存数量（V15 P0-F10 库存联动）';
COMMENT ON COLUMN "color_cards"."issued_quantity" IS '已发放数量（V15 P0-F10 库存联动，issued_qty <= stock_quantity）';
"#,
            )
            .await?;

        Ok(())
        // === m0058_create_bulk_color_approval.rs ===
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
        // === m0059_add_rework_order_fields.rs ===
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
        // === m0060_create_quality_8d_reports.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- 8D 质量管理流程报告表（V15 P0-F20 创建）
                -- 与 quality_issues 一对一关联（一个质量异常最多启动一个 8D 报告）
                -- 11 态状态机：not_started → d0_plan → d1_team → d2_problem → d3_interim
                --              → d4_root_cause → d5_permanent → d6_verify → d7_prevent
                --              → d8_recognize → closed
                CREATE TABLE IF NOT EXISTS "quality_8d_reports" (
                    "id" BIGSERIAL PRIMARY KEY,
                    -- 关联质量异常（一对一）
                    "quality_issue_id" BIGINT NOT NULL REFERENCES "quality_issues"("id") ON DELETE CASCADE,
                    -- 11 态状态机
                    "status" VARCHAR(20) NOT NULL DEFAULT 'not_started',
                    -- D0 准备阶段（计划与发起）
                    "d0_date" TIMESTAMPTZ,
                    "d0_prepared_by" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    "d0_plan" TEXT,
                    -- D1 团队组建
                    "d1_date" TIMESTAMPTZ,
                    "d1_team_members" TEXT,
                    -- D2 问题描述
                    "d2_date" TIMESTAMPTZ,
                    "d2_problem_description" TEXT,
                    -- D3 临时措施（围堵）
                    "d3_date" TIMESTAMPTZ,
                    "d3_interim_action" TEXT,
                    -- D4 根本原因分析（缺陷 4.2：5Why/鱼骨图）
                    "d4_date" TIMESTAMPTZ,
                    "d4_root_cause_method" VARCHAR(20),
                    "d4_root_cause_detail" TEXT,
                    "d4_root_cause_summary" TEXT,
                    -- D5 永久纠正措施（缺陷 4.3：责任人 + 完成日期跟踪）
                    "d5_date" TIMESTAMPTZ,
                    "d5_permanent_action" TEXT,
                    "d5_action_owner" VARCHAR(100),
                    "d5_due_date" DATE,
                    "d5_completed_at" TIMESTAMPTZ,
                    -- D6 实施验证
                    "d6_date" TIMESTAMPTZ,
                    "d6_verification_result" TEXT,
                    -- D7 预防措施（标准化）
                    "d7_date" TIMESTAMPTZ,
                    "d7_prevention_action" TEXT,
                    -- D8 团队表彰与闭环
                    "d8_date" TIMESTAMPTZ,
                    "d8_closure_summary" TEXT,
                    -- 关闭信息
                    "closed_at" TIMESTAMPTZ,
                    "closed_by" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    -- 元数据
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 约束
                    CONSTRAINT "chk_q8d_status" CHECK (
                        "status" IN (
                            'not_started', 'd0_plan', 'd1_team', 'd2_problem', 'd3_interim',
                            'd4_root_cause', 'd5_permanent', 'd6_verify', 'd7_prevent',
                            'd8_recognize', 'closed'
                        )
                    ),
                    CONSTRAINT "chk_q8d_root_cause_method" CHECK (
                        "d4_root_cause_method" IS NULL OR
                        "d4_root_cause_method" IN ('5why', 'fishbone', 'other')
                    )
                );

                -- 索引（3 个，覆盖高频查询场景）
                CREATE INDEX IF NOT EXISTS "idx_q8d_quality_issue_id" ON "quality_8d_reports"("quality_issue_id");
                CREATE INDEX IF NOT EXISTS "idx_q8d_status" ON "quality_8d_reports"("status");
                -- 一个 quality_issue 最多一个 8D 报告（一对一）
                CREATE UNIQUE INDEX IF NOT EXISTS "uq_q8d_quality_issue_id" ON "quality_8d_reports"("quality_issue_id");

                COMMENT ON TABLE "quality_8d_reports" IS '8D 质量管理流程报告表 - D0~D8 八步流程 + 11 态状态机';
                COMMENT ON COLUMN "quality_8d_reports"."d4_root_cause_method" IS '根因分析方法：5why（五问法）/ fishbone（鱼骨图）/ other（其他）';
                COMMENT ON COLUMN "quality_8d_reports"."d5_action_owner" IS 'D5 永久措施责任人姓名或工号';
                COMMENT ON COLUMN "quality_8d_reports"."d5_due_date" IS 'D5 永久措施计划完成日期（超期由定时任务扫描告警）';
                "#,
            )
            .await?;
        Ok(())
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
