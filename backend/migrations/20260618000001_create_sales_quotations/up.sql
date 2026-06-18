-- 销售报价单主表
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
COMMENT ON COLUMN "sales_quotations"."status" IS '状态 - DRAFT/SUBMITTED/APPROVED/REJECTED/CONVERTED/CANCELLED/EXPIRED';
