//! sales_crm 域聚合迁移

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name() -> &'static str {
        "m_sales_crm_domain"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- 销售报价单主表
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

-- 销售报价单明细
-- 用于存储报价单中每个产品/色号的行项目
-- 创建时间: 2026-06-16

CREATE TABLE IF NOT EXISTS "sales_quotation_items" (
    "id" BIGSERIAL PRIMARY KEY,
    "quotation_id" BIGINT NOT NULL REFERENCES "sales_quotations"("id") ON DELETE CASCADE,

    "product_id" BIGINT NOT NULL REFERENCES "products"("id"),
    "color_id" BIGINT REFERENCES "product_colors"("id"),
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

-- 销售报价单贸易条款
-- 用于存储报价单中各类贸易条款（物流/付款/样品/检验）
-- 创建时间: 2026-06-16

CREATE TABLE IF NOT EXISTS "sales_quotation_terms" (
    "id" BIGSERIAL PRIMARY KEY,
    "quotation_id" BIGINT NOT NULL REFERENCES "sales_quotations"("id") ON DELETE CASCADE,
    "term_type" VARCHAR(50) NOT NULL,
    "term_key" VARCHAR(100) NOT NULL,
    "term_value" TEXT NOT NULL,
    "sequence" INT NOT NULL DEFAULT 0,

    CONSTRAINT "chk_term_type" CHECK ("term_type" IN ('logistics','payment','sample','inspection'))
);

CREATE INDEX IF NOT EXISTS "idx_quotation_terms_quotation" ON "sales_quotation_terms"("quotation_id");
CREATE INDEX IF NOT EXISTS "idx_quotation_terms_type" ON "sales_quotation_terms"("term_type");

-- 色号价格表（预先建，报价单依赖）
-- 用于存储每个产品色号在指定币种/客户等级下的基础价
-- 创建时间: 2026-06-16

CREATE TABLE IF NOT EXISTS "product_color_prices" (
    "id" BIGSERIAL PRIMARY KEY,
    "product_id" BIGINT NOT NULL REFERENCES "products"("id"),
    "color_id" BIGINT NOT NULL REFERENCES "product_colors"("id"),
    "currency" VARCHAR(10) NOT NULL DEFAULT 'CNY',
    "base_price" DECIMAL(18,6) NOT NULL,
    "effective_from" DATE NOT NULL,
    "effective_to" DATE,
    "customer_level" VARCHAR(20),
    "min_quantity" DECIMAL(18,2) DEFAULT 1,
    "notes" TEXT,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT "uq_color_price" UNIQUE ("product_id", "color_id", "currency", "customer_level", "effective_from")
);

CREATE INDEX IF NOT EXISTS "idx_color_prices_product_color" ON "product_color_prices"("product_id", "color_id");

-- omni_audit_logs 签名列迁移（P0 8-2 批次 53）
-- 创建时间: 2026-07-01
-- 关联修复: 八维度审计 P0 8-2 — 审计日志签名计算后丢弃，无防篡改
--
-- 向 omni_audit_logs 表添加 signature 列，存储 HMAC-SHA256 防篡改签名。
-- 签名材料：trace_id|event_type|action|payload
-- 使用 ADD COLUMN IF NOT EXISTS 防止迁移重入。

ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "signature" VARCHAR(128);

COMMENT ON COLUMN "omni_audit_logs"."signature" IS 'HMAC-SHA256 防篡改签名（trace_id|event_type|action|payload）';

-- custom_orders 备注列迁移（批次 88 PH-1）
-- 创建时间: 2026-07-03
-- 关联修复: 占位符 PH-1 — DTO 有 notes 字段但 service 层 `let _ = v;` 丢弃
--
-- 向 custom_orders 表添加 notes 列（TEXT，可选），存储订单备注。
-- 使用 ADD COLUMN IF NOT EXISTS 防止迁移重入。
--
-- 顺序保护：custom_orders 表由 production 域创建，
-- 本迁移位于 sales_crm 域，production 域在其后执行。表尚未创建时用 information_schema
-- 检查跳过，避免 "relation custom_orders does not exist" 中断整个迁移链。
-- notes 列在 production 域的 CREATE TABLE 中已声明，表创建后即具备该列，本迁移为幂等兜底。

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'custom_orders') THEN
        ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "notes" TEXT;
        COMMENT ON COLUMN "custom_orders"."notes" IS '订单备注（批次 88 PH-1 占位符实现）';
    END IF;
