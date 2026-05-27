-- 批次2：总账与财务基础
-- 创建时间: 2026-05-27
-- 描述: 创建总账和财务基础表

-- ============================================
-- 1. 会计科目表（自引用）
-- ============================================
CREATE TABLE IF NOT EXISTS "account_subjects" (
    "id" SERIAL PRIMARY KEY,
    "code" VARCHAR(50) NOT NULL UNIQUE,
    "name" VARCHAR(200) NOT NULL,
    "level" INTEGER NOT NULL DEFAULT 1,
    "parent_id" INTEGER,
    "full_code" VARCHAR(200),
    "balance_direction" VARCHAR(20) DEFAULT 'debit',
    "initial_balance_debit" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "initial_balance_credit" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "current_period_debit" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "current_period_credit" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "ending_balance_debit" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "ending_balance_credit" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "assist_customer" BOOLEAN NOT NULL DEFAULT false,
    "assist_supplier" BOOLEAN NOT NULL DEFAULT false,
    "assist_department" BOOLEAN NOT NULL DEFAULT false,
    "assist_employee" BOOLEAN NOT NULL DEFAULT false,
    "assist_project" BOOLEAN NOT NULL DEFAULT false,
    "assist_batch" BOOLEAN NOT NULL DEFAULT false,
    "assist_color_no" BOOLEAN NOT NULL DEFAULT false,
    "assist_dye_lot" BOOLEAN NOT NULL DEFAULT false,
    "assist_grade" BOOLEAN NOT NULL DEFAULT false,
    "assist_workshop" BOOLEAN NOT NULL DEFAULT false,
    "enable_dual_unit" BOOLEAN NOT NULL DEFAULT false,
    "primary_unit" VARCHAR(20),
    "secondary_unit" VARCHAR(20),
    "is_cash_account" BOOLEAN NOT NULL DEFAULT false,
    "is_bank_account" BOOLEAN NOT NULL DEFAULT false,
    "allow_manual_entry" BOOLEAN NOT NULL DEFAULT true,
    "require_summary" BOOLEAN NOT NULL DEFAULT false,
    "status" VARCHAR(20) NOT NULL DEFAULT 'active',
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_account_subjects_parent" FOREIGN KEY ("parent_id") REFERENCES "account_subjects" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_account_subjects_parent" ON "account_subjects" ("parent_id");
CREATE INDEX IF NOT EXISTS "idx_account_subjects_code" ON "account_subjects" ("code");
COMMENT ON TABLE "account_subjects" IS '会计科目表 - 存储会计科目信息';

-- ============================================
-- 2. 账户余额表
-- ============================================
CREATE TABLE IF NOT EXISTS "account_balances" (
    "id" SERIAL PRIMARY KEY,
    "subject_id" INTEGER NOT NULL,
    "period" VARCHAR(10) NOT NULL,
    "initial_balance_debit" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "initial_balance_credit" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "current_period_debit" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "current_period_credit" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "ending_balance_debit" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "ending_balance_credit" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_account_balances_subject" ON "account_balances" ("subject_id");
CREATE INDEX IF NOT EXISTS "idx_account_balances_period" ON "account_balances" ("period");
CREATE UNIQUE INDEX IF NOT EXISTS "idx_account_balances_unique" ON "account_balances" ("subject_id", "period");
COMMENT ON TABLE "account_balances" IS '账户余额表 - 存储科目余额信息';

-- ============================================
-- 3. 会计期间表
-- ============================================
CREATE TABLE IF NOT EXISTS "accounting_periods" (
    "id" SERIAL PRIMARY KEY,
    "year" INTEGER NOT NULL,
    "period" INTEGER NOT NULL,
    "period_name" VARCHAR(20) NOT NULL,
    "start_date" DATE NOT NULL,
    "end_date" DATE NOT NULL,
    "status" VARCHAR(20) NOT NULL DEFAULT 'OPEN',
    "closed_at" TIMESTAMP,
    "closed_by" INTEGER,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS "idx_accounting_periods_unique" ON "accounting_periods" ("year", "period");
COMMENT ON TABLE "accounting_periods" IS '会计期间表 - 存储会计期间信息';

-- ============================================
-- 4. 凭证表
-- ============================================
CREATE TABLE IF NOT EXISTS "vouchers" (
    "id" SERIAL PRIMARY KEY,
    "voucher_no" VARCHAR(50) NOT NULL UNIQUE,
    "voucher_type" VARCHAR(20) NOT NULL,
    "voucher_date" DATE NOT NULL,
    "source_type" VARCHAR(50),
    "source_module" VARCHAR(50),
    "source_bill_id" INTEGER,
    "source_bill_no" VARCHAR(50),
    "batch_no" VARCHAR(50),
    "color_no" VARCHAR(50),
    "dye_lot_no" VARCHAR(50),
    "workshop" VARCHAR(50),
    "production_order_no" VARCHAR(50),
    "quantity_meters" DECIMAL(12, 2),
    "quantity_kg" DECIMAL(12, 2),
    "gram_weight" DECIMAL(10, 2),
    "status" VARCHAR(20) NOT NULL DEFAULT 'draft',
    "attachment_count" INTEGER NOT NULL DEFAULT 0,
    "created_by" INTEGER NOT NULL,
    "reviewed_by" INTEGER,
    "reviewed_at" TIMESTAMP,
    "posted_by" INTEGER,
    "posted_at" TIMESTAMP,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_vouchers_date" ON "vouchers" ("voucher_date");
CREATE INDEX IF NOT EXISTS "idx_vouchers_status" ON "vouchers" ("status");
CREATE INDEX IF NOT EXISTS "idx_vouchers_created_by" ON "vouchers" ("created_by");
COMMENT ON TABLE "vouchers" IS '凭证表 - 存储会计凭证';

-- ============================================
-- 5. 凭证分录表
-- ============================================
CREATE TABLE IF NOT EXISTS "voucher_items" (
    "id" SERIAL PRIMARY KEY,
    "voucher_id" INTEGER NOT NULL,
    "line_no" INTEGER NOT NULL,
    "subject_code" VARCHAR(50) NOT NULL,
    "subject_name" VARCHAR(200) NOT NULL,
    "debit" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "credit" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "summary" TEXT,
    "assist_customer_id" INTEGER,
    "assist_supplier_id" INTEGER,
    "assist_department_id" INTEGER,
    "assist_employee_id" INTEGER,
    "assist_project_id" INTEGER,
    "assist_batch_id" INTEGER,
    "assist_color_no_id" INTEGER,
    "assist_dye_lot_id" INTEGER,
    "assist_grade" VARCHAR(20),
    "assist_workshop_id" INTEGER,
    "quantity_meters" DECIMAL(12, 2),
    "quantity_kg" DECIMAL(12, 2),
    "unit_price" DECIMAL(12, 4),
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_voucher_items_voucher" FOREIGN KEY ("voucher_id") REFERENCES "vouchers" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_voucher_items_voucher" ON "voucher_items" ("voucher_id");
CREATE INDEX IF NOT EXISTS "idx_voucher_items_subject" ON "voucher_items" ("subject_code");
COMMENT ON TABLE "voucher_items" IS '凭证分录表 - 存储凭证明细';

-- ============================================
-- 6. 辅助核算维度表
-- ============================================
CREATE TABLE IF NOT EXISTS "assist_accounting_dimension" (
    "id" SERIAL PRIMARY KEY,
    "dimension_code" VARCHAR(50) NOT NULL UNIQUE,
    "dimension_name" VARCHAR(100) NOT NULL,
    "description" TEXT,
    "is_active" BOOLEAN NOT NULL DEFAULT true,
    "sort_order" INTEGER NOT NULL DEFAULT 0,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "assist_accounting_dimension" IS '辅助核算维度表 - 存储核算维度定义';

-- ============================================
-- 7. 辅助核算记录表
-- ============================================
CREATE TABLE IF NOT EXISTS "assist_accounting_record" (
    "id" SERIAL PRIMARY KEY,
    "business_type" VARCHAR(50) NOT NULL,
    "business_no" VARCHAR(50) NOT NULL,
    "business_id" INTEGER NOT NULL,
    "account_subject_id" INTEGER NOT NULL,
    "debit_amount" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "credit_amount" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "five_dimension_id" VARCHAR(100) NOT NULL,
    "product_id" INTEGER NOT NULL,
    "batch_no" VARCHAR(50) NOT NULL,
    "color_no" VARCHAR(50) NOT NULL,
    "dye_lot_no" VARCHAR(50),
    "grade" VARCHAR(20) NOT NULL,
    "workshop_id" INTEGER,
    "warehouse_id" INTEGER NOT NULL,
    "customer_id" INTEGER,
    "supplier_id" INTEGER,
    "quantity_meters" DECIMAL(12, 2) NOT NULL DEFAULT 0,
    "quantity_kg" DECIMAL(12, 2) NOT NULL DEFAULT 0,
    "remarks" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "created_by" INTEGER,
    CONSTRAINT "fk_assist_accounting_record_subject" FOREIGN KEY ("account_subject_id") REFERENCES "account_subjects" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_assist_accounting_record_business" ON "assist_accounting_record" ("business_type", "business_id");
CREATE INDEX IF NOT EXISTS "idx_assist_accounting_record_subject" ON "assist_accounting_record" ("account_subject_id");
CREATE INDEX IF NOT EXISTS "idx_assist_accounting_record_product" ON "assist_accounting_record" ("product_id");
COMMENT ON TABLE "assist_accounting_record" IS '辅助核算记录表 - 存储辅助核算明细';

-- ============================================
-- 8. 辅助核算汇总表
-- ============================================
CREATE TABLE IF NOT EXISTS "assist_accounting_summary" (
    "id" SERIAL PRIMARY KEY,
    "accounting_period" VARCHAR(10) NOT NULL,
    "dimension_code" VARCHAR(50) NOT NULL,
    "dimension_value_id" INTEGER NOT NULL,
    "dimension_value_name" VARCHAR(100) NOT NULL,
    "account_subject_id" INTEGER NOT NULL,
    "total_debit" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "total_credit" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "total_quantity_meters" DECIMAL(12, 2) NOT NULL DEFAULT 0,
    "total_quantity_kg" DECIMAL(12, 2) NOT NULL DEFAULT 0,
    "record_count" BIGINT NOT NULL DEFAULT 0,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_assist_accounting_summary_period" ON "assist_accounting_summary" ("accounting_period");
CREATE INDEX IF NOT EXISTS "idx_assist_accounting_summary_dimension" ON "assist_accounting_summary" ("dimension_code", "dimension_value_id");
COMMENT ON TABLE "assist_accounting_summary" IS '辅助核算汇总表 - 存储辅助核算汇总数据';
