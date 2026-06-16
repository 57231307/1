-- 销售报价单主表
-- 用于存储销售报价单的核心业务信息
-- 创建时间: 2026-06-16

CREATE TABLE IF NOT EXISTS "sales_quotations" (
    "id" BIGSERIAL PRIMARY KEY,
    "quotation_no" VARCHAR(50) UNIQUE NOT NULL,
    "customer_id" BIGINT NOT NULL REFERENCES "customers"("id"),
    "sales_user_id" BIGINT NOT NULL REFERENCES "users"("id"),
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

    -- 状态
    "status" VARCHAR(20) NOT NULL DEFAULT 'draft',

    -- BPM 审批
    -- 注意：approval_instance_id 的外键约束由 m0024_quotaion_approval_fk 补建（避免依赖表 approval_instances 缺失时阻塞本迁移）
    "approval_instance_id" BIGINT,
    "approved_by" BIGINT REFERENCES "users"("id"),
    "approved_at" TIMESTAMPTZ,
    "rejection_reason" TEXT,

    -- 转换
    "converted_sales_order_id" BIGINT REFERENCES "sales_orders"("id"),
    "converted_at" TIMESTAMPTZ,

    -- 元数据
    "notes" TEXT,
    "created_by" BIGINT NOT NULL REFERENCES "users"("id"),
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT "chk_price_terms" CHECK ("price_terms" IN ('FOB','CIF','EXW','DDP','DAP')),
    CONSTRAINT "chk_status" CHECK ("status" IN ('draft','pending_approval','approved','rejected','expired','converted','cancelled'))
);

CREATE INDEX IF NOT EXISTS "idx_quotations_customer" ON "sales_quotations"("customer_id");
CREATE INDEX IF NOT EXISTS "idx_quotations_status" ON "sales_quotations"("status");
CREATE INDEX IF NOT EXISTS "idx_quotations_valid_until" ON "sales_quotations"("valid_until");
CREATE INDEX IF NOT EXISTS "idx_quotations_sales_user" ON "sales_quotations"("sales_user_id");