END $$;

-- fixed_asset_disposals 处置损益列迁移（批次 88 PH-3）
-- 创建时间: 2026-07-03
-- 关联修复: 占位符 PH-3 — service 计算后 `let _disposal_gain_loss = ...` 丢弃
--
-- 向 fixed_asset_disposals 表添加 gain_loss 列（DECIMAL(15,2)，可选），
-- 存储处置损益 = disposal_amount - 处置时账面净值（正数为收益，负数为损失）。
-- 使用 ADD COLUMN IF NOT EXISTS 防止迁移重入。

ALTER TABLE "fixed_asset_disposals" ADD COLUMN IF NOT EXISTS "gain_loss" DECIMAL(15, 2);
COMMENT ON COLUMN "fixed_asset_disposals"."gain_loss" IS '处置损益 = disposal_amount - 处置时账面净值（正数为收益，负数为损失，批次 88 PH-3 占位符实现）';

-- 固定资产折旧期间记录表迁移（批次 88 PH-2）
-- 创建时间: 2026-07-03
-- 关联修复: 占位符 PH-2 — service `period` 参数仅写日志，未按期间记录折旧
--
-- 新建 fixed_asset_depreciation_records 表，按期间记录每笔折旧计提明细，
-- 支持审计追溯"资产 X 在 2026-06 期间计提了多少折旧"。
-- (asset_id, period) 唯一约束防止同一资产同一期间重复计提。

CREATE TABLE IF NOT EXISTS "fixed_asset_depreciation_records" (
    "id" SERIAL PRIMARY KEY,
    "asset_id" INTEGER NOT NULL REFERENCES "fixed_assets"("id"),
    "period" VARCHAR(7) NOT NULL,
    "depreciation_amount" DECIMAL(15, 2) NOT NULL,
    "accumulated_before" DECIMAL(15, 2) NOT NULL,
    "accumulated_after" DECIMAL(15, 2) NOT NULL,
    "net_value_before" DECIMAL(15, 2),
    "net_value_after" DECIMAL(15, 2),
    "depreciation_method" VARCHAR(50),
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "uk_fa_depreciation_records_asset_period" UNIQUE ("asset_id", "period")
);

CREATE INDEX IF NOT EXISTS "idx_fa_depreciation_records_asset" ON "fixed_asset_depreciation_records"("asset_id");
CREATE INDEX IF NOT EXISTS "idx_fa_depreciation_records_period" ON "fixed_asset_depreciation_records"("period");

COMMENT ON TABLE "fixed_asset_depreciation_records" IS '固定资产折旧期间记录表（批次 88 PH-2 占位符实现）';
COMMENT ON COLUMN "fixed_asset_depreciation_records"."period" IS '折旧期间（YYYY-MM 格式）';
COMMENT ON COLUMN "fixed_asset_depreciation_records"."depreciation_amount" IS '本期折旧额';
COMMENT ON COLUMN "fixed_asset_depreciation_records"."accumulated_before" IS '本期前累计折旧';
COMMENT ON COLUMN "fixed_asset_depreciation_records"."accumulated_after" IS '本期后累计折旧';
COMMENT ON COLUMN "fixed_asset_depreciation_records"."net_value_before" IS '本期前账面净值';
COMMENT ON COLUMN "fixed_asset_depreciation_records"."net_value_after" IS '本期后账面净值';
COMMENT ON COLUMN "fixed_asset_depreciation_records"."depreciation_method" IS '折旧方法（如 straight_line）';

