//! V15 批次19
//!
//! 合并自: 10 个迁移文件

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // === m0081_create_fixed_asset_counts.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
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
                "#,
            )
            .await?;
        // === m0082_create_customer_team_and_share.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
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
                "#,
            )
            .await?;
        // === m0083_create_report_template_versions.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
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
                    "snapshot_at" TIMESTAMP NOT NULL DEFAULT NOW()
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
                    ADD COLUMN IF NOT EXISTS "next_retry_at" TIMESTAMP;
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
                    "created_at" TIMESTAMP NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMP NOT NULL DEFAULT NOW()
                );

                CREATE INDEX IF NOT EXISTS "idx_dashboard_layouts_user_id"
                    ON "dashboard_layouts"("user_id");

                COMMENT ON TABLE "dashboard_layouts" IS '用户仪表板布局配置表（卡片顺序/可见性/尺寸）';
                COMMENT ON COLUMN "dashboard_layouts"."user_id" IS '用户 ID（每个用户独立布局）';
                COMMENT ON COLUMN "dashboard_layouts"."card_config" IS '卡片配置 JSON（卡片顺序、可见性、尺寸等）';
                COMMENT ON COLUMN "dashboard_layouts"."is_default" IS '是否默认布局（true 时其他用户初始可见）';
                "#,
            )
            .await?;
        // === m0084_add_color_card_issue_export_permissions.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
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
                "#,
            )
            .await?;
        // === m0085_create_bulk_color_approval_history.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
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
                "#,
            )
            .await?;
        // === m0086_add_inspection_id_to_outsourcing_receipt.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE "outsourcing_receipt"
                    ADD COLUMN IF NOT EXISTS "inspection_id" INTEGER;

                COMMENT ON COLUMN "outsourcing_receipt"."inspection_id" IS
                    '缺陷 2.2：关联质检记录 ID（确认收回时自动创建质检记录并回写）';

                CREATE INDEX IF NOT EXISTS "idx_outsourcing_receipt_inspection_id"
                    ON "outsourcing_receipt" ("inspection_id")
                    WHERE "inspection_id" IS NOT NULL;
                "#,
            )
            .await?;
        // === m0087_batch19_custom_order_aftersales_logistics_incoterms.rs ===
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
        // === m0088_audit_log_export_log.rs ===
let sql = include_str!("../../migrations/20260801000002_audit_log_export_log/up.sql");
        manager.get_connection().execute_unprepared(sql).await?;
        // === m0089_add_rework_cost_to_dye_batch_rework.rs ===
let sql = include_str!(
            "../../migrations/20260801000003_add_rework_cost_to_dye_batch_rework/up.sql"
        );
        manager.get_connection().execute_unprepared(sql).await?;
        // === m0090_create_dye_vat_occupation.rs ===
let sql = include_str!("../../migrations/20260801000004_create_dye_vat_occupation/up.sql");
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // === m0081_create_fixed_asset_counts.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP TABLE IF EXISTS "fixed_asset_count_items";
                DROP TABLE IF EXISTS "fixed_asset_counts";
                "#,
            )
            .await?;
        // === m0082_create_customer_team_and_share.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP TABLE IF EXISTS "customer_shares";
                DROP TABLE IF EXISTS "customer_team_members";
                "#,
            )
            .await?;
        // === m0083_create_report_template_versions.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP TABLE IF EXISTS "dashboard_layouts";
                DROP TABLE IF EXISTS "report_template_versions";
                "#,
            )
            .await?;
        // === m0084_add_color_card_issue_export_permissions.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- 回滚：撤销本迁移为非 admin 角色授予的 color_card_issue:export 权限
                -- 仅删除 sales_manager / warehouse_manager / cost_accountant 的 export 权限，
                -- 不影响 admin 的同名权限（admin 由 init_admin_permissions.sql 维护）。
                DELETE FROM role_permissions rp
                USING roles r
                WHERE rp.role_id = r.id
                  AND rp.resource_type = 'color_card_issue'
                  AND rp.action = 'export'
                  AND r.code IN ('sales_manager', 'warehouse_manager', 'cost_accountant')
                  AND r.is_system = true;
                "#,
            )
            .await?;
        // === m0085_create_bulk_color_approval_history.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP TABLE IF EXISTS "bulk_color_approval_history";
                "#,
            )
            .await?;
        // === m0086_add_inspection_id_to_outsourcing_receipt.rs ===
manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP INDEX IF EXISTS "idx_outsourcing_receipt_inspection_id";
                ALTER TABLE "outsourcing_receipt" DROP COLUMN IF EXISTS "inspection_id";
                "#,
            )
            .await?;
        // === m0087_batch19_custom_order_aftersales_logistics_incoterms.rs ===
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
        // === m0088_audit_log_export_log.rs ===
let sql = include_str!("../../migrations/20260801000002_audit_log_export_log/down.sql");
        manager.get_connection().execute_unprepared(sql).await?;
        // === m0089_add_rework_cost_to_dye_batch_rework.rs ===
let sql = include_str!(
            "../../migrations/20260801000003_add_rework_cost_to_dye_batch_rework/down.sql"
        );
        manager.get_connection().execute_unprepared(sql).await?;
        // === m0090_create_dye_vat_occupation.rs ===
let sql =
            include_str!("../../migrations/20260801000004_create_dye_vat_occupation/down.sql");
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}


