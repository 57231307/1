-- 批次1：财务管理模块数据库迁移
-- 创建时间: 2026-05-28
-- 描述: 创建应付发票、应付付款、应收发票、应收对账、应付对账、预算计划、预算科目、固定资产、资金账户表

-- ============================================
-- 1. 应付发票表 (ap_invoice)
-- ============================================
CREATE TABLE IF NOT EXISTS "ap_invoice" (
    "id" SERIAL PRIMARY KEY,
    "invoice_no" VARCHAR(50) NOT NULL UNIQUE,
    "supplier_id" INTEGER NOT NULL,
    "invoice_type" VARCHAR(20) NOT NULL,
    "source_type" VARCHAR(50),
    "source_id" INTEGER,
    "invoice_date" DATE NOT NULL,
    "due_date" DATE NOT NULL,
    "payment_terms" INTEGER NOT NULL DEFAULT 30,
    "amount" DECIMAL(18, 2) NOT NULL,
    "paid_amount" DECIMAL(18, 2) NOT NULL DEFAULT 0.00,
    "unpaid_amount" DECIMAL(18, 2) NOT NULL DEFAULT 0.00,
    "invoice_status" VARCHAR(20) NOT NULL DEFAULT 'DRAFT',
    "currency" VARCHAR(10) NOT NULL DEFAULT 'CNY',
    "exchange_rate" DECIMAL(18, 6) NOT NULL DEFAULT 1.000000,
    "amount_foreign" DECIMAL(18, 2),
    "tax_amount" DECIMAL(18, 2) NOT NULL DEFAULT 0.00,
    "notes" TEXT,
    "attachment_urls" TEXT,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_by" INTEGER,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "approved_by" INTEGER,
    "approved_at" TIMESTAMP,
    "cancelled_by" INTEGER,
    "cancelled_at" TIMESTAMP,
    "cancelled_reason" TEXT,
    CONSTRAINT "fk_ap_invoice_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id"),
    CONSTRAINT "fk_ap_invoice_created_by" FOREIGN KEY ("created_by") REFERENCES "users" ("id"),
    CONSTRAINT "fk_ap_invoice_updated_by" FOREIGN KEY ("updated_by") REFERENCES "users" ("id"),
    CONSTRAINT "fk_ap_invoice_approved_by" FOREIGN KEY ("approved_by") REFERENCES "users" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_ap_invoice_supplier" ON "ap_invoice" ("supplier_id");
CREATE INDEX IF NOT EXISTS "idx_ap_invoice_date" ON "ap_invoice" ("invoice_date");
CREATE INDEX IF NOT EXISTS "idx_ap_invoice_status" ON "ap_invoice" ("invoice_status");
COMMENT ON TABLE "ap_invoice" IS '应付发票表';

-- ============================================
-- 2. 应付付款表 (ap_payment)
-- ============================================
CREATE TABLE IF NOT EXISTS "ap_payment" (
    "id" SERIAL PRIMARY KEY,
    "payment_no" VARCHAR(50) NOT NULL UNIQUE,
    "payment_date" DATE NOT NULL,
    "supplier_id" INTEGER NOT NULL,
    "request_id" INTEGER,
    "payment_method" VARCHAR(20) NOT NULL,
    "payment_amount" DECIMAL(18, 2) NOT NULL,
    "payment_status" VARCHAR(20) NOT NULL DEFAULT 'REGISTERED',
    "currency" VARCHAR(10) NOT NULL DEFAULT 'CNY',
    "exchange_rate" DECIMAL(18, 6) NOT NULL DEFAULT 1.000000,
    "payment_amount_foreign" DECIMAL(18, 2),
    "bank_name" VARCHAR(200),
    "bank_account" VARCHAR(50),
    "transaction_no" VARCHAR(100),
    "notes" TEXT,
    "attachment_urls" TEXT,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_by" INTEGER,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "confirmed_by" INTEGER,
    "confirmed_at" TIMESTAMP,
    CONSTRAINT "fk_ap_payment_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id"),
    CONSTRAINT "fk_ap_payment_created_by" FOREIGN KEY ("created_by") REFERENCES "users" ("id"),
    CONSTRAINT "fk_ap_payment_updated_by" FOREIGN KEY ("updated_by") REFERENCES "users" ("id"),
    CONSTRAINT "fk_ap_payment_confirmed_by" FOREIGN KEY ("confirmed_by") REFERENCES "users" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_ap_payment_supplier" ON "ap_payment" ("supplier_id");