-- 客户联系人表迁移（批次 90b P2-12）
-- 创建时间: 2026-07-03
-- 关联修复: 前端 crm/detail.vue "新增联系人功能待实现" 占位符实现
--
-- 新建 customer_contacts 表，记录客户的多个联系人信息（含主联系人标识）。
-- 替代 crm_customer_handler.rs:list_contacts 中从 crm_lead 拼接 JSON 的伪实现。

CREATE TABLE IF NOT EXISTS "customer_contacts" (
    "id" SERIAL PRIMARY KEY,
    "customer_id" INTEGER NOT NULL REFERENCES "customers"("id") ON DELETE CASCADE,
    "name" VARCHAR(50) NOT NULL,
    "title" VARCHAR(100),
    "phone" VARCHAR(50) NOT NULL,
    "email" VARCHAR(100),
    "is_primary" BOOLEAN NOT NULL DEFAULT FALSE,
    "remarks" VARCHAR(500),
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 每个客户最多一个主联系人，部分唯一索引：is_primary=true 时 (customer_id, is_primary) 唯一
CREATE UNIQUE INDEX IF NOT EXISTS "uk_customer_contacts_primary"
    ON "customer_contacts"("customer_id", "is_primary")
    WHERE "is_primary" = TRUE;

CREATE INDEX IF NOT EXISTS "idx_customer_contacts_customer" ON "customer_contacts"("customer_id");

COMMENT ON TABLE "customer_contacts" IS '客户联系人表（批次 90b P2-12 占位符实现）';
COMMENT ON COLUMN "customer_contacts"."customer_id" IS '客户 ID（关联 customers.id）';
COMMENT ON COLUMN "customer_contacts"."name" IS '联系人姓名';
COMMENT ON COLUMN "customer_contacts"."title" IS '职务';
COMMENT ON COLUMN "customer_contacts"."phone" IS '联系电话';
COMMENT ON COLUMN "customer_contacts"."email" IS '联系邮箱';
COMMENT ON COLUMN "customer_contacts"."is_primary" IS '是否主要联系人（每个客户最多一个主联系人，由部分唯一索引约束）';
COMMENT ON COLUMN "customer_contacts"."remarks" IS '备注';
COMMENT ON COLUMN "customer_contacts"."created_by" IS '创建人（关联 users.id）';

-- API 端点管理表（批次 91 P0-1）
-- 管理 API 网关暴露的端点元数据，支持 CRUD 操作
CREATE TABLE IF NOT EXISTS "api_endpoints" (
    "id" SERIAL PRIMARY KEY,
    "path" VARCHAR(255) NOT NULL,
    "method" VARCHAR(10) NOT NULL,
    "description" VARCHAR(500),
    "module" VARCHAR(100),
    "status" VARCHAR(20) NOT NULL DEFAULT 'active',
    "rate_limit" INTEGER NOT NULL DEFAULT 0,
    "timeout" INTEGER NOT NULL DEFAULT 30000,
    "authentication" BOOLEAN NOT NULL DEFAULT TRUE,
    "authorization" JSONB,
    "request_schema" JSONB,
    "response_schema" JSONB,
    "version" VARCHAR(20) DEFAULT 'v1',
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 按路径+方法唯一索引，防止重复注册同一端点
CREATE UNIQUE INDEX IF NOT EXISTS "uk_api_endpoints_path_method"
    ON "api_endpoints"("path", "method");

-- 按模块索引，便于按模块分组查询
CREATE INDEX IF NOT EXISTS "idx_api_endpoints_module" ON "api_endpoints"("module");

-- 按状态索引，便于筛选启用的端点
CREATE INDEX IF NOT EXISTS "idx_api_endpoints_status" ON "api_endpoints"("status");

-- 批次 92 P3-12/P3-13：fixed_asset_depreciation_records 外键策略 + 冗余索引清理
-- 创建时间: 2026-07-03
-- 关联修复:
--   P3-12：外键 ON DELETE RESTRICT —— 禁止连带删除资产时静默删除折旧记录（保留审计完整性）
--   P3-13：DROP 冗余索引 idx_fa_depreciation_records_asset ——
--          UNIQUE(asset_id, period) 复合唯一索引最左前缀已覆盖 WHERE asset_id = ? 查询，
--          单列索引冗余，徒增写入开销和存储。
--
-- 注：PostgreSQL 不支持直接 ALTER CONSTRAINT 改 ON DELETE 行为，需 DROP + ADD 重建。

-- 1. 删除原外键（ON DELETE NO ACTION 默认行为）
ALTER TABLE "fixed_asset_depreciation_records"
    DROP CONSTRAINT IF EXISTS "fixed_asset_depreciation_records_asset_id_fkey";

-- 2. 重建外键，显式 ON DELETE RESTRICT
ALTER TABLE "fixed_asset_depreciation_records"
    ADD CONSTRAINT IF NOT EXISTS "fixed_asset_depreciation_records_asset_id_fkey"
    FOREIGN KEY ("asset_id") REFERENCES "fixed_assets"("id") ON DELETE RESTRICT;

-- 3. 删除冗余单列索引（已被 UNIQUE(asset_id, period) 最左前缀覆盖）
DROP INDEX IF EXISTS "idx_fa_depreciation_records_asset";

-- ar_reconciliations 备注列迁移（批次 109 P1-1）
-- 创建时间: 2026-07-04
-- 关联修复: v7 复审 P1-1 — DTO/Request 中 notes 字段已对外暴露但未持久化
--   - CreateReconciliationRequest.notes（services/ar/mod.rs:45）
--   - UpdateReconciliationRequest.notes（services/ar/mod.rs:57）
--   - GenerateReconciliationRequest.notes（services/ar/mod.rs:152）
--
-- 向 ar_reconciliations 表添加 notes 列（TEXT，可选），存储对账单备注。
-- 使用 ADD COLUMN IF NOT EXISTS 防止迁移重入。

ALTER TABLE "ar_reconciliations" ADD COLUMN IF NOT EXISTS "notes" TEXT;
COMMENT ON COLUMN "ar_reconciliations"."notes" IS '对账单备注（批次 109 P1-1 修复：原 DTO 有字段但未持久化）';

-- 批次 112 P1-9：api_keys 表添加 created_by 列
-- 原 api_keys 表无 created_by 列，list/get 历史密钥无法回溯创建者，handler 传 0 占位。
-- 现新增 created_by 列（可空，兼容历史数据），由 create_api_key / regenerate_api_key 注入真实 user_id。

ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;

COMMENT ON COLUMN "api_keys"."created_by" IS 'API 密钥创建者用户 ID（批次 112 P1-9 修复：原表无此列，handler 传 0 占位）';

-- 创建外键索引便于按创建者查询
CREATE INDEX IF NOT EXISTS "idx_api_keys_created_by" ON "api_keys" ("created_by");

-- 批次 122 v8 复审 P1 修复：CRM 标签字典表
-- 原 crm_customer_handler list_tags 返回硬编码 5 个标签，create_tag/delete_tag 为空操作假实现。
-- 现新增 crm_tag 表存储标签字典（id/name/color/category/created_at），handler 真实接入。
-- 保留 crm_lead.tags TEXT[] 数组字段向后兼容（add_tags handler 仍覆盖式更新该数组）。

CREATE TABLE IF NOT EXISTS "crm_tag" (
    "id" SERIAL PRIMARY KEY,
    "name" VARCHAR(30) NOT NULL UNIQUE,
    "color" VARCHAR(20) NOT NULL DEFAULT '#1890ff',
    "category" VARCHAR(50),
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "crm_tag" IS 'CRM 标签字典表（批次 122 v8 复审 P1 修复：替代 list_tags 硬编码 + create_tag/delete_tag 假实现）';
COMMENT ON COLUMN "crm_tag"."name" IS '标签名称（唯一，长度 1-30）';
COMMENT ON COLUMN "crm_tag"."color" IS '标签颜色（HEX 格式，默认 #1890ff）';
COMMENT ON COLUMN "crm_tag"."category" IS '标签分类（可选，如 customer/lead/supplier）';
COMMENT ON COLUMN "crm_tag"."created_by" IS '创建者用户 ID';

-- 创建按分类查询的索引
CREATE INDEX IF NOT EXISTS "idx_crm_tag_category" ON "crm_tag" ("category");

-- 初始化预定义标签（与原硬编码 list_tags 保持一致，保证向后兼容）
INSERT INTO "crm_tag" ("name", "color", "category") VALUES
    ('VIP', '#f50', 'customer'),
    ('重点客户', '#2db7f5', 'customer'),
    ('潜在客户', '#87d068', 'lead'),
    ('新客户', '#108ee9', 'lead'),
    ('流失客户', '#f50', 'customer')
ON CONFLICT ("name") DO NOTHING;

-- 序列同步（INSERT 后重置序列，防止主键冲突）
SELECT setval('crm_tag_id_seq', COALESCE((SELECT MAX(id) FROM "crm_tag"), 0) + 1, false);


-- === 从旧迁移恢复的 ALTER ADD COLUMN（确保迁移表结构与 Model 一致）===
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "approved_by" BIGINT;
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "max_quantity" DECIMAL(18,2);
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "season" VARCHAR(10);
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "created_by" BIGINT;
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "is_active" BOOLEAN;
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "priority" INT;
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "customer_id" BIGINT;
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "approval_status" VARCHAR(20);
ALTER TABLE "product_color_prices" ADD COLUMN IF NOT EXISTS "approved_at" TIMESTAMPTZ;
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "freight_cost" DECIMAL(14,2);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "duty_cost" DECIMAL(14,2);
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "insurance_cost" DECIMAL(14,2);


-- === 从 Model 推断补全 ALTER ADD COLUMN（确保迁移与 Model 字段一致）===
ALTER TABLE "api_endpoints" ADD COLUMN IF NOT EXISTS "deprecated_at" TIMESTAMPTZ;
ALTER TABLE "api_endpoints" ADD COLUMN IF NOT EXISTS "deprecation_note" VARCHAR(255);
ALTER TABLE "api_endpoints" ADD COLUMN IF NOT EXISTS "sunset_at" TIMESTAMPTZ;
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255);
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "expires_at" TIMESTAMPTZ;
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "is_active" BOOLEAN;
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "key_hash" VARCHAR(255);
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "key_prefix" VARCHAR(255);
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "last_used_at" TIMESTAMPTZ;
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "name" VARCHAR(255);
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "permissions" VARCHAR(255);
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "rate_limit_per_minute" INTEGER;
ALTER TABLE "api_keys" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "ar_reconciliations" ADD COLUMN IF NOT EXISTS "closing_balance" DECIMAL(18,4);
ALTER TABLE "ar_reconciliations" ADD COLUMN IF NOT EXISTS "confirmed_at" TIMESTAMPTZ;
ALTER TABLE "ar_reconciliations" ADD COLUMN IF NOT EXISTS "confirmed_by" INTEGER;
ALTER TABLE "ar_reconciliations" ADD COLUMN IF NOT EXISTS "confirmed_by_customer" BOOLEAN;
ALTER TABLE "ar_reconciliations" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "ar_reconciliations" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "ar_reconciliations" ADD COLUMN IF NOT EXISTS "customer_id" INTEGER;
ALTER TABLE "ar_reconciliations" ADD COLUMN IF NOT EXISTS "customer_name" VARCHAR(255);
ALTER TABLE "ar_reconciliations" ADD COLUMN IF NOT EXISTS "dispute_reason" VARCHAR(255);
ALTER TABLE "ar_reconciliations" ADD COLUMN IF NOT EXISTS "opening_balance" DECIMAL(18,4);
ALTER TABLE "ar_reconciliations" ADD COLUMN IF NOT EXISTS "period_end" DATE;
ALTER TABLE "ar_reconciliations" ADD COLUMN IF NOT EXISTS "period_start" DATE;
ALTER TABLE "ar_reconciliations" ADD COLUMN IF NOT EXISTS "reconciliation_date" DATE;
ALTER TABLE "ar_reconciliations" ADD COLUMN IF NOT EXISTS "reconciliation_no" VARCHAR(255);
ALTER TABLE "ar_reconciliations" ADD COLUMN IF NOT EXISTS "reconciliation_status" VARCHAR(255);
ALTER TABLE "ar_reconciliations" ADD COLUMN IF NOT EXISTS "total_collections" DECIMAL(18,4);
ALTER TABLE "ar_reconciliations" ADD COLUMN IF NOT EXISTS "total_invoices" DECIMAL(18,4);
ALTER TABLE "ar_reconciliations" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "actual_delivery_date" DATE;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "approval_instance_id" BIGINT;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "approved_at" TIMESTAMPTZ;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "approved_by" BIGINT;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "color_id" BIGINT;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "created_by" BIGINT;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "currency" VARCHAR(255);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "custom_requirements" JSONB;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "customer_approval_comment" VARCHAR(255);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "customer_approved_at" TIMESTAMPTZ;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "customer_id" BIGINT;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "dye_method" VARCHAR(255);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "expected_delivery_date" DATE;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "finishing_method" VARCHAR(255);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "lab_dip_request_id" INTEGER;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "order_no" VARCHAR(255);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "product_id" BIGINT;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "quality_standard_id" INTEGER;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "quantity" DECIMAL(18,4);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "quotation_id" BIGINT;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "rejection_reason" VARCHAR(255);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "sales_order_id" BIGINT;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "spec" VARCHAR(255);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "total_amount" DECIMAL(18,4);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "unit" VARCHAR(255);
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "yarn_spec" VARCHAR(255);
ALTER TABLE "fixed_asset_disposals" ADD COLUMN IF NOT EXISTS "asset_id" INTEGER;
ALTER TABLE "fixed_asset_disposals" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "fixed_asset_disposals" ADD COLUMN IF NOT EXISTS "created_by" INTEGER;
ALTER TABLE "fixed_asset_disposals" ADD COLUMN IF NOT EXISTS "disposal_amount" DECIMAL(18,4);
ALTER TABLE "fixed_asset_disposals" ADD COLUMN IF NOT EXISTS "disposal_date" DATE;
ALTER TABLE "fixed_asset_disposals" ADD COLUMN IF NOT EXISTS "disposal_no" VARCHAR(255);
ALTER TABLE "fixed_asset_disposals" ADD COLUMN IF NOT EXISTS "disposal_reason" VARCHAR(255);
ALTER TABLE "fixed_asset_disposals" ADD COLUMN IF NOT EXISTS "disposal_type" VARCHAR(255);
ALTER TABLE "fixed_asset_disposals" ADD COLUMN IF NOT EXISTS "quantity" INTEGER;
ALTER TABLE "fixed_asset_disposals" ADD COLUMN IF NOT EXISTS "remarks" VARCHAR(255);
ALTER TABLE "fixed_asset_disposals" ADD COLUMN IF NOT EXISTS "status" VARCHAR(255);
ALTER TABLE "fixed_asset_disposals" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ;
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "action" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "condition" TEXT;
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "created_at" TIMESTAMPTZ;
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "description" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "duration_ms" INTEGER;
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "export_approval_token" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "export_record_count" INTEGER;
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "ip_address" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "module" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "new_value" JSONB;
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "old_value" JSONB;
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "operation_category" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "parent_span_id" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "request_body" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "request_method" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "request_path" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "resource_id" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "resource_name" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "resource_type" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "response_status" INTEGER;
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "span_id" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "trace_id" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "user_agent" VARCHAR(255);
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "user_id" INTEGER;
ALTER TABLE "omni_audit_logs" ADD COLUMN IF NOT EXISTS "username" VARCHAR(255);
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
