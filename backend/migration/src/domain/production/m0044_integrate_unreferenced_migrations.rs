//! 批次 190 迁移整合：执行所有未被 Rust 模块引用的 SQL 迁移
//!
//! 问题背景：
//! 31 个 SQL 迁移目录（20260616/17/18/0706/0707 系列）从未被 Rust 迁移模块引用，
//! 导致这些表/列从未创建。m0029_drop_tenant_columns 尝试 ALTER TABLE custom_orders
//! 时报错 "relation custom_orders does not exist"。
//!
//! 整合方案（规则 0/2 真实修复 + 用户指示"迁移文件太多需要整合"）：
//! 内联所有未引用 SQL 的 up.sql，按依赖顺序执行。
//! 所有 SQL 均使用 IF NOT EXISTS / IF EXISTS，保证幂等可重入。
//! 本迁移注册在 m0028 之后、m0029 之前，确保 custom_orders 等表在 drop_tenant_columns 之前创建。

use sea_orm_migration::prelude::*;

/// 整合迁移条目：(名称, up.sql 内容)
const UNREFERENCED_MIGRATIONS: &[(&str, &str)] = &[
    // 20260616 系列（容灾表）
    (
        "20260616000005_create_failover_tables",
        r#"-- 主备隔离模块 migration
-- 创建 3 张核心表：failover_status / failover_event / failover_config

-- 1. 主备实时状态表：记录每个功能（数据库/缓存）当前的主备状态
CREATE TABLE IF NOT EXISTS failover_status (
    id BIGSERIAL PRIMARY KEY,
    function_name VARCHAR(100) NOT NULL UNIQUE,
    current_state VARCHAR(20) NOT NULL DEFAULT 'primary',
    circuit_state VARCHAR(20) NOT NULL DEFAULT 'closed',
    primary_url VARCHAR(500),
    backup_type VARCHAR(50),
    last_switch_at TIMESTAMPTZ,
    last_success_at TIMESTAMPTZ,
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    total_primary_calls BIGINT NOT NULL DEFAULT 0,
    total_backup_calls BIGINT NOT NULL DEFAULT 0,
    total_switches BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_failover_status_state CHECK (current_state IN ('primary', 'backup', 'both_down')),
    CONSTRAINT chk_failover_status_circuit CHECK (circuit_state IN ('closed', 'open', 'half_open'))
);

CREATE INDEX IF NOT EXISTS idx_failover_status_func ON failover_status(function_name);
COMMENT ON TABLE failover_status IS '主备隔离实时状态表';
COMMENT ON COLUMN failover_status.function_name IS '功能名：database / cache';
COMMENT ON COLUMN failover_status.current_state IS '当前状态：primary（主调用中）/ backup（备用中）/ both_down（双不可用）';
COMMENT ON COLUMN failover_status.circuit_state IS '熔断器状态：closed（关闭）/ open（打开）/ half_open（半开）';

-- 2. 切换事件流水表：记录每次主备切换、熔断、回切事件
CREATE TABLE IF NOT EXISTS failover_event (
    id BIGSERIAL PRIMARY KEY,
    function_name VARCHAR(100) NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    from_state VARCHAR(20),
    to_state VARCHAR(20),
    reason TEXT,
    latency_ms INTEGER,
    tenant_id BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_failover_event_type CHECK (event_type IN ('switch_to_backup', 'switch_back', 'primary_recovered', 'both_failed', 'circuit_open', 'circuit_close', 'circuit_half_open'))
);

CREATE INDEX IF NOT EXISTS idx_failover_event_func_time ON failover_event(function_name, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_failover_event_type ON failover_event(event_type);
CREATE INDEX IF NOT EXISTS idx_failover_event_tenant ON failover_event(tenant_id);
COMMENT ON TABLE failover_event IS '主备隔离切换事件流水';
COMMENT ON COLUMN failover_event.event_type IS '事件类型：switch_to_backup/switch_back/primary_recovered/both_failed/circuit_open/circuit_close/circuit_half_open';

-- 3. 配置持久化表：将动态配置持久化（运行时可调整）
CREATE TABLE IF NOT EXISTS failover_config (
    id BIGSERIAL PRIMARY KEY,
    function_name VARCHAR(100) NOT NULL,
    config_key VARCHAR(200) NOT NULL,
    config_value TEXT NOT NULL,
    description TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(function_name, config_key)
);

CREATE INDEX IF NOT EXISTS idx_failover_config_func ON failover_config(function_name) WHERE is_active = TRUE;
COMMENT ON TABLE failover_config IS '主备隔离配置持久化';

-- 4. 初始化数据：插入默认主备状态
INSERT INTO failover_status (function_name, current_state, circuit_state, backup_type)
VALUES
    ('database', 'primary', 'closed', 'postgres'),
    ('cache', 'primary', 'closed', 'lru')
ON CONFLICT (function_name) DO NOTHING;"#,
    ),
    // 20260617 系列（定制订单/工艺/质量/售后/色卡/AI/维度表）
    (
        "20260617000001_create_custom_orders",
        r#"-- 定制订单全流程跟踪模块 migration
-- 创建 5 张核心表：custom_orders / process_nodes / process_logs / quality_issues / after_sales
-- 创建时间: 2026-06-17
-- 关联 spec: docs/superpowers/specs/2026-06-16-custom-order-design.md

-- 1. 定制订单主表：记录定制订单基础信息和 5 阶段工艺状态
CREATE TABLE IF NOT EXISTS "custom_orders" (
    "id" BIGSERIAL PRIMARY KEY,
    "order_no" VARCHAR(50) UNIQUE NOT NULL,
    "customer_id" BIGINT NOT NULL REFERENCES "customers"("id"),
    "product_id" BIGINT NOT NULL REFERENCES "products"("id"),
    "color_id" BIGINT REFERENCES "product_colors"("id"),
    "spec" VARCHAR(200) NOT NULL,
    "quantity" DECIMAL(18,2) NOT NULL CHECK ("quantity" > 0),
    "unit" VARCHAR(20) NOT NULL DEFAULT 'm',
    "custom_requirements" JSONB NOT NULL DEFAULT '{}'::jsonb,
    "yarn_spec" VARCHAR(200),
    "dye_method" VARCHAR(50),
    "finishing_method" VARCHAR(50),
    "status" VARCHAR(30) NOT NULL DEFAULT 'draft',
    "expected_delivery_date" DATE,
    "actual_delivery_date" DATE,
    "sales_order_id" BIGINT REFERENCES "sales_orders"("id"),
    "total_amount" DECIMAL(18,2),
    "currency" VARCHAR(10) NOT NULL DEFAULT 'CNY',
    "tenant_id" BIGINT NOT NULL,
    "notes" TEXT,
    "created_by" BIGINT,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "chk_custom_order_status" CHECK ("status" IN (
        'draft', 'yarn_purchasing', 'dyeing', 'finishing',
        'delivery', 'after_sales', 'completed', 'cancelled'
    ))
);

CREATE INDEX IF NOT EXISTS "idx_custom_orders_tenant" ON "custom_orders"("tenant_id");
CREATE INDEX IF NOT EXISTS "idx_custom_orders_customer" ON "custom_orders"("customer_id");
CREATE INDEX IF NOT EXISTS "idx_custom_orders_status" ON "custom_orders"("status");
CREATE INDEX IF NOT EXISTS "idx_custom_orders_sales_order" ON "custom_orders"("sales_order_id");

COMMENT ON TABLE "custom_orders" IS '定制订单主表 - 客户特殊定制订单跟踪';
COMMENT ON COLUMN "custom_orders"."status" IS '订单状态：draft(草稿) / yarn_purchasing(纱线采购) / dyeing(染整) / finishing(后整理) / delivery(交付) / after_sales(售后) / completed(已完成) / cancelled(已取消)';
COMMENT ON COLUMN "custom_orders"."custom_requirements" IS '客户定制要求（特殊工艺、克重、幅宽等）JSONB';
COMMENT ON COLUMN "custom_orders"."yarn_spec" IS '指定纱线规格';
COMMENT ON COLUMN "custom_orders"."dye_method" IS '染色工艺方法';
COMMENT ON COLUMN "custom_orders"."finishing_method" IS '后整理工艺方法';
COMMENT ON COLUMN "custom_orders"."notes" IS '订单备注（批次 88 PH-1，与 m0032 幂等对齐）';"#,
    ),
    (
        "20260617000002_create_process_nodes",
        r#"-- 工艺节点表：5 阶段工艺节点（纱线采购/染整/后整理/交付/售后）
CREATE TABLE IF NOT EXISTS "process_nodes" (
    "id" BIGSERIAL PRIMARY KEY,
    "custom_order_id" BIGINT NOT NULL REFERENCES "custom_orders"("id") ON DELETE CASCADE,
    "node_type" VARCHAR(30) NOT NULL,
    "node_name" VARCHAR(100) NOT NULL,
    "sequence" INTEGER NOT NULL,
    "status" VARCHAR(20) NOT NULL DEFAULT 'pending',
    "planned_start_date" TIMESTAMPTZ,
    "planned_end_date" TIMESTAMPTZ,
    "actual_start_date" TIMESTAMPTZ,
    "actual_end_date" TIMESTAMPTZ,
    "operator_id" BIGINT REFERENCES "users"("id"),
    "notes" TEXT,
    "tenant_id" BIGINT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "chk_node_type" CHECK ("node_type" IN (
        'yarn_purchasing', 'dyeing', 'finishing', 'delivery', 'after_sales'
    )),
    CONSTRAINT "chk_node_status" CHECK ("status" IN (
        'pending', 'in_progress', 'completed', 'blocked'
    ))
);

CREATE INDEX IF NOT EXISTS "idx_process_nodes_order" ON "process_nodes"("custom_order_id");
CREATE INDEX IF NOT EXISTS "idx_process_nodes_status" ON "process_nodes"("status");
CREATE INDEX IF NOT EXISTS "idx_process_nodes_tenant" ON "process_nodes"("tenant_id");

COMMENT ON TABLE "process_nodes" IS '定制订单工艺节点表';
COMMENT ON COLUMN "process_nodes"."node_type" IS '节点类型：yarn_purchasing(纱线采购) / dyeing(染整) / finishing(后整理) / delivery(交付) / after_sales(售后)';
COMMENT ON COLUMN "process_nodes"."status" IS '节点状态：pending(待开始) / in_progress(进行中) / completed(已完成) / blocked(阻塞)';"#,
    ),
    (
        "20260617000003_create_process_logs",
        r#"-- 流程日志表：记录节点操作日志（时间戳/操作人/前后状态/附件）
CREATE TABLE IF NOT EXISTS "process_logs" (
    "id" BIGSERIAL PRIMARY KEY,
    "process_node_id" BIGINT NOT NULL REFERENCES "process_nodes"("id") ON DELETE CASCADE,
    "action" VARCHAR(50) NOT NULL,
    "operator_id" BIGINT REFERENCES "users"("id"),
    "before_status" VARCHAR(20),
    "after_status" VARCHAR(20),
    "log_time" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "log_content" TEXT,
    "attachments" JSONB NOT NULL DEFAULT '[]'::jsonb,
    "tenant_id" BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS "idx_process_logs_node" ON "process_logs"("process_node_id");
CREATE INDEX IF NOT EXISTS "idx_process_logs_time" ON "process_logs"("log_time" DESC);
CREATE INDEX IF NOT EXISTS "idx_process_logs_tenant" ON "process_logs"("tenant_id");

COMMENT ON TABLE "process_logs" IS '定制订单工艺节点操作日志表';
COMMENT ON COLUMN "process_logs"."action" IS '操作类型：start/pause/resume/complete/block/unblock';
COMMENT ON COLUMN "process_logs"."attachments" IS '操作附件 URL 列表 JSONB';"#,
    ),
    (
        "20260617000004_create_quality_issues",
        r#"-- 质量异常表：记录色差、色牢度等质量问题
CREATE TABLE IF NOT EXISTS "quality_issues" (
    "id" BIGSERIAL PRIMARY KEY,
    "custom_order_id" BIGINT NOT NULL REFERENCES "custom_orders"("id") ON DELETE CASCADE,
    "process_node_id" BIGINT REFERENCES "process_nodes"("id"),
    "issue_type" VARCHAR(50) NOT NULL,
    "severity" VARCHAR(20) NOT NULL DEFAULT 'medium',
    "description" TEXT NOT NULL,
    "discovered_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "resolved_at" TIMESTAMPTZ,
    "resolution" TEXT,
    "status" VARCHAR(20) NOT NULL DEFAULT 'open',
    "tenant_id" BIGINT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "chk_issue_severity" CHECK ("severity" IN ('low', 'medium', 'high', 'critical')),
    CONSTRAINT "chk_issue_status" CHECK ("status" IN ('open', 'investigating', 'resolved', 'closed'))
);

CREATE INDEX IF NOT EXISTS "idx_quality_issues_order" ON "quality_issues"("custom_order_id");
CREATE INDEX IF NOT EXISTS "idx_quality_issues_status" ON "quality_issues"("status");
CREATE INDEX IF NOT EXISTS "idx_quality_issues_tenant" ON "quality_issues"("tenant_id");

COMMENT ON TABLE "quality_issues" IS '定制订单质量异常表';
COMMENT ON COLUMN "quality_issues"."issue_type" IS '异常类型：color_diff(色差) / color_fastness(色牢度) / spec(规格不符) / damage(破损) / other';
COMMENT ON COLUMN "quality_issues"."severity" IS '严重度：low(低) / medium(中) / high(高) / critical(严重)';
COMMENT ON COLUMN "quality_issues"."status" IS '状态：open(待处理) / investigating(调查中) / resolved(已解决) / closed(已关闭)';"#,
    ),
    (
        "20260617000005_create_after_sales",
        r#"-- 售后工单表：4 种类型（客诉/维修/换货/退款）
CREATE TABLE IF NOT EXISTS "after_sales" (
    "id" BIGSERIAL PRIMARY KEY,
    "custom_order_id" BIGINT NOT NULL REFERENCES "custom_orders"("id"),
    "issue_type" VARCHAR(30) NOT NULL,
    "customer_id" BIGINT NOT NULL REFERENCES "customers"("id"),
    "description" TEXT NOT NULL,
    "status" VARCHAR(20) NOT NULL DEFAULT 'opened',
    "opened_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "closed_at" TIMESTAMPTZ,
    "resolution" TEXT,
    "refund_amount" DECIMAL(18,2),
    "tenant_id" BIGINT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "chk_aftersales_type" CHECK ("issue_type" IN (
        'complaint', 'repair', 'exchange', 'refund'
    )),
    CONSTRAINT "chk_aftersales_status" CHECK ("status" IN (
        'opened', 'processing', 'resolved', 'closed', 'rejected'
    ))
);

CREATE INDEX IF NOT EXISTS "idx_aftersales_order" ON "after_sales"("custom_order_id");
CREATE INDEX IF NOT EXISTS "idx_aftersales_customer" ON "after_sales"("customer_id");
CREATE INDEX IF NOT EXISTS "idx_aftersales_status" ON "after_sales"("status");
CREATE INDEX IF NOT EXISTS "idx_aftersales_tenant" ON "after_sales"("tenant_id");

COMMENT ON TABLE "after_sales" IS '定制订单售后工单表';
COMMENT ON COLUMN "after_sales"."issue_type" IS '售后类型：complaint(客诉) / repair(维修) / exchange(换货) / refund(退款)';
COMMENT ON COLUMN "after_sales"."status" IS '状态：opened(已开) / processing(处理中) / resolved(已解决) / closed(已关闭) / rejected(已拒绝)';"#,
    ),
    (
        "20260617000006_create_color_cards",
        r#"-- 色卡仓储管理模块 migration - color_cards 表
-- 创建时间: 2026-06-17
-- 关联 spec: docs/superpowers/specs/2026-06-16-color-card-design.md §3.2

-- 色卡主表：色卡基本信息和生命周期状态
CREATE TABLE IF NOT EXISTS "color_cards" (
    "id" BIGSERIAL PRIMARY KEY,
    "card_no" VARCHAR(50) UNIQUE NOT NULL,
    "card_name" VARCHAR(200) NOT NULL,
    "card_type" VARCHAR(50) NOT NULL DEFAULT 'CUSTOM',
    "season" VARCHAR(20),
    "brand" VARCHAR(100),
    "total_colors" INT NOT NULL DEFAULT 0,
    "status" VARCHAR(20) NOT NULL DEFAULT 'active',
    "description" TEXT,
    "cover_image_url" TEXT,
    "tenant_id" BIGINT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "chk_color_card_type" CHECK ("card_type" IN ('PANTONE', 'CNCS', 'CUSTOM')),
    CONSTRAINT "chk_color_card_status" CHECK ("status" IN ('active', 'archived', 'lost'))
);

-- 索引
CREATE INDEX IF NOT EXISTS "idx_color_cards_tenant" ON "color_cards"("tenant_id");
CREATE INDEX IF NOT EXISTS "idx_color_cards_status" ON "color_cards"("status");
CREATE INDEX IF NOT EXISTS "idx_color_cards_type_season" ON "color_cards"("card_type", "season");

COMMENT ON TABLE "color_cards" IS '色卡主表 - 纺织行业色卡生命周期与借出跟踪';
COMMENT ON COLUMN "color_cards"."card_no" IS '色卡编号，如 PANTONE-TPX-2024-SS';
COMMENT ON COLUMN "color_cards"."card_type" IS '色卡类型：PANTONE / CNCS / CUSTOM';
COMMENT ON COLUMN "color_cards"."season" IS '季节标签：2024SS / 2024AW / 经典';
COMMENT ON COLUMN "color_cards"."status" IS '状态：active(在用) / archived(归档) / lost(遗失)';"#,
    ),
    (
        "20260617000007_create_color_card_items",
        r#"-- 色卡仓储管理模块 migration - color_card_items 表
-- 创建时间: 2026-06-17
-- 关联 spec: docs/superpowers/specs/2026-06-16-color-card-design.md §3.3

-- 色卡明细表：每个色号的色彩空间坐标、配方关联、价格关联
CREATE TABLE IF NOT EXISTS "color_card_items" (
    "id" BIGSERIAL PRIMARY KEY,
    "color_card_id" BIGINT NOT NULL REFERENCES "color_cards"("id") ON DELETE CASCADE,
    "color_code" VARCHAR(50) NOT NULL,
    "color_name" VARCHAR(200) NOT NULL,
    "rgb_r" INT NOT NULL CHECK ("rgb_r" BETWEEN 0 AND 255),
    "rgb_g" INT NOT NULL CHECK ("rgb_g" BETWEEN 0 AND 255),
    "rgb_b" INT NOT NULL CHECK ("rgb_b" BETWEEN 0 AND 255),
    "cmyk_c" DECIMAL(5,2) CHECK ("cmyk_c" IS NULL OR ("cmyk_c" BETWEEN 0 AND 100)),
    "cmyk_m" DECIMAL(5,2) CHECK ("cmyk_m" IS NULL OR ("cmyk_m" BETWEEN 0 AND 100)),
    "cmyk_y" DECIMAL(5,2) CHECK ("cmyk_y" IS NULL OR ("cmyk_y" BETWEEN 0 AND 100)),
    "cmyk_k" DECIMAL(5,2) CHECK ("cmyk_k" IS NULL OR ("cmyk_k" BETWEEN 0 AND 100)),
    "lab_l" DECIMAL(6,2) CHECK ("lab_l" IS NULL OR ("lab_l" BETWEEN 0 AND 100)),
    "lab_a" DECIMAL(6,2) CHECK ("lab_a" IS NULL OR ("lab_a" BETWEEN -128 AND 127)),
    "lab_b" DECIMAL(6,2) CHECK ("lab_b" IS NULL OR ("lab_b" BETWEEN -128 AND 127)),
    "pantone_code" VARCHAR(50),
    "cncs_code" VARCHAR(50),
    "custom_code" VARCHAR(50),
    "hex_value" VARCHAR(7) NOT NULL,
    "dye_recipe_id" INTEGER REFERENCES "dye_recipe"("id"),
    "product_color_price_id" BIGINT REFERENCES "product_color_prices"("id"),
    "swatch_image_url" TEXT,
    "sequence" INT NOT NULL DEFAULT 0,
    "tenant_id" BIGINT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "uq_color_card_items_card_code" UNIQUE ("color_card_id", "color_code")
);

-- 索引
CREATE INDEX IF NOT EXISTS "idx_color_items_card" ON "color_card_items"("color_card_id");
CREATE INDEX IF NOT EXISTS "idx_color_items_code" ON "color_card_items"("color_code");
CREATE INDEX IF NOT EXISTS "idx_color_items_pantone" ON "color_card_items"("pantone_code") WHERE "pantone_code" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_color_items_cncs" ON "color_card_items"("cncs_code") WHERE "cncs_code" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_color_items_tenant" ON "color_card_items"("tenant_id");
CREATE INDEX IF NOT EXISTS "idx_color_items_dye_recipe" ON "color_card_items"("dye_recipe_id");
CREATE INDEX IF NOT EXISTS "idx_color_items_price" ON "color_card_items"("product_color_price_id");

COMMENT ON TABLE "color_card_items" IS '色卡明细表 - 纺织行业色号详细参数与关联业务';
COMMENT ON COLUMN "color_card_items"."hex_value" IS 'HEX 颜色值 #RRGGBB';
COMMENT ON COLUMN "color_card_items"."lab_l" IS 'CIELab 颜色空间 L (亮度 0-100)';
COMMENT ON COLUMN "color_card_items"."lab_a" IS 'CIELab 颜色空间 a (红绿 -128~127)';
COMMENT ON COLUMN "color_card_items"."lab_b" IS 'CIELab 颜色空间 b (黄蓝 -128~127)';
COMMENT ON COLUMN "color_card_items"."sequence" IS '色卡中色号的显示顺序';"#,
    ),
    (
        "20260617000008_create_color_card_borrow_records",
        r#"-- 色卡仓储管理模块 migration - color_card_borrow_records 表
-- 创建时间: 2026-06-17
-- 关联 spec: docs/superpowers/specs/2026-06-16-color-card-design.md §3.4
-- 注意：tenant_id 字段已由 m0028_drop_tenant_columns 迁移删除，此文件保留历史记录用途

-- 色卡借出记录表：跟踪色卡借出/归还/遗失的全生命周期
CREATE TABLE IF NOT EXISTS "color_card_borrow_records" (
    "id" BIGSERIAL PRIMARY KEY,
    "color_card_id" BIGINT NOT NULL REFERENCES "color_cards"("id") ON DELETE RESTRICT,
    "customer_id" BIGINT NOT NULL REFERENCES "customers"("id") ON DELETE RESTRICT,
    "borrowed_by" BIGINT NOT NULL REFERENCES "users"("id") ON DELETE RESTRICT,
    "borrowed_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "expected_return_at" TIMESTAMPTZ,
    "actual_return_at" TIMESTAMPTZ,
    "status" VARCHAR(20) NOT NULL DEFAULT 'borrowed',
    "purpose" TEXT,
    "notes" TEXT,
    "compensation_amount" DECIMAL(15,2) CHECK ("compensation_amount" IS NULL OR "compensation_amount" >= 0),
    "tenant_id" BIGINT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "chk_borrow_status" CHECK ("status" IN ('borrowed', 'returned', 'lost', 'damaged'))
);

-- 索引
CREATE INDEX IF NOT EXISTS "idx_borrow_card" ON "color_card_borrow_records"("color_card_id");
CREATE INDEX IF NOT EXISTS "idx_borrow_customer" ON "color_card_borrow_records"("customer_id");
CREATE INDEX IF NOT EXISTS "idx_borrow_status" ON "color_card_borrow_records"("status");
CREATE INDEX IF NOT EXISTS "idx_borrow_tenant" ON "color_card_borrow_records"("tenant_id");
CREATE INDEX IF NOT EXISTS "idx_borrow_borrowed_at" ON "color_card_borrow_records"("borrowed_at" DESC);
CREATE INDEX IF NOT EXISTS "idx_borrow_borrower" ON "color_card_borrow_records"("borrowed_by");

COMMENT ON TABLE "color_card_borrow_records" IS '色卡借出记录 - 色卡借出/归还/遗失的全生命周期跟踪';
COMMENT ON COLUMN "color_card_borrow_records"."status" IS '借出状态：borrowed(借出中) / returned(已归还) / lost(遗失) / damaged(损坏)';
COMMENT ON COLUMN "color_card_borrow_records"."compensation_amount" IS '遗失/损坏赔付金额（CNY）';
COMMENT ON COLUMN "color_card_borrow_records"."borrowed_by" IS '经办员工 ID（关联 users 表）';"#,
    ),
    (
        "20260617000009_create_ai_process_optimizations",
        r#"-- AI 工艺优化历史表 - P2-4 AI 分析深化
-- 持久化 AI 染色工艺优化推荐结果，支持历史追溯与应用率统计
-- 创建时间: 2026-06-17
-- 关联 spec: doto.md P2-4 任务清单

CREATE TABLE IF NOT EXISTS "ai_process_optimizations" (
    "id" BIGSERIAL PRIMARY KEY,
    "request_id" VARCHAR(64) NOT NULL UNIQUE,
    "color_no" VARCHAR(64) NOT NULL,
    "color_name" VARCHAR(128),
    "fabric_type" VARCHAR(64) NOT NULL,
    "dye_type" VARCHAR(64),
    "recommended_temperature" DECIMAL(5,2) NOT NULL,
    "recommended_time_minutes" INTEGER NOT NULL,
    "recommended_ph_value" DECIMAL(4,2) NOT NULL,
    "recommended_liquor_ratio" DECIMAL(5,2) NOT NULL,
    "similar_cases" INTEGER NOT NULL DEFAULT 0,
    "confidence" DECIMAL(4,3) NOT NULL,
    "source" VARCHAR(16) NOT NULL,
    "reason" TEXT,
    "candidates_json" JSONB,
    "is_applied" BOOLEAN NOT NULL DEFAULT false,
    "applied_at" TIMESTAMPTZ,
    "applied_by" BIGINT REFERENCES "users"("id"),
    "feedback_score" SMALLINT,
    "feedback_remark" TEXT,
    "tenant_id" BIGINT NOT NULL,
    "created_by" BIGINT REFERENCES "users"("id"),
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "chk_ai_proc_source" CHECK ("source" IN ('knn', 'fallback')),
    CONSTRAINT "chk_ai_proc_confidence" CHECK ("confidence" >= 0.0 AND "confidence" <= 1.0),
    CONSTRAINT "chk_ai_proc_feedback" CHECK ("feedback_score" IS NULL OR ("feedback_score" >= 1 AND "feedback_score" <= 5))
);

-- 索引：列表查询（按租户 + 创建时间倒序）
CREATE INDEX IF NOT EXISTS "idx_ai_proc_tenant_created" ON "ai_process_optimizations"("tenant_id", "created_at" DESC);
-- 索引：按色号 + 布类查询历史推荐
CREATE INDEX IF NOT EXISTS "idx_ai_proc_color_fabric" ON "ai_process_optimizations"("tenant_id", "color_no", "fabric_type");
-- 索引：按应用状态统计应用率
CREATE INDEX IF NOT EXISTS "idx_ai_proc_applied" ON "ai_process_optimizations"("tenant_id", "is_applied");
-- 索引：按 k-NN / fallback 来源统计
CREATE INDEX IF NOT EXISTS "idx_ai_proc_source" ON "ai_process_optimizations"("tenant_id", "source");

-- 注释
COMMENT ON TABLE "ai_process_optimizations" IS 'AI 工艺优化历史表（P2-4）：持久化 k-NN 染色工艺推荐 + 应用反馈';
COMMENT ON COLUMN "ai_process_optimizations"."request_id" IS '请求唯一 ID（UUID），用于幂等防重';
COMMENT ON COLUMN "ai_process_optimizations"."similar_cases" IS '命中相似历史配方数（k-NN 路径）';
COMMENT ON COLUMN "ai_process_optimizations"."confidence" IS '推荐置信度（0.0-1.0）';
COMMENT ON COLUMN "ai_process_optimizations"."source" IS '推荐来源：knn（k-NN 加权）/ fallback（典型参数表）';
COMMENT ON COLUMN "ai_process_optimizations"."candidates_json" IS '候选案例 JSON（最多 10 条）';
COMMENT ON COLUMN "ai_process_optimizations"."is_applied" IS '是否已被工艺员采纳并下发生产';
COMMENT ON COLUMN "ai_process_optimizations"."feedback_score" IS '采纳后质量反馈（1-5 星，null=未反馈）';"#,
    ),
    (
        "20260617000010_create_ai_quality_predictions",
        r#"-- AI 质量预测历史表 - P2-4 AI 分析深化
-- 持久化 AI 质量预测结果（风险评分 / 趋势 / 问题归因），支持历史回溯与质量看板
-- 创建时间: 2026-06-17
-- 关联 spec: doto.md P2-4 任务清单

CREATE TABLE IF NOT EXISTS "ai_quality_predictions" (
    "id" BIGSERIAL PRIMARY KEY,
    "request_id" VARCHAR(64) NOT NULL UNIQUE,
    "product_id" BIGINT REFERENCES "products"("id"),
    "inspection_type" VARCHAR(32) NOT NULL DEFAULT 'all',
    "window_days" INTEGER NOT NULL DEFAULT 90,
    "total_inspections" BIGINT NOT NULL DEFAULT 0,
    "avg_qualification_rate" DECIMAL(5,2) NOT NULL,
    "trend" VARCHAR(16) NOT NULL,
    "trend_rate" DECIMAL(6,3) NOT NULL,
    "risk_score" SMALLINT NOT NULL,
    "risk_level" VARCHAR(8) NOT NULL,
    "confidence" DECIMAL(4,3) NOT NULL,
    "top_issues_json" JSONB,
    "recommendations_json" JSONB,
    "period_breakdown_json" JSONB,
    "source" VARCHAR(16) NOT NULL,
    "is_acknowledged" BOOLEAN NOT NULL DEFAULT false,
    "acknowledged_at" TIMESTAMPTZ,
    "acknowledged_by" BIGINT REFERENCES "users"("id"),
    "tenant_id" BIGINT NOT NULL,
    "created_by" BIGINT REFERENCES "users"("id"),
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "chk_ai_qual_type" CHECK ("inspection_type" IN ('all', 'incoming', 'inprocess', 'final', 'outgoing')),
    CONSTRAINT "chk_ai_qual_trend" CHECK ("trend" IN ('up', 'flat', 'down', 'nodata')),
    CONSTRAINT "chk_ai_qual_level" CHECK ("risk_level" IN ('low', 'medium', 'high')),
    CONSTRAINT "chk_ai_qual_source" CHECK ("source" IN ('history', 'fallback')),
    CONSTRAINT "chk_ai_qual_risk" CHECK ("risk_score" >= 0 AND "risk_score" <= 100),
    CONSTRAINT "chk_ai_qual_confidence" CHECK ("confidence" >= 0.0 AND "confidence" <= 1.0),
    CONSTRAINT "chk_ai_qual_window" CHECK ("window_days" >= 1 AND "window_days" <= 365)
);

-- 索引：列表查询（按租户 + 创建时间倒序）
CREATE INDEX IF NOT EXISTS "idx_ai_qual_tenant_created" ON "ai_quality_predictions"("tenant_id", "created_at" DESC);
-- 索引：按产品查询历史预测
CREATE INDEX IF NOT EXISTS "idx_ai_qual_product" ON "ai_quality_predictions"("tenant_id", "product_id", "created_at" DESC);
-- 索引：按风险等级（看板）
CREATE INDEX IF NOT EXISTS "idx_ai_qual_risk" ON "ai_quality_predictions"("tenant_id", "risk_level", "created_at" DESC);
-- 索引：按确认状态
CREATE INDEX IF NOT EXISTS "idx_ai_qual_ack" ON "ai_quality_predictions"("tenant_id", "is_acknowledged");

-- 注释
COMMENT ON TABLE "ai_quality_predictions" IS 'AI 质量预测历史表（P2-4）：持久化质量风险评分 / 趋势 / 归因 / 建议';
COMMENT ON COLUMN "ai_quality_predictions"."trend" IS '趋势：up（上升）/ flat（平稳）/ down（下降）/ nodata（无数据）';
COMMENT ON COLUMN "ai_quality_predictions"."trend_rate" IS '趋势变化率（百分点，例 +12.5 表示合格率上升 12.5 个百分点）';
COMMENT ON COLUMN "ai_quality_predictions"."risk_score" IS '风险评分 0-100，越高越危险';
COMMENT ON COLUMN "ai_quality_predictions"."risk_level" IS '风险等级：low（低）/ medium（中）/ high（高）';
COMMENT ON COLUMN "ai_quality_predictions"."top_issues_json" IS '主要问题归因 JSON（top 3：颜色差异 / 色牢度 / 克重偏差 / 纬密偏差 / 强度不足 / 其他）';
COMMENT ON COLUMN "ai_quality_predictions"."recommendations_json" IS '建议措施 JSON（按风险等级 1-3 条）';
COMMENT ON COLUMN "ai_quality_predictions"."period_breakdown_json" IS '按月分段统计 JSON（period / inspections / avg_qualification_rate）';
COMMENT ON COLUMN "ai_quality_predictions"."source" IS '预测来源：history（≥ 5 条历史）/ fallback（< 5 条保守兜底）';
COMMENT ON COLUMN "ai_quality_predictions"."is_acknowledged" IS '是否已被质量管理员确认查看';"#,
    ),
    (
        "20260617000011_create_sales_facts",
        r#"-- P3-4 BI 数据仓库：销售事实表
-- 多租户隔离：tenant_id 必填
-- 索引：tenant_id + order_date 倒序（按时间分析）

CREATE TABLE IF NOT EXISTS sales_facts (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    order_id BIGINT NOT NULL,
    order_date DATE NOT NULL,
    customer_id BIGINT NOT NULL,
    product_id BIGINT NOT NULL,
    region_id BIGINT,
    quantity NUMERIC(18, 4) NOT NULL,
    unit_price NUMERIC(18, 4) NOT NULL,
    total_amount NUMERIC(18, 4) NOT NULL,
    cost_amount NUMERIC(18, 4) NOT NULL,
    profit_amount NUMERIC(18, 4) NOT NULL,
    status VARCHAR(20) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 多租户 + 时间倒序联合索引（按时间分析）
CREATE INDEX IF NOT EXISTS idx_sales_facts_tenant_date
    ON sales_facts (tenant_id, order_date DESC);

-- 多租户 + 客户（按客户分析）
CREATE INDEX IF NOT EXISTS idx_sales_facts_tenant_customer
    ON sales_facts (tenant_id, customer_id, order_date DESC);

-- 多租户 + 产品（按产品分析）
CREATE INDEX IF NOT EXISTS idx_sales_facts_tenant_product
    ON sales_facts (tenant_id, product_id, order_date DESC);

-- 多租户 + 区域（按区域分析）
CREATE INDEX IF NOT EXISTS idx_sales_facts_tenant_region
    ON sales_facts (tenant_id, region_id, order_date DESC);

COMMENT ON TABLE sales_facts IS 'P3-4 BI 数据仓库：销售事实表（Star Schema fact table）';
COMMENT ON COLUMN sales_facts.tenant_id IS '租户 ID（多租户隔离强制字段）';
COMMENT ON COLUMN sales_facts.total_amount IS '销售额（quantity * unit_price）';
COMMENT ON COLUMN sales_facts.profit_amount IS '利润（total - cost）';"#,
    ),
    (
        "20260617000012_create_dim_products",
        r#"-- P3-4 BI 数据仓库：产品维表（SCD Type 2）
-- 保留历史版本：valid_from / valid_to / is_current
-- 多租户隔离：tenant_id 必填

CREATE TABLE IF NOT EXISTS dim_products (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    product_id BIGINT NOT NULL,
    product_code VARCHAR(50) NOT NULL,
    product_name VARCHAR(255) NOT NULL,
    category VARCHAR(100),
    color_no VARCHAR(50),
    fabric_type VARCHAR(50),
    valid_from DATE NOT NULL,
    valid_to DATE NOT NULL DEFAULT '9999-12-31',
    is_current BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 多租户 + 当前版本（业务查询）
CREATE INDEX IF NOT EXISTS idx_dim_products_tenant_current
    ON dim_products (tenant_id, product_id) WHERE is_current = true;

-- 多租户 + 时间范围（历史查询）
CREATE INDEX IF NOT EXISTS idx_dim_products_tenant_history
    ON dim_products (tenant_id, product_id, valid_from, valid_to);

-- 多租户 + 品类（按品类分析）
CREATE INDEX IF NOT EXISTS idx_dim_products_tenant_category
    ON dim_products (tenant_id, category) WHERE is_current = true;

COMMENT ON TABLE dim_products IS 'P3-4 BI 产品维表（SCD Type 2）';
COMMENT ON COLUMN dim_products.is_current IS '是否当前版本（SCD Type 2）';"#,
    ),
    (
        "20260617000013_create_dim_customers",
        r#"-- P3-4 BI 数据仓库：客户维表（SCD Type 2）
-- 保留历史版本：valid_from / valid_to / is_current

CREATE TABLE IF NOT EXISTS dim_customers (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    customer_id BIGINT NOT NULL,
    customer_code VARCHAR(50) NOT NULL,
    customer_name VARCHAR(255) NOT NULL,
    customer_type VARCHAR(50),
    region VARCHAR(100),
    industry VARCHAR(100),
    valid_from DATE NOT NULL,
    valid_to DATE NOT NULL DEFAULT '9999-12-31',
    is_current BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 多租户 + 当前版本
CREATE INDEX IF NOT EXISTS idx_dim_customers_tenant_current
    ON dim_customers (tenant_id, customer_id) WHERE is_current = true;

-- 多租户 + 区域（按区域分析）
CREATE INDEX IF NOT EXISTS idx_dim_customers_tenant_region
    ON dim_customers (tenant_id, region) WHERE is_current = true;

-- 多租户 + 客户类型
CREATE INDEX IF NOT EXISTS idx_dim_customers_tenant_type
    ON dim_customers (tenant_id, customer_type) WHERE is_current = true;

COMMENT ON TABLE dim_customers IS 'P3-4 BI 客户维表（SCD Type 2）';"#,
    ),
    (
        "20260617000014_create_dim_dates",
        r#"-- P3-4 BI 数据仓库：日期维表
-- 标准日期维度（年/季/月/周/日 + 是否周末/节假日 + 财年）

CREATE TABLE IF NOT EXISTS dim_dates (
    id BIGSERIAL PRIMARY KEY,
    date DATE NOT NULL UNIQUE,
    year SMALLINT NOT NULL,
    quarter SMALLINT NOT NULL,
    month SMALLINT NOT NULL,
    week SMALLINT NOT NULL,
    day_of_week SMALLINT NOT NULL,
    is_weekend BOOLEAN NOT NULL,
    is_holiday BOOLEAN NOT NULL DEFAULT false,
    fiscal_year SMALLINT,
    fiscal_quarter SMALLINT
);

CREATE INDEX IF NOT EXISTS idx_dim_dates_year_month ON dim_dates (year, month);
CREATE INDEX IF NOT EXISTS idx_dim_dates_year ON dim_dates (year);
CREATE INDEX IF NOT EXISTS idx_dim_dates_quarter ON dim_dates (year, quarter);

COMMENT ON TABLE dim_dates IS 'P3-4 BI 日期维表';
COMMENT ON COLUMN dim_dates.day_of_week IS '1=周一，7=周日';"#,
    ),
    // 20260618 系列（增强版销售报价/色价历史/梯度/客户色价/季节性价格）
    (
        "20260618000001_create_sales_quotations",
        r#"-- 销售报价单主表
-- 用于存储销售报价单的核心业务信息（Incoterms 2020 + 多币种 + 状态机）
-- 创建时间: 2026-06-18
-- 关联计划: 2026-06-17-p12-batch1-quotation-port-plan.md PR-1
-- main 适配说明：
--   - ID 由 BIGSERIAL 调整为 SERIAL（i32），与 main 已有 sales_order / sales_fabric_order 主键类型保持一致
--   - 引用 main 现有表的外键列使用 INTEGER，与 customers.id / users.id / sales_orders.id 类型一致
--   - 枚举状态按任务规范：DRAFT / SUBMITTED / APPROVED / REJECTED / CONVERTED / CANCELLED / EXPIRED

CREATE TABLE IF NOT EXISTS "sales_quotations" (
    "id" SERIAL PRIMARY KEY,
    "quotation_no" VARCHAR(50) UNIQUE NOT NULL,
    "customer_id" INTEGER NOT NULL REFERENCES "customers"("id"),
    "sales_user_id" INTEGER NOT NULL REFERENCES "users"("id"),
    "quotation_date" DATE NOT NULL,
    "valid_until" DATE NOT NULL,

    -- 货币
    "currency" VARCHAR(10) NOT NULL DEFAULT 'CNY',
    "exchange_rate" DECIMAL(18,6) NOT NULL DEFAULT 1.0,
    "base_currency" VARCHAR(10) NOT NULL DEFAULT 'CNY',

    -- 价格条款（Incoterms 2020）
    "price_terms" VARCHAR(20) NOT NULL,
    "incoterms_version" VARCHAR(20) DEFAULT '2020',
    "incoterm_location" VARCHAR(200),

    -- 税务
    "tax_inclusive" BOOLEAN NOT NULL DEFAULT TRUE,
    "tax_rate" DECIMAL(5,2) NOT NULL DEFAULT 13.0,

    -- 业务参数
    "moq" DECIMAL(18,2),
    "lead_time_days" INT,
    "customer_level" VARCHAR(20),

    -- 金额
    "subtotal" DECIMAL(18,2) NOT NULL,
    "tax_amount" DECIMAL(18,2) NOT NULL,
    "total_amount" DECIMAL(18,2) NOT NULL,

    -- 状态（DRAFT / SUBMITTED / APPROVED / REJECTED / CONVERTED / CANCELLED / EXPIRED）
    "status" VARCHAR(20) NOT NULL DEFAULT 'DRAFT',

    -- BPM 审批：approval_instance_id 暂不建外键约束（避免阻塞本迁移），后续 PR 通过补充迁移补建
    "approval_instance_id" INTEGER,
    "approved_by" INTEGER REFERENCES "users"("id"),
    "approved_at" TIMESTAMPTZ,
    "rejection_reason" TEXT,

    -- 转换
    "converted_sales_order_id" INTEGER REFERENCES "sales_orders"("id"),
    "converted_at" TIMESTAMPTZ,

    -- 元数据
    "notes" TEXT,
    "created_by" INTEGER NOT NULL REFERENCES "users"("id"),
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT "chk_price_terms" CHECK ("price_terms" IN ('FOB','CIF','EXW','DDP','DAP')),
    CONSTRAINT "chk_quotation_status" CHECK ("status" IN ('DRAFT','SUBMITTED','APPROVED','REJECTED','CONVERTED','CANCELLED','EXPIRED'))
);

CREATE INDEX IF NOT EXISTS "idx_quotations_customer" ON "sales_quotations"("customer_id");
CREATE INDEX IF NOT EXISTS "idx_quotations_status" ON "sales_quotations"("status");
CREATE INDEX IF NOT EXISTS "idx_quotations_valid_until" ON "sales_quotations"("valid_until");
CREATE INDEX IF NOT EXISTS "idx_quotations_sales_user" ON "sales_quotations"("sales_user_id");
CREATE INDEX IF NOT EXISTS "idx_quotations_date" ON "sales_quotations"("quotation_date");

COMMENT ON TABLE "sales_quotations" IS '销售报价单主表 - 销售模块核心，订单前序';
COMMENT ON COLUMN "sales_quotations"."quotation_no" IS '报价单号（唯一）';
COMMENT ON COLUMN "sales_quotations"."customer_id" IS '客户 ID（外键 customers.id）';
COMMENT ON COLUMN "sales_quotations"."sales_user_id" IS '销售员 ID（外键 users.id）';
COMMENT ON COLUMN "sales_quotations"."price_terms" IS '价格条款 - FOB/CIF/EXW/DDP/DAP';
COMMENT ON COLUMN "sales_quotations"."status" IS '状态 - DRAFT/SUBMITTED/APPROVED/REJECTED/CONVERTED/CANCELLED/EXPIRED';"#,
    ),
    (
        "20260618000001_extend_product_color_prices",
        r#"-- 扩展 product_color_prices 表 - P0-5 面料多色号定价扩展
-- 添加字段：阶梯价区间、客户等级、季节、客户专属、优先级、审批等
-- 创建时间: 2026-06-18
-- 关联 spec: docs/superpowers/specs/2026-06-16-color-price-extension-design.md §3.2

-- 添加 max_quantity 阶梯价区间上限
ALTER TABLE "product_color_prices"
    ADD COLUMN IF NOT EXISTS "max_quantity" DECIMAL(18,2);

-- 添加 customer_id 客户专属（NULL = 通用）
ALTER TABLE "product_color_prices"
    ADD COLUMN IF NOT EXISTS "customer_id" BIGINT REFERENCES "customers"("id");

-- 添加 season 季节标签
ALTER TABLE "product_color_prices"
    ADD COLUMN IF NOT EXISTS "season" VARCHAR(10);

-- 添加 is_active 是否启用
ALTER TABLE "product_color_prices"
    ADD COLUMN IF NOT EXISTS "is_active" BOOLEAN NOT NULL DEFAULT true;

-- 添加 priority 优先级（数值大 = 优先级高）
ALTER TABLE "product_color_prices"
    ADD COLUMN IF NOT EXISTS "priority" INT NOT NULL DEFAULT 0;

-- 添加创建人 / 审批人 / 审批时间
ALTER TABLE "product_color_prices"
    ADD COLUMN IF NOT EXISTS "created_by" BIGINT REFERENCES "users"("id");

ALTER TABLE "product_color_prices"
    ADD COLUMN IF NOT EXISTS "approved_by" BIGINT REFERENCES "users"("id");

ALTER TABLE "product_color_prices"
    ADD COLUMN IF NOT EXISTS "approved_at" TIMESTAMPTZ;

-- 添加审批状态
ALTER TABLE "product_color_prices"
    ADD COLUMN IF NOT EXISTS "approval_status" VARCHAR(20) NOT NULL DEFAULT 'APPROVED';

-- 添加 tenant_id
ALTER TABLE "product_color_prices"
    ADD COLUMN IF NOT EXISTS "tenant_id" BIGINT NOT NULL DEFAULT 1;

-- 添加约束（CHECK）
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE constraint_name = 'chk_color_price_approval_status'
          AND table_name = 'product_color_prices'
    ) THEN
        ALTER TABLE "product_color_prices"
            ADD CONSTRAINT "chk_color_price_approval_status"
            CHECK ("approval_status" IN ('PENDING', 'APPROVED', 'REJECTED'));
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE constraint_name = 'chk_color_price_season'
          AND table_name = 'product_color_prices'
    ) THEN
        ALTER TABLE "product_color_prices"
            ADD CONSTRAINT "chk_color_price_season"
            CHECK ("season" IS NULL OR "season" IN ('SS', 'AW', 'HOLIDAY'));
    END IF;
END $$;

-- 添加索引
CREATE INDEX IF NOT EXISTS "idx_color_prices_tenant" ON "product_color_prices"("tenant_id");
CREATE INDEX IF NOT EXISTS "idx_color_prices_customer" ON "product_color_prices"("customer_id");
CREATE INDEX IF NOT EXISTS "idx_color_prices_season" ON "product_color_prices"("season");
CREATE INDEX IF NOT EXISTS "idx_color_prices_active" ON "product_color_prices"("is_active");
CREATE INDEX IF NOT EXISTS "idx_color_prices_approval" ON "product_color_prices"("approval_status");

-- 注释
COMMENT ON COLUMN "product_color_prices"."max_quantity" IS '阶梯价区间上限（NULL = 无限）';
COMMENT ON COLUMN "product_color_prices"."customer_id" IS '客户专属（NULL = 通用）';
COMMENT ON COLUMN "product_color_prices"."season" IS '季节标签：SS(春夏) / AW(秋冬) / HOLIDAY(节日) / NULL(通用)';
COMMENT ON COLUMN "product_color_prices"."is_active" IS '是否启用';
COMMENT ON COLUMN "product_color_prices"."priority" IS '优先级（数值大 = 优先级高）';
COMMENT ON COLUMN "product_color_prices"."approval_status" IS '审批状态：PENDING / APPROVED / REJECTED';"#,
    ),
    (
        "20260618000002_create_color_price_history",
        r#"-- 价格历史表 - P0-5 面料多色号定价扩展
-- 记录每次调价的变更前/后价格、操作人、原因、审批信息
-- 创建时间: 2026-06-18
-- 关联 spec: docs/superpowers/specs/2026-06-16-color-price-extension-design.md §3.3

CREATE TABLE IF NOT EXISTS "color_price_history" (
    "id" BIGSERIAL PRIMARY KEY,
    "product_color_price_id" BIGINT NOT NULL REFERENCES "product_color_prices"("id"),
    "old_price" DECIMAL(18,6) NOT NULL,
    "new_price" DECIMAL(18,6) NOT NULL,
    "currency" VARCHAR(10) NOT NULL DEFAULT 'CNY',
    "change_type" VARCHAR(20) NOT NULL,
    "change_reason" TEXT,
    "change_percent" DECIMAL(8,4),
    "quantity" DECIMAL(18,2),
    "operated_by" BIGINT NOT NULL REFERENCES "users"("id"),
    "operated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "approved_by" BIGINT REFERENCES "users"("id"),
    "approved_at" TIMESTAMPTZ,
    "tenant_id" BIGINT NOT NULL,
    CONSTRAINT "chk_history_change_type" CHECK ("change_type" IN ('manual', 'batch', 'seasonal', 'customer_specific', 'tier'))
);

-- 索引
CREATE INDEX IF NOT EXISTS "idx_price_history_price" ON "color_price_history"("product_color_price_id");
CREATE INDEX IF NOT EXISTS "idx_price_history_operated_at" ON "color_price_history"("operated_at");
CREATE INDEX IF NOT EXISTS "idx_price_history_tenant" ON "color_price_history"("tenant_id");
CREATE INDEX IF NOT EXISTS "idx_price_history_change_type" ON "color_price_history"("change_type");
CREATE INDEX IF NOT EXISTS "idx_price_history_operator" ON "color_price_history"("operated_by");

-- 注释
COMMENT ON TABLE "color_price_history" IS '色号价格变更历史表 - 纺织行业价格审计与回溯';
COMMENT ON COLUMN "color_price_history"."change_type" IS '变更类型：manual(手工) / batch(批量) / seasonal(季节) / customer_specific(客户专属) / tier(阶梯)';
COMMENT ON COLUMN "color_price_history"."change_percent" IS '涨跌幅（百分比，正数为涨，负数为跌）';
COMMENT ON COLUMN "color_price_history"."quantity" IS '触发价格的数量（阶梯价场景）';"#,
    ),
    (
        "20260618000002_create_sales_quotation_items",
        r#"-- 销售报价单明细
-- 用于存储报价单中每个产品/色号的行项目
-- 创建时间: 2026-06-18
-- 关联计划: 2026-06-17-p12-batch1-quotation-port-plan.md PR-1
-- main 适配说明：
--   - ID / 外键类型与主表保持一致（SERIAL / INTEGER）
--   - product_id / color_id 引用 main 已有的 products / product_colors 表

CREATE TABLE IF NOT EXISTS "sales_quotation_items" (
    "id" SERIAL PRIMARY KEY,
    "quotation_id" INTEGER NOT NULL REFERENCES "sales_quotations"("id") ON DELETE CASCADE,

    "product_id" INTEGER NOT NULL REFERENCES "products"("id"),
    "color_id" INTEGER REFERENCES "product_colors"("id"),
    "color_code" VARCHAR(50),
    "pantone_code" VARCHAR(50),
    "cncs_code" VARCHAR(50),

    "specification" TEXT,
    "unit" VARCHAR(20) NOT NULL,

    "quantity" DECIMAL(18,2) NOT NULL,
    "unit_price" DECIMAL(18,6) NOT NULL,
    "unit_price_with_tax" DECIMAL(18,6) NOT NULL,
    "amount" DECIMAL(18,2) NOT NULL,
    "amount_with_tax" DECIMAL(18,2) NOT NULL,

    "tier_pricing" JSONB,
    "discount_rate" DECIMAL(5,2) DEFAULT 0,
    "discount_amount" DECIMAL(18,2) DEFAULT 0,

    "notes" TEXT,
    "sequence" INT NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS "idx_quotation_items_quotation" ON "sales_quotation_items"("quotation_id");
CREATE INDEX IF NOT EXISTS "idx_quotation_items_product" ON "sales_quotation_items"("product_id");
CREATE INDEX IF NOT EXISTS "idx_quotation_items_color" ON "sales_quotation_items"("color_id");

COMMENT ON TABLE "sales_quotation_items" IS '销售报价单明细 - 行项目（产品/色号/数量/单价/金额）';
COMMENT ON COLUMN "sales_quotation_items"."quotation_id" IS '报价单 ID（外键 sales_quotations.id，级联删除）';
COMMENT ON COLUMN "sales_quotation_items"."tier_pricing" IS '阶梯价 JSON 数据';"#,
    ),
    (
        "20260618000003_create_color_price_tiers",
        r#"-- 阶梯定价表 - P0-5 面料多色号定价扩展
-- 数量区间 × 客户等级 → 阶梯价
-- 创建时间: 2026-06-18
-- 关联 spec: docs/superpowers/specs/2026-06-16-color-price-extension-design.md §3.4

CREATE TABLE IF NOT EXISTS "color_price_tiers" (
    "id" BIGSERIAL PRIMARY KEY,
    "product_color_price_id" BIGINT NOT NULL REFERENCES "product_color_prices"("id"),
    "min_quantity" DECIMAL(18,2) NOT NULL DEFAULT 1,
    "max_quantity" DECIMAL(18,2),
    "tier_price" DECIMAL(18,6) NOT NULL,
    "customer_level" VARCHAR(20),
    "sequence" INT NOT NULL DEFAULT 0,
    "tenant_id" BIGINT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "uq_tier_price" UNIQUE ("product_color_price_id", "min_quantity", "customer_level"),
    CONSTRAINT "chk_tier_customer_level" CHECK ("customer_level" IS NULL OR "customer_level" IN ('VIP', 'NORMAL', 'GOLD', 'SILVER'))
);

-- 索引
CREATE INDEX IF NOT EXISTS "idx_price_tiers_price" ON "color_price_tiers"("product_color_price_id");
CREATE INDEX IF NOT EXISTS "idx_price_tiers_sequence" ON "color_price_tiers"("product_color_price_id", "sequence");
CREATE INDEX IF NOT EXISTS "idx_price_tiers_tenant" ON "color_price_tiers"("tenant_id");

-- 注释
COMMENT ON TABLE "color_price_tiers" IS '色号价格阶梯表 - 数量越多价越低，支持按客户等级叠加';
COMMENT ON COLUMN "color_price_tiers"."min_quantity" IS '起始数量（含）';
COMMENT ON COLUMN "color_price_tiers"."max_quantity" IS '结束数量（不含），NULL = 无限';
COMMENT ON COLUMN "color_price_tiers"."tier_price" IS '阶梯价';
COMMENT ON COLUMN "color_price_tiers"."customer_level" IS '客户等级（NULL = 通用，VIP/NORMAL/GOLD/SILVER 各自阶梯）';
COMMENT ON COLUMN "color_price_tiers"."sequence" IS '阶梯顺序（数值小 = 低阶梯）';"#,
    ),
    (
        "20260618000003_create_sales_quotation_terms",
        r#"-- 销售报价单贸易条款
-- 用于存储报价单中各类贸易条款（物流/付款/样品/检验）
-- 创建时间: 2026-06-18
-- 关联计划: 2026-06-17-p12-batch1-quotation-port-plan.md PR-1
-- main 适配说明：
--   - ID / 外键类型与主表保持一致（SERIAL / INTEGER）
--   - term_type 枚举沿用 test 分支约定（logistics/payment/sample/inspection）

CREATE TABLE IF NOT EXISTS "sales_quotation_terms" (
    "id" SERIAL PRIMARY KEY,
    "quotation_id" INTEGER NOT NULL REFERENCES "sales_quotations"("id") ON DELETE CASCADE,
    "term_type" VARCHAR(50) NOT NULL,
    "term_key" VARCHAR(100) NOT NULL,
    "term_value" TEXT NOT NULL,
    "sequence" INT NOT NULL DEFAULT 0,

    CONSTRAINT "chk_term_type" CHECK ("term_type" IN ('logistics','payment','sample','inspection'))
);

CREATE INDEX IF NOT EXISTS "idx_quotation_terms_quotation" ON "sales_quotation_terms"("quotation_id");
CREATE INDEX IF NOT EXISTS "idx_quotation_terms_type" ON "sales_quotation_terms"("term_type");

COMMENT ON TABLE "sales_quotation_terms" IS '销售报价单贸易条款 - 物流/付款/样品/检验四类条款';
COMMENT ON COLUMN "sales_quotation_terms"."term_type" IS '条款类型 - logistics/payment/sample/inspection';"#,
    ),
    (
        "20260618000004_create_customer_color_prices",
        r#"-- 客户专属价表 - P0-5 面料多色号定价扩展
-- 战略客户 / 大客户协议价（最高优先级）
-- 创建时间: 2026-06-18
-- 关联 spec: docs/superpowers/specs/2026-06-16-color-price-extension-design.md §3.5

CREATE TABLE IF NOT EXISTS "customer_color_prices" (
    "id" BIGSERIAL PRIMARY KEY,
    "customer_id" BIGINT NOT NULL REFERENCES "customers"("id"),
    "product_id" BIGINT NOT NULL REFERENCES "products"("id"),
    "color_id" BIGINT NOT NULL REFERENCES "product_colors"("id"),
    "special_price" DECIMAL(18,6) NOT NULL,
    "discount_percent" DECIMAL(5,2),
    "currency" VARCHAR(10) NOT NULL DEFAULT 'CNY',
    "valid_from" DATE NOT NULL,
    "valid_until" DATE,
    "notes" TEXT,
    "approved_by" BIGINT REFERENCES "users"("id"),
    "approved_at" TIMESTAMPTZ,
    "tenant_id" BIGINT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "uq_customer_color_price" UNIQUE ("customer_id", "product_id", "color_id", "valid_from")
);

-- 索引
CREATE INDEX IF NOT EXISTS "idx_cust_color_price_customer" ON "customer_color_prices"("customer_id");
CREATE INDEX IF NOT EXISTS "idx_cust_color_price_product_color" ON "customer_color_prices"("product_id", "color_id");
CREATE INDEX IF NOT EXISTS "idx_cust_color_price_tenant" ON "customer_color_prices"("tenant_id");
CREATE INDEX IF NOT EXISTS "idx_cust_color_price_valid" ON "customer_color_prices"("valid_from", "valid_until");

-- 注释
COMMENT ON TABLE "customer_color_prices" IS '客户专属价格表 - 战略客户大客户协议价（最高优先级）';
COMMENT ON COLUMN "customer_color_prices"."special_price" IS '专属价格（直接覆盖所有其他规则）';
COMMENT ON COLUMN "customer_color_prices"."discount_percent" IS '折扣率（0.95 = 95 折，0.85 = 85 折）';
COMMENT ON COLUMN "customer_color_prices"."valid_from" IS '生效日期';
COMMENT ON COLUMN "customer_color_prices"."valid_until" IS '失效日期（NULL = 长期有效）';"#,
    ),
    (
        "20260618000005_create_seasonal_price_rules",
        r#"-- 季节调价规则表 - P0-5 面料多色号定价扩展
-- 按季节自动调价（春夏 / 秋冬 / 节日）
-- 创建时间: 2026-06-18
-- 关联 spec: docs/superpowers/specs/2026-06-16-color-price-extension-design.md §3.6

CREATE TABLE IF NOT EXISTS "seasonal_price_rules" (
    "id" BIGSERIAL PRIMARY KEY,
    "rule_name" VARCHAR(100) NOT NULL,
    "season" VARCHAR(10) NOT NULL,
    "product_category_id" BIGINT REFERENCES "product_categories"("id"),
    "adjustment_type" VARCHAR(20) NOT NULL,
    "adjustment_value" DECIMAL(8,4) NOT NULL,
    "valid_from" DATE NOT NULL,
    "valid_until" DATE,
    "is_active" BOOLEAN NOT NULL DEFAULT true,
    "description" TEXT,
    "tenant_id" BIGINT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "chk_seasonal_type" CHECK ("season" IN ('SS', 'AW', 'HOLIDAY')),
    CONSTRAINT "chk_seasonal_adjustment_type" CHECK ("adjustment_type" IN ('percentage', 'fixed')),
    CONSTRAINT "chk_seasonal_valid_range" CHECK ("valid_until" IS NULL OR "valid_until" >= "valid_from")
);

-- 索引
CREATE INDEX IF NOT EXISTS "idx_seasonal_tenant_active" ON "seasonal_price_rules"("tenant_id", "is_active");
CREATE INDEX IF NOT EXISTS "idx_seasonal_season_valid" ON "seasonal_price_rules"("season", "valid_from", "valid_until");
CREATE INDEX IF NOT EXISTS "idx_seasonal_category" ON "seasonal_price_rules"("product_category_id");

-- 注释
COMMENT ON TABLE "seasonal_price_rules" IS '季节性调价规则表 - 春夏/秋冬/节日自动调价';
COMMENT ON COLUMN "seasonal_price_rules"."season" IS '季节：SS(春夏) / AW(秋冬) / HOLIDAY(节日)';
COMMENT ON COLUMN "seasonal_price_rules"."product_category_id" IS '品类（NULL = 全部产品）';
COMMENT ON COLUMN "seasonal_price_rules"."adjustment_type" IS '调整方式：percentage(百分比) / fixed(固定金额)';
COMMENT ON COLUMN "seasonal_price_rules"."adjustment_value" IS '调整值：+0.10 = 涨 10%，-0.05 = 降 5%，+1.5 = 加 1.5 元';"#,
    ),
    // 20260706 系列（TOTP/库存对齐/跟踪/预算）
    (
        "20260706000001_add_totp_recovery_codes_to_users",
        r#"-- v11 批次 141：2FA 恢复码后端实现
-- 新增 totp_recovery_codes 字段存储恢复码的 bcrypt 哈希（JSON 数组格式）
-- 恢复码明文仅在生成时返回给用户，服务端只存哈希
ALTER TABLE "users" ADD COLUMN IF NOT EXISTS "totp_recovery_codes" TEXT;
COMMENT ON COLUMN "users"."totp_recovery_codes" IS 'TOTP 恢复码哈希数组（JSON 格式，bcrypt 哈希）';"#,
    ),
    (
        "20260706000002_align_inventory_count_schema",
        r#"-- v11 批次 143 P1-1：库存盘点模块 schema 对齐
-- 将 inventory_counts / inventory_count_items 表结构对齐到 SeaORM 模型定义
-- 模型位置：backend/src/models/inventory_count.rs / inventory_count_item.rs

-- ============================================
-- 1. inventory_counts 表：新增模型字段 + 类型对齐
-- ============================================

-- count_date 由 DATE 升级为 TIMESTAMPTZ（与模型 DateTime<Utc> 对齐）
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_counts' AND column_name = 'count_date' AND data_type = 'date') THEN
        ALTER TABLE "inventory_counts" ALTER COLUMN "count_date" TYPE TIMESTAMPTZ USING "count_date" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- status 默认值由 'draft' 调整为 'pending'（与盘点流程入口一致）
ALTER TABLE "inventory_counts" ALTER COLUMN "status" SET DEFAULT 'pending';

-- 新增盘点单字段
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "total_items" INTEGER NOT NULL DEFAULT 0;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "counted_items" INTEGER NOT NULL DEFAULT 0;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "variance_items" INTEGER NOT NULL DEFAULT 0;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "approved_at" TIMESTAMPTZ;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "completed_at" TIMESTAMPTZ;

-- 删除旧字段（已被 total_items/counted_items/variance_items 取代）
-- total_discrepancy 字段语义与 variance_items 重叠，且无业务引用
ALTER TABLE "inventory_counts" DROP COLUMN IF EXISTS "total_discrepancy";
-- is_deleted 字段无业务引用（盘点单通过状态字段管理生命周期）
ALTER TABLE "inventory_counts" DROP COLUMN IF EXISTS "is_deleted";

COMMENT ON COLUMN "inventory_counts"."total_items" IS '盘点单总明细数';
COMMENT ON COLUMN "inventory_counts"."counted_items" IS '已盘点明细数';
COMMENT ON COLUMN "inventory_counts"."variance_items" IS '存在差异的明细数';
COMMENT ON COLUMN "inventory_counts"."approved_at" IS '审批时间';
COMMENT ON COLUMN "inventory_counts"."completed_at" IS '完成时间';

-- ============================================
-- 2. inventory_count_items 表：新增模型字段 + 类型对齐
-- ============================================

-- 新增盘点明细字段（stock_id/warehouse_id 使用 NOT NULL DEFAULT 0 以兼容已有行）
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "stock_id" INTEGER NOT NULL DEFAULT 0;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "warehouse_id" INTEGER NOT NULL DEFAULT 0;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "quantity_before" DECIMAL(10, 2) NOT NULL DEFAULT 0;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "quantity_actual" DECIMAL(10, 2) NOT NULL DEFAULT 0;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "quantity_difference" DECIMAL(10, 2) NOT NULL DEFAULT 0;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "total_cost" DECIMAL(12, 2) NOT NULL DEFAULT 0;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "notes" TEXT;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP;

-- 旧字段数据迁移到新字段（若旧字段存在且有数据）
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_count_items' AND column_name = 'system_quantity') THEN
        UPDATE "inventory_count_items" SET "quantity_before" = "system_quantity" WHERE "quantity_before" = 0 AND "system_quantity" IS NOT NULL;
    END IF;
END $$;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_count_items' AND column_name = 'actual_quantity') THEN
        UPDATE "inventory_count_items" SET "quantity_actual" = "actual_quantity" WHERE "quantity_actual" = 0 AND "actual_quantity" IS NOT NULL;
    END IF;
END $$;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_count_items' AND column_name = 'discrepancy_quantity') THEN
        UPDATE "inventory_count_items" SET "quantity_difference" = "discrepancy_quantity" WHERE "quantity_difference" = 0 AND "discrepancy_quantity" IS NOT NULL;
    END IF;
END $$;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_count_items' AND column_name = 'discrepancy_amount') THEN
        UPDATE "inventory_count_items" SET "total_cost" = "discrepancy_amount" WHERE "total_cost" = 0 AND "discrepancy_amount" IS NOT NULL;
    END IF;
END $$;

-- 删除旧字段（数据已迁移到新字段）
ALTER TABLE "inventory_count_items" DROP COLUMN IF EXISTS "batch_no";
ALTER TABLE "inventory_count_items" DROP COLUMN IF EXISTS "system_quantity";
ALTER TABLE "inventory_count_items" DROP COLUMN IF EXISTS "actual_quantity";
ALTER TABLE "inventory_count_items" DROP COLUMN IF EXISTS "discrepancy_quantity";
ALTER TABLE "inventory_count_items" DROP COLUMN IF EXISTS "discrepancy_amount";
ALTER TABLE "inventory_count_items" DROP COLUMN IF EXISTS "is_deleted";

-- unit_cost 字段类型保持 DECIMAL(12, 2) 不变（与模型一致）
COMMENT ON COLUMN "inventory_count_items"."stock_id" IS '库存 ID（关联 inventory_stocks.id）';
COMMENT ON COLUMN "inventory_count_items"."warehouse_id" IS '仓库 ID';
COMMENT ON COLUMN "inventory_count_items"."quantity_before" IS '盘点前账面数量';
COMMENT ON COLUMN "inventory_count_items"."quantity_actual" IS '实际盘点数量';
COMMENT ON COLUMN "inventory_count_items"."quantity_difference" IS '差异数量（实际 - 账面）';
COMMENT ON COLUMN "inventory_count_items"."total_cost" IS '总成本差异';
COMMENT ON COLUMN "inventory_count_items"."notes" IS '明细备注';

-- 添加外键约束
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_inventory_count_items_stock') THEN
        ALTER TABLE "inventory_count_items" ADD CONSTRAINT "fk_inventory_count_items_stock" FOREIGN KEY ("stock_id") REFERENCES "inventory_stocks" ("id");
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_inventory_count_items_warehouse') THEN
        ALTER TABLE "inventory_count_items" ADD CONSTRAINT "fk_inventory_count_items_warehouse" FOREIGN KEY ("warehouse_id") REFERENCES "warehouses" ("id");
    END IF;
END $$;

-- 添加索引
CREATE INDEX IF NOT EXISTS "idx_inventory_counts_warehouse" ON "inventory_counts" ("warehouse_id");
CREATE INDEX IF NOT EXISTS "idx_inventory_counts_status" ON "inventory_counts" ("status");
CREATE INDEX IF NOT EXISTS "idx_inventory_counts_count_date" ON "inventory_counts" ("count_date");
CREATE INDEX IF NOT EXISTS "idx_inventory_count_items_count" ON "inventory_count_items" ("count_id");
CREATE INDEX IF NOT EXISTS "idx_inventory_count_items_stock" ON "inventory_count_items" ("stock_id");
CREATE INDEX IF NOT EXISTS "idx_inventory_count_items_warehouse" ON "inventory_count_items" ("warehouse_id");"#,
    ),
    (
        "20260706000003_create_tracking_tables",
        r#"-- v11 批次 143 P1-2：用户行为追踪分析模块
-- 创建 page_views / user_behaviors 表，支持页面访问统计、热门页面、漏斗分析、用户路径分析

-- ============================================
-- 1. 页面访问记录表
-- ============================================
CREATE TABLE IF NOT EXISTS "page_views" (
    "id" BIGSERIAL PRIMARY KEY,
    "session_id" VARCHAR(100),
    "user_id" INTEGER,
    "path" VARCHAR(500) NOT NULL,
    "referrer" VARCHAR(500),
    "user_agent" VARCHAR(500),
    "ip_address" VARCHAR(45),
    "viewed_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_page_views_path" ON "page_views" ("path");
CREATE INDEX IF NOT EXISTS "idx_page_views_viewed_at" ON "page_views" ("viewed_at");
CREATE INDEX IF NOT EXISTS "idx_page_views_session" ON "page_views" ("session_id");
CREATE INDEX IF NOT EXISTS "idx_page_views_user" ON "page_views" ("user_id");

COMMENT ON TABLE "page_views" IS '页面访问记录表';
COMMENT ON COLUMN "page_views"."session_id" IS '会话 ID（匿名用户标识）';
COMMENT ON COLUMN "page_views"."user_id" IS '用户 ID（登录用户）';
COMMENT ON COLUMN "page_views"."path" IS '页面路径';
COMMENT ON COLUMN "page_views"."referrer" IS '来源页面';
COMMENT ON COLUMN "page_views"."viewed_at" IS '访问时间';

-- ============================================
-- 2. 用户行为记录表
-- ============================================
CREATE TABLE IF NOT EXISTS "user_behaviors" (
    "id" BIGSERIAL PRIMARY KEY,
    "session_id" VARCHAR(100),
    "user_id" INTEGER,
    "event_type" VARCHAR(50) NOT NULL,
    "event_target" VARCHAR(200),
    "event_data" JSONB,
    "path" VARCHAR(500),
    "ip_address" VARCHAR(45),
    "occurred_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_user_behaviors_event_type" ON "user_behaviors" ("event_type");
CREATE INDEX IF NOT EXISTS "idx_user_behaviors_occurred_at" ON "user_behaviors" ("occurred_at");
CREATE INDEX IF NOT EXISTS "idx_user_behaviors_session" ON "user_behaviors" ("session_id");
CREATE INDEX IF NOT EXISTS "idx_user_behaviors_user" ON "user_behaviors" ("user_id");

COMMENT ON TABLE "user_behaviors" IS '用户行为记录表';
COMMENT ON COLUMN "user_behaviors"."event_type" IS '事件类型（click/scroll/submit 等）';
COMMENT ON COLUMN "user_behaviors"."event_target" IS '事件目标（元素 ID/类名等）';
COMMENT ON COLUMN "user_behaviors"."event_data" IS '事件附加数据（JSON）';"#,
    ),
    (
        "20260706000004_add_max_stock_point_to_inventory_stocks",
        r#"-- v11 批次 144 P1-4：为 inventory_stocks 添加 max_stock_point 字段
--
-- 背景：
--   stock_alert.rs 中 AlertType::OverStock 此前标注为 dead_code，
--   原因是 inventory_stocks 表无 max_stock_point（库存上限）字段，
--   compute_alert_type 无法判定"高于上限"告警。
--
-- 修复：
--   1. 添加 max_stock_point DECIMAL(12,2) NOT NULL DEFAULT 0
--      （与 reorder_point 保持一致的类型与默认值语义，0 表示未设置上限）
--   2. 添加注释
--
-- 关联：
--   - P1-5 移除 stock_alert.rs 中 OverStock / SlowMoving 的 dead_code 标注
--   - compute_alert_type 扩展 OverStock / SlowMoving 告警判定
--   - last_movement_date 字段已在 20260613000001_add_missing_columns 中添加，无需重复

ALTER TABLE "inventory_stocks" ADD COLUMN "max_stock_point" DECIMAL(12, 2) NOT NULL DEFAULT 0;

COMMENT ON COLUMN "inventory_stocks"."max_stock_point" IS '库存上限（高于此值触发 OverStock 告警，0 表示未设置）';"#,
    ),
    (
        "20260706000005_extend_budget_management",
        r#"-- v11 批次 145 P1-8：扩展 budget_items 表，接入预算科目扩展字段
--
-- 背景：
--   budget_management_service.rs 中 CreateBudgetItemRequest / UpdateBudgetItemRequest
--   的 budget_year / planned_amount / remark 字段此前标注为 dead_code，
--   原因是 budget_items 表无对应字段，service.create_item / update_item 无法持久化这些字段。
--   handler 层（budget_management_handler.rs）已经从客户端接收这些字段并传入 service，
--   但 service 层直接丢弃，造成数据丢失。
--
-- 修复：
--   1. budget_year INT4 NULL（可选预算年度，便于按年度筛选科目）
--   2. planned_amount DECIMAL(14,2) NOT NULL DEFAULT 0（计划金额，与 budget_plan.total_amount 类型保持一致）
--   3. remark VARCHAR(500) NULL（备注）
--
-- 关联：
--   - 移除 CreateBudgetItemRequest / UpdateBudgetItemRequest 的 dead_code 标注
--   - create_item / update_item 方法接入这些字段
--   - budget_management 模型同步扩展

ALTER TABLE "budget_items" ADD COLUMN "budget_year" INT4 NULL;
ALTER TABLE "budget_items" ADD COLUMN "planned_amount" DECIMAL(14, 2) NOT NULL DEFAULT 0;
ALTER TABLE "budget_items" ADD COLUMN "remark" VARCHAR(500) NULL;

COMMENT ON COLUMN "budget_items"."budget_year" IS '预算年度（可选，用于按年度筛选预算科目）';
COMMENT ON COLUMN "budget_items"."planned_amount" IS '计划金额（该科目的年度计划预算金额）';
COMMENT ON COLUMN "budget_items"."remark" IS '备注（最多 500 字符）';"#,
    ),
    // 20260707 系列（仓库容量/API密钥描述/密码历史）
    (
        "20260707000001_add_capacity_to_warehouses",
        r#"-- 批次 158 v11 真实接入：warehouse 表添加 capacity 列（规则 0/1/2 真实实现）
-- 原 warehouse_handler.rs CreateWarehouseRequest/UpdateWarehouseRequest 含 capacity 字段
-- 但 warehouse 表无对应列，字段被 #[allow(dead_code)] 标注
-- 现扩展 schema 接入业务，移除 allow 标注
ALTER TABLE warehouses ADD COLUMN IF NOT EXISTS capacity INTEGER;

-- 注释说明字段语义
COMMENT ON COLUMN warehouses.capacity IS '仓库容量（单位由业务约定，如托盘数/立方米）';"#,
    ),
    (
        "20260707000002_add_description_to_api_keys",
        r#"-- 批次 158 v11 真实接入：api_keys 表添加 description 列（规则 0/1/2 真实实现）
-- 原 UpdateApiKeyGwRequest 含 description 字段，但 api_keys 表无对应列，
-- 字段被 #[allow(dead_code)] 标注。现扩展 schema 接入业务，移除 allow 标注。
ALTER TABLE api_keys ADD COLUMN IF NOT EXISTS description TEXT;

COMMENT ON COLUMN api_keys.description IS 'API 密钥描述（可选）';"#,
    ),
    (
        "20260707000003_create_password_histories",
        r#"-- 批次 158 v11 真实接入：密码策略服务 - 密码历史持久化
-- 配合 password_policy_service.rs 的 PasswordHistory / validate_with_history 方法
-- 每次修改密码后将旧密码哈希写入此表，校验新密码时查询最近 N 条记录
CREATE TABLE IF NOT EXISTS "password_histories" (
    "id" SERIAL PRIMARY KEY,
    "user_id" INTEGER NOT NULL REFERENCES "users"("id") ON DELETE CASCADE,
    "password_hash" TEXT NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS "idx_password_histories_user_id" ON "password_histories"("user_id");
CREATE INDEX IF NOT EXISTS "idx_password_histories_user_created" ON "password_histories"("user_id", "created_at" DESC);
COMMENT ON TABLE "password_histories" IS '密码历史表（批次 158 v11 真实接入 PasswordPolicyService）';"#,
    ),
];

/// 修复 SQL 中 BIGINT 外键类型不匹配：引用 SERIAL(INTEGER) id 的 BIGINT 外键需改 INTEGER，BIGSERIAL 的无需修改
fn fix_fk_types(sql: &str) -> String {
    // id 类型为 SERIAL (INTEGER) 的表
    const INTEGER_ID_TABLES: &[&str] = &[
        "users",
        "products",
        "customers",
        "dye_recipe",
        "sales_orders",
        "product_categories",
        "warehouses",
        "suppliers",
        "product_colors",
        "fixed_assets",
    ];
    let mut result = sql.to_string();
    for table in INTEGER_ID_TABLES {
        // BIGINT NOT NULL REFERENCES "table"("id") → INTEGER NOT NULL REFERENCES
        let bigint_nn = format!("BIGINT NOT NULL REFERENCES \"{}\"(\"id\")", table);
        let integer_nn = format!("INTEGER NOT NULL REFERENCES \"{}\"(\"id\")", table);
        result = result.replace(&bigint_nn, &integer_nn);
        // BIGINT REFERENCES "table"("id") → INTEGER REFERENCES
        let bigint = format!("BIGINT REFERENCES \"{}\"(\"id\")", table);
        let integer = format!("INTEGER REFERENCES \"{}\"(\"id\")", table);
        result = result.replace(&bigint, &integer);
    }
    result
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for (name, sql) in UNREFERENCED_MIGRATIONS {
            if !sql.trim().is_empty() {
                // 修复 BIGINT 外键类型不匹配后再执行
                let fixed_sql = fix_fk_types(sql);
                db.execute_unprepared(&fixed_sql)
                    .await
                    .map_err(|e| DbErr::Custom(format!("执行整合迁移 {} 失败: {}", name, e)))?;
            }
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // 整合迁移不支持回滚（含 CREATE TABLE 等不可逆操作）
        Ok(())
    }
}