CREATE INDEX IF NOT EXISTS "idx_ap_payment_date" ON "ap_payment" ("payment_date");
CREATE INDEX IF NOT EXISTS "idx_ap_payment_status" ON "ap_payment" ("payment_status");
COMMENT ON TABLE "ap_payment" IS '应付付款表';

-- ============================================
-- 3. 应收发票表 (ar_invoices)
-- ============================================
CREATE TABLE IF NOT EXISTS "ar_invoices" (
    "id" SERIAL PRIMARY KEY,
    "invoice_no" VARCHAR(50) NOT NULL,
    "invoice_date" DATE NOT NULL,
    "due_date" DATE NOT NULL,
    "customer_id" INTEGER NOT NULL,
    "customer_name" VARCHAR(200),
    "customer_code" VARCHAR(50),
    "source_type" VARCHAR(50),
    "source_module" VARCHAR(50),
    "source_bill_id" INTEGER,
    "source_bill_no" VARCHAR(50),
    "batch_no" VARCHAR(50),
    "color_no" VARCHAR(50),
    "dye_lot_no" VARCHAR(50),
    "sales_order_no" VARCHAR(50),
    "invoice_amount" DECIMAL(18, 2) NOT NULL,
    "received_amount" DECIMAL(18, 2) NOT NULL DEFAULT 0.00,
    "unpaid_amount" DECIMAL(18, 2) NOT NULL DEFAULT 0.00,
    "tax_amount" DECIMAL(18, 2),
    "quantity_meters" DECIMAL(12, 2),
    "quantity_kg" DECIMAL(12, 2),
    "unit_price" DECIMAL(18, 6),
    "status" VARCHAR(20) NOT NULL,
    "approval_status" VARCHAR(20) NOT NULL,
    "created_by" INTEGER NOT NULL,
    "reviewed_by" INTEGER,
    "reviewed_at" TIMESTAMP,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_ar_invoices_customer" FOREIGN KEY ("customer_id") REFERENCES "customers" ("id"),
    CONSTRAINT "fk_ar_invoices_created_by" FOREIGN KEY ("created_by") REFERENCES "users" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_ar_invoices_customer" ON "ar_invoices" ("customer_id");
CREATE INDEX IF NOT EXISTS "idx_ar_invoices_date" ON "ar_invoices" ("invoice_date");
CREATE INDEX IF NOT EXISTS "idx_ar_invoices_status" ON "ar_invoices" ("status");
COMMENT ON TABLE "ar_invoices" IS '应收发票表';

-- ============================================
-- 4. 应收对账表 (ar_reconciliations)
-- ============================================
CREATE TABLE IF NOT EXISTS "ar_reconciliations" (
    "id" SERIAL PRIMARY KEY,
    "reconciliation_no" VARCHAR(50) NOT NULL,
    "reconciliation_date" DATE NOT NULL,
    "period_start" DATE NOT NULL,
    "period_end" DATE NOT NULL,
    "customer_id" INTEGER NOT NULL,
    "customer_name" VARCHAR(200),
    "opening_balance" DECIMAL(18, 2) NOT NULL,
    "total_invoices" DECIMAL(18, 2) NOT NULL,
    "total_collections" DECIMAL(18, 2) NOT NULL,
    "closing_balance" DECIMAL(18, 2) NOT NULL,
    "reconciliation_status" VARCHAR(20),
    "confirmed_by_customer" BOOLEAN,
    "dispute_reason" TEXT,
    "confirmed_by" INTEGER,
    "confirmed_at" TIMESTAMP,
    "created_by" INTEGER,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_ar_reconciliations_customer" FOREIGN KEY ("customer_id") REFERENCES "customers" ("id"),
    CONSTRAINT "fk_ar_reconciliations_created_by" FOREIGN KEY ("created_by") REFERENCES "users" ("id"),
    CONSTRAINT "fk_ar_reconciliations_confirmed_by" FOREIGN KEY ("confirmed_by") REFERENCES "users" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_ar_reconciliations_customer" ON "ar_reconciliations" ("customer_id");
CREATE INDEX IF NOT EXISTS "idx_ar_reconciliations_date" ON "ar_reconciliations" ("reconciliation_date");
CREATE INDEX IF NOT EXISTS "idx_ar_reconciliations_status" ON "ar_reconciliations" ("reconciliation_status");
COMMENT ON TABLE "ar_reconciliations" IS '应收对账表';

-- ============================================
-- 5. 应付对账表 (ap_reconciliation)
-- ============================================
CREATE TABLE IF NOT EXISTS "ap_reconciliation" (
    "id" SERIAL PRIMARY KEY,
    "reconciliation_no" VARCHAR(50) NOT NULL UNIQUE,
    "supplier_id" INTEGER NOT NULL,
    "start_date" DATE NOT NULL,
    "end_date" DATE NOT NULL,
    "opening_balance" DECIMAL(18, 2) NOT NULL DEFAULT 0.00,
    "total_invoice" DECIMAL(18, 2) NOT NULL DEFAULT 0.00,
    "total_payment" DECIMAL(18, 2) NOT NULL DEFAULT 0.00,
    "closing_balance" DECIMAL(18, 2) NOT NULL DEFAULT 0.00,
    "reconciliation_status" VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    "notes" TEXT,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "confirmed_by" INTEGER,
    "confirmed_at" TIMESTAMP,
    "disputed_by" INTEGER,
    "disputed_at" TIMESTAMP,
    "disputed_reason" TEXT,
    CONSTRAINT "fk_ap_reconciliation_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id"),
    CONSTRAINT "fk_ap_reconciliation_created_by" FOREIGN KEY ("created_by") REFERENCES "users" ("id"),
    CONSTRAINT "fk_ap_reconciliation_confirmed_by" FOREIGN KEY ("confirmed_by") REFERENCES "users" ("id"),
    CONSTRAINT "fk_ap_reconciliation_disputed_by" FOREIGN KEY ("disputed_by") REFERENCES "users" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_ap_reconciliation_supplier" ON "ap_reconciliation" ("supplier_id");
CREATE INDEX IF NOT EXISTS "idx_ap_reconciliation_status" ON "ap_reconciliation" ("reconciliation_status");
COMMENT ON TABLE "ap_reconciliation" IS '应付对账表';

-- ============================================
-- 6. 预算计划表 (budget_plans)
-- ============================================
CREATE TABLE IF NOT EXISTS "budget_plans" (
    "id" SERIAL PRIMARY KEY,
    "plan_no" VARCHAR(50) NOT NULL,
    "plan_name" VARCHAR(200) NOT NULL,
    "budget_year" INTEGER NOT NULL,
    "budget_type" VARCHAR(50) NOT NULL,
    "department_id" INTEGER,
    "total_amount" DECIMAL(15, 2) NOT NULL,
    "status" VARCHAR(20),
    "prepared_by" INTEGER,
    "approved_by" INTEGER,
    "approved_at" TIMESTAMP,
    "remark" TEXT,
    "created_at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_budget_plans_department" FOREIGN KEY ("department_id") REFERENCES "departments" ("id"),
    CONSTRAINT "fk_budget_plans_prepared_by" FOREIGN KEY ("prepared_by") REFERENCES "users" ("id"),
    CONSTRAINT "fk_budget_plans_approved_by" FOREIGN KEY ("approved_by") REFERENCES "users" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_budget_plans_year" ON "budget_plans" ("budget_year");
CREATE INDEX IF NOT EXISTS "idx_budget_plans_type" ON "budget_plans" ("budget_type");
CREATE INDEX IF NOT EXISTS "idx_budget_plans_status" ON "budget_plans" ("status");
COMMENT ON TABLE "budget_plans" IS '预算计划表';

-- ============================================
-- 7. 预算科目表 (budget_items)
-- ============================================
CREATE TABLE IF NOT EXISTS "budget_items" (
    "id" SERIAL PRIMARY KEY,
    "item_code" VARCHAR(50) NOT NULL,
    "item_name" VARCHAR(100) NOT NULL,
    "parent_id" INTEGER,
    "item_type" VARCHAR(50) NOT NULL,
    "level" INTEGER NOT NULL,
    "status" VARCHAR(20) NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_budget_items_parent" ON "budget_items" ("parent_id");
CREATE INDEX IF NOT EXISTS "idx_budget_items_type" ON "budget_items" ("item_type");
COMMENT ON TABLE "budget_items" IS '预算科目表';

-- ============================================
-- 8. 固定资产表 (fixed_assets)
-- ============================================
CREATE TABLE IF NOT EXISTS "fixed_assets" (
    "id" SERIAL PRIMARY KEY,
    "asset_no" VARCHAR(50) NOT NULL,
    "asset_name" VARCHAR(200) NOT NULL,
    "asset_category" VARCHAR(50),
    "specification" VARCHAR(200),
    "model" VARCHAR(100),
    "use_department_id" INTEGER,
    "use_location" VARCHAR(200),
    "responsible_person_id" INTEGER,
    "original_value" DECIMAL(15, 2) NOT NULL,
    "salvage_value" DECIMAL(15, 2),
    "salvage_rate" DECIMAL(5, 2),
    "depreciable_value" DECIMAL(15, 2),
    "depreciation_method" VARCHAR(50),
    "useful_life" INTEGER,
    "monthly_depreciation" DECIMAL(15, 2),
    "accumulated_depreciation" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "net_value" DECIMAL(15, 2),
    "status" VARCHAR(20) NOT NULL,
    "purchase_date" DATE,
    "in_service_date" DATE,
    "disposal_date" DATE,
    "supplier_id" INTEGER,
    "supplier_name" VARCHAR(200),
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_fixed_assets_department" FOREIGN KEY ("use_department_id") REFERENCES "departments" ("id"),
    CONSTRAINT "fk_fixed_assets_responsible" FOREIGN KEY ("responsible_person_id") REFERENCES "users" ("id"),
    CONSTRAINT "fk_fixed_assets_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id"),
    CONSTRAINT "fk_fixed_assets_created_by" FOREIGN KEY ("created_by") REFERENCES "users" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_fixed_assets_category" ON "fixed_assets" ("asset_category");
CREATE INDEX IF NOT EXISTS "idx_fixed_assets_status" ON "fixed_assets" ("status");
CREATE INDEX IF NOT EXISTS "idx_fixed_assets_department" ON "fixed_assets" ("use_department_id");
COMMENT ON TABLE "fixed_assets" IS '固定资产表';

-- ============================================
-- 9. 资金账户表 (fund_accounts)
-- ============================================
CREATE TABLE IF NOT EXISTS "fund_accounts" (
    "id" SERIAL PRIMARY KEY,
    "account_no" VARCHAR(50) NOT NULL UNIQUE,
    "account_name" VARCHAR(100) NOT NULL,
    "account_type" VARCHAR(20) NOT NULL,
    "bank_name" VARCHAR(100),
    "bank_account" VARCHAR(50),
    "balance" DECIMAL(15, 2) NOT NULL,
    "currency" VARCHAR(10) NOT NULL DEFAULT 'CNY',
    "remarks" TEXT,
    "is_active" BOOLEAN NOT NULL DEFAULT true,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_fund_accounts_created_by" FOREIGN KEY ("created_by") REFERENCES "users" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_fund_accounts_type" ON "fund_accounts" ("account_type");
CREATE INDEX IF NOT EXISTS "idx_fund_accounts_active" ON "fund_accounts" ("is_active");
COMMENT ON TABLE "fund_accounts" IS '资金账户表';
