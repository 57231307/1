-- 批次8：财务应收应付 + 多币种 + 财务分析
-- 创建时间: 2026-05-27
-- 描述: 创建财务应收应付、多币种和财务分析相关表

-- ============================================
-- AP应付模块（7张表）
-- ============================================

-- 1. 应付单表
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
    "paid_amount" DECIMAL(18, 2) NOT NULL DEFAULT 0,
    "unpaid_amount" DECIMAL(18, 2) NOT NULL DEFAULT 0,
    "invoice_status" VARCHAR(20) NOT NULL DEFAULT 'DRAFT',
    "currency" VARCHAR(10) NOT NULL DEFAULT 'CNY',
    "exchange_rate" DECIMAL(18, 6) NOT NULL DEFAULT 1.0,
    "amount_foreign" DECIMAL(18, 2),
    "tax_amount" DECIMAL(18, 2) NOT NULL DEFAULT 0,
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
    CONSTRAINT "fk_ap_invoice_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_ap_invoice_supplier" ON "ap_invoice" ("supplier_id");
CREATE INDEX IF NOT EXISTS "idx_ap_invoice_date" ON "ap_invoice" ("invoice_date");
CREATE INDEX IF NOT EXISTS "idx_ap_invoice_status" ON "ap_invoice" ("invoice_status");
COMMENT ON TABLE "ap_invoice" IS '应付单表';

-- 2. 付款申请表
CREATE TABLE IF NOT EXISTS "ap_payment_request" (
    "id" SERIAL PRIMARY KEY,
    "request_no" VARCHAR(50) NOT NULL UNIQUE,
    "request_date" DATE NOT NULL,
    "supplier_id" INTEGER NOT NULL,
    "payment_type" VARCHAR(20) NOT NULL,
    "payment_method" VARCHAR(20) NOT NULL,
    "request_amount" DECIMAL(18, 2) NOT NULL,
    "approval_status" VARCHAR(20) NOT NULL DEFAULT 'DRAFT',
    "currency" VARCHAR(10) NOT NULL DEFAULT 'CNY',
    "exchange_rate" DECIMAL(18, 6) NOT NULL DEFAULT 1.0,
    "request_amount_foreign" DECIMAL(18, 2),
    "expected_payment_date" DATE,
    "bank_name" VARCHAR(100),
    "bank_account" VARCHAR(50),
    "bank_account_name" VARCHAR(100),
    "notes" TEXT,
    "attachment_urls" TEXT,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_by" INTEGER,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "submitted_by" INTEGER,
    "submitted_at" TIMESTAMP,
    "approved_by" INTEGER,
    "approved_at" TIMESTAMP,
    "rejected_by" INTEGER,
    "rejected_at" TIMESTAMP,
    "rejected_reason" TEXT,
    CONSTRAINT "fk_ap_payment_request_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_ap_payment_request_supplier" ON "ap_payment_request" ("supplier_id");
CREATE INDEX IF NOT EXISTS "idx_ap_payment_request_date" ON "ap_payment_request" ("request_date");
COMMENT ON TABLE "ap_payment_request" IS '付款申请表';

-- 3. 付款申请明细表
CREATE TABLE IF NOT EXISTS "ap_payment_request_item" (
    "id" SERIAL PRIMARY KEY,
    "request_id" INTEGER NOT NULL,
    "invoice_id" INTEGER NOT NULL,
    "apply_amount" DECIMAL(18, 2) NOT NULL,
    "notes" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_ap_payment_request_item_request" FOREIGN KEY ("request_id") REFERENCES "ap_payment_request" ("id"),
    CONSTRAINT "fk_ap_payment_request_item_invoice" FOREIGN KEY ("invoice_id") REFERENCES "ap_invoice" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_ap_payment_request_item_request" ON "ap_payment_request_item" ("request_id");
COMMENT ON TABLE "ap_payment_request_item" IS '付款申请明细表';

-- 4. 付款单表
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
    "exchange_rate" DECIMAL(18, 6) NOT NULL DEFAULT 1.0,
    "payment_amount_foreign" DECIMAL(18, 2),
    "bank_name" VARCHAR(100),
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
    CONSTRAINT "fk_ap_payment_request" FOREIGN KEY ("request_id") REFERENCES "ap_payment_request" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_ap_payment_supplier" ON "ap_payment" ("supplier_id");
CREATE INDEX IF NOT EXISTS "idx_ap_payment_date" ON "ap_payment" ("payment_date");
COMMENT ON TABLE "ap_payment" IS '付款单表';

-- 5. 供应商对账单表
CREATE TABLE IF NOT EXISTS "ap_reconciliation" (
    "id" SERIAL PRIMARY KEY,
    "reconciliation_no" VARCHAR(50) NOT NULL UNIQUE,
    "supplier_id" INTEGER NOT NULL,
    "start_date" DATE NOT NULL,
    "end_date" DATE NOT NULL,
    "opening_balance" DECIMAL(18, 2) NOT NULL DEFAULT 0,
    "total_invoice" DECIMAL(18, 2) NOT NULL DEFAULT 0,
    "total_payment" DECIMAL(18, 2) NOT NULL DEFAULT 0,
    "closing_balance" DECIMAL(18, 2) NOT NULL DEFAULT 0,
    "reconciliation_status" VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    "notes" TEXT,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "confirmed_by" INTEGER,
    "confirmed_at" TIMESTAMP,
    "disputed_by" INTEGER,
    "disputed_at" TIMESTAMP,
    "disputed_reason" TEXT,
    CONSTRAINT "fk_ap_reconciliation_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_ap_reconciliation_supplier" ON "ap_reconciliation" ("supplier_id");
COMMENT ON TABLE "ap_reconciliation" IS '供应商对账单表';

-- 6. 应付核销表
CREATE TABLE IF NOT EXISTS "ap_verification" (
    "id" SERIAL PRIMARY KEY,
    "verification_no" VARCHAR(50) NOT NULL UNIQUE,
    "verification_date" DATE NOT NULL,
    "supplier_id" INTEGER NOT NULL,
    "verification_type" VARCHAR(20) NOT NULL,
    "total_amount" DECIMAL(18, 2) NOT NULL,
    "verification_status" VARCHAR(20) NOT NULL DEFAULT 'COMPLETED',
    "notes" TEXT,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "cancelled_by" INTEGER,
    "cancelled_at" TIMESTAMP,
    "cancelled_reason" TEXT,
    CONSTRAINT "fk_ap_verification_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_ap_verification_supplier" ON "ap_verification" ("supplier_id");
COMMENT ON TABLE "ap_verification" IS '应付核销表';

-- 7. 核销明细表
CREATE TABLE IF NOT EXISTS "ap_verification_item" (
    "id" SERIAL PRIMARY KEY,
    "verification_id" INTEGER NOT NULL,
    "invoice_id" INTEGER NOT NULL,
    "payment_id" INTEGER NOT NULL,
    "verify_amount" DECIMAL(18, 2) NOT NULL,
    "notes" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_ap_verification_item_verification" FOREIGN KEY ("verification_id") REFERENCES "ap_verification" ("id"),
    CONSTRAINT "fk_ap_verification_item_invoice" FOREIGN KEY ("invoice_id") REFERENCES "ap_invoice" ("id"),
    CONSTRAINT "fk_ap_verification_item_payment" FOREIGN KEY ("payment_id") REFERENCES "ap_payment" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_ap_verification_item_verification" ON "ap_verification_item" ("verification_id");
COMMENT ON TABLE "ap_verification_item" IS '核销明细表';

-- ============================================
-- AR应收模块（5张表）
-- ============================================

-- 8. 应收单表
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
    "received_amount" DECIMAL(18, 2) NOT NULL DEFAULT 0,
    "unpaid_amount" DECIMAL(18, 2) NOT NULL DEFAULT 0,
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
    CONSTRAINT "fk_ar_invoices_customer" FOREIGN KEY ("customer_id") REFERENCES "customers" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_ar_invoices_customer" ON "ar_invoices" ("customer_id");
CREATE INDEX IF NOT EXISTS "idx_ar_invoices_date" ON "ar_invoices" ("invoice_date");
COMMENT ON TABLE "ar_invoices" IS '应收单表';

-- 9. 收款单表
CREATE TABLE IF NOT EXISTS "ar_collections" (
    "id" SERIAL PRIMARY KEY,
    "collection_no" VARCHAR(50) NOT NULL,
    "collection_date" DATE NOT NULL,
    "customer_id" INTEGER NOT NULL,
    "customer_name" VARCHAR(200),
    "collection_amount" DECIMAL(18, 2) NOT NULL,
    "collection_method" VARCHAR(50),
    "bank_account" VARCHAR(50),
    "check_no" VARCHAR(50),
    "request_id" INTEGER,
    "request_no" VARCHAR(50),
    "status" VARCHAR(20) NOT NULL,
    "confirmed_by" INTEGER,
    "confirmed_at" TIMESTAMP,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_ar_collections_customer" ON "ar_collections" ("customer_id");
CREATE INDEX IF NOT EXISTS "idx_ar_collections_date" ON "ar_collections" ("collection_date");
COMMENT ON TABLE "ar_collections" IS '收款单表';

-- 10. 应收对账单表
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
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_ar_reconciliations_customer" ON "ar_reconciliations" ("customer_id");
COMMENT ON TABLE "ar_reconciliations" IS '应收对账单表';

-- 11. 应收对账明细表
CREATE TABLE IF NOT EXISTS "ar_reconciliation_items" (
    "id" SERIAL PRIMARY KEY,
    "reconciliation_id" INTEGER NOT NULL,
    "item_type" VARCHAR(20) NOT NULL,
    "document_type" VARCHAR(50),
    "document_id" INTEGER,
    "document_no" VARCHAR(50),
    "document_date" DATE,
    "amount" DECIMAL(18, 2) NOT NULL,
    "matched_amount" DECIMAL(18, 2),
    "match_status" VARCHAR(20) NOT NULL,
    "matched_item_id" INTEGER,
    "remarks" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_ar_reconciliation_items_reconciliation" FOREIGN KEY ("reconciliation_id") REFERENCES "ar_reconciliations" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_ar_reconciliation_items_reconciliation" ON "ar_reconciliation_items" ("reconciliation_id");
COMMENT ON TABLE "ar_reconciliation_items" IS '应收对账明细表';

-- 12. 应收账龄分析表
CREATE TABLE IF NOT EXISTS "ar_aging_analysis" (
    "id" SERIAL PRIMARY KEY,
    "customer_id" INTEGER NOT NULL,
    "analysis_date" DATE NOT NULL,
    "current_amount" DECIMAL(18, 2) NOT NULL,
    "days_1_30" DECIMAL(18, 2) NOT NULL,
    "days_31_60" DECIMAL(18, 2) NOT NULL,
    "days_61_90" DECIMAL(18, 2) NOT NULL,
    "days_over_90" DECIMAL(18, 2) NOT NULL,
    "total_amount" DECIMAL(18, 2) NOT NULL,
    "salesperson_id" INTEGER,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_ar_aging_analysis_customer" ON "ar_aging_analysis" ("customer_id");
CREATE INDEX IF NOT EXISTS "idx_ar_aging_analysis_date" ON "ar_aging_analysis" ("analysis_date");
COMMENT ON TABLE "ar_aging_analysis" IS '应收账龄分析表';

-- ============================================
-- 财务分析 + 成本 + 预算 + 固定资产 + 资金（13张表）
-- ============================================

-- 13. 财务指标表
CREATE TABLE IF NOT EXISTS "financial_indicators" (
    "id" SERIAL PRIMARY KEY,
    "indicator_name" VARCHAR(100) NOT NULL,
    "indicator_code" VARCHAR(50) NOT NULL UNIQUE,
    "indicator_type" VARCHAR(50) NOT NULL,
    "formula" TEXT,
    "unit" VARCHAR(20),
    "status" VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
    "remark" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "financial_indicators" IS '财务指标表';

-- 14. 财务分析结果表
CREATE TABLE IF NOT EXISTS "financial_analysis_results" (
    "id" SERIAL PRIMARY KEY,
    "analysis_type" VARCHAR(50) NOT NULL,
    "period" VARCHAR(20) NOT NULL,
    "indicator_id" INTEGER NOT NULL,
    "indicator_value" DECIMAL(18, 4) NOT NULL,
    "target_value" DECIMAL(18, 4),
    "variance" DECIMAL(18, 4),
    "variance_rate" DECIMAL(5, 2),
    "trend" VARCHAR(20),
    "analysis_date" DATE,
    "created_by" INTEGER,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_financial_analysis_results_type" ON "financial_analysis_results" ("analysis_type");
CREATE INDEX IF NOT EXISTS "idx_financial_analysis_results_period" ON "financial_analysis_results" ("period");
COMMENT ON TABLE "financial_analysis_results" IS '财务分析结果表';

-- 15. 成本归集表
CREATE TABLE IF NOT EXISTS "cost_collections" (
    "id" SERIAL PRIMARY KEY,
    "collection_no" VARCHAR(50) NOT NULL,
    "collection_date" DATE NOT NULL,
    "cost_object_type" VARCHAR(50),
    "cost_object_id" INTEGER,
    "cost_object_no" VARCHAR(50),
    "batch_no" VARCHAR(50),
    "color_no" VARCHAR(50),
    "dye_lot_no" VARCHAR(50),
    "workshop" VARCHAR(50),
    "production_order_no" VARCHAR(50),
    "direct_material" DECIMAL(15, 2) NOT NULL,
    "direct_labor" DECIMAL(15, 2) NOT NULL,
    "manufacturing_overhead" DECIMAL(15, 2) NOT NULL,
    "processing_fee" DECIMAL(15, 2) NOT NULL,
    "dyeing_fee" DECIMAL(15, 2) NOT NULL,
    "total_cost" DECIMAL(15, 2) NOT NULL,
    "output_quantity_meters" DECIMAL(12, 2),
    "output_quantity_kg" DECIMAL(12, 2),
    "unit_cost_meters" DECIMAL(18, 6),
    "unit_cost_kg" DECIMAL(18, 6),
    "status" VARCHAR(20) NOT NULL,
    "created_by" INTEGER,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "cost_collections" IS '成本归集表';

-- 16. 成本分析表
CREATE TABLE IF NOT EXISTS "cost_analyses" (
    "id" SERIAL PRIMARY KEY,
    "analysis_no" VARCHAR(50) NOT NULL,
    "analysis_date" DATE NOT NULL,
    "period" VARCHAR(20) NOT NULL,
    "batch_no" VARCHAR(50),
    "color_no" VARCHAR(50),
    "workshop" VARCHAR(50),
    "total_direct_material" DECIMAL(15, 2) NOT NULL,
    "total_direct_labor" DECIMAL(15, 2) NOT NULL,
    "total_overhead" DECIMAL(15, 2) NOT NULL,
    "total_processing_fee" DECIMAL(15, 2) NOT NULL,
    "total_dyeing_fee" DECIMAL(15, 2) NOT NULL,
    "total_cost" DECIMAL(15, 2) NOT NULL,
    "total_output_meters" DECIMAL(12, 2),
    "total_output_kg" DECIMAL(12, 2),
    "avg_unit_cost_meters" DECIMAL(18, 6),
    "avg_unit_cost_kg" DECIMAL(18, 6),
    "standard_cost" DECIMAL(15, 2),
    "variance" DECIMAL(15, 2),
    "variance_rate" DECIMAL(5, 2),
    "conclusion" TEXT,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "cost_analyses" IS '成本分析表';

-- 17. 预算方案表
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
    "updated_at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "budget_plans" IS '预算方案表';

-- 18. 预算科目表
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

COMMENT ON TABLE "budget_items" IS '预算科目表';

-- 19. 预算执行表
CREATE TABLE IF NOT EXISTS "budget_executions" (
    "id" SERIAL PRIMARY KEY,
    "plan_id" INTEGER NOT NULL,
    "execution_type" VARCHAR(20) NOT NULL,
    "amount" DECIMAL(15, 2) NOT NULL,
    "expense_type" VARCHAR(50),
    "expense_date" DATE NOT NULL,
    "related_document_type" VARCHAR(50),
    "related_document_id" INTEGER,
    "remark" TEXT,
    "created_by" INTEGER,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_budget_executions_plan" FOREIGN KEY ("plan_id") REFERENCES "budget_plans" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_budget_executions_plan" ON "budget_executions" ("plan_id");
COMMENT ON TABLE "budget_executions" IS '预算执行表';

-- 20. 预算调整表
CREATE TABLE IF NOT EXISTS "budget_adjustments" (
    "id" SERIAL PRIMARY KEY,
    "adjustment_no" VARCHAR(50) NOT NULL UNIQUE,
    "budget_id" INTEGER NOT NULL,
    "adjustment_date" DATE NOT NULL,
    "adjustment_type" VARCHAR(20) NOT NULL,
    "amount" DECIMAL(15, 2) NOT NULL,
    "budget_before" DECIMAL(15, 2) NOT NULL,
    "budget_after" DECIMAL(15, 2) NOT NULL,
    "reason" TEXT NOT NULL,
    "approval_status" VARCHAR(20) NOT NULL,
    "remarks" TEXT,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_budget_adjustments_budget" FOREIGN KEY ("budget_id") REFERENCES "budget_plans" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_budget_adjustments_budget" ON "budget_adjustments" ("budget_id");
COMMENT ON TABLE "budget_adjustments" IS '预算调整表';

-- 21. 固定资产表
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
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "fixed_assets" IS '固定资产表';

-- 22. 固定资产处置表
CREATE TABLE IF NOT EXISTS "fixed_asset_disposals" (
    "id" SERIAL PRIMARY KEY,
    "disposal_no" VARCHAR(50) NOT NULL UNIQUE,
    "asset_id" INTEGER NOT NULL,
    "disposal_date" DATE NOT NULL,
    "disposal_type" VARCHAR(20) NOT NULL,
    "quantity" INTEGER NOT NULL,
    "disposal_amount" DECIMAL(15, 2) NOT NULL,
    "disposal_reason" TEXT NOT NULL,
    "status" VARCHAR(20) NOT NULL,
    "remarks" TEXT,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_fixed_asset_disposals_asset" FOREIGN KEY ("asset_id") REFERENCES "fixed_assets" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_fixed_asset_disposals_asset" ON "fixed_asset_disposals" ("asset_id");
COMMENT ON TABLE "fixed_asset_disposals" IS '固定资产处置表';

-- 23. 资金账户表
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
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "fund_accounts" IS '资金账户表';

-- 24. 资金转账记录表
CREATE TABLE IF NOT EXISTS "fund_transfers" (
    "id" SERIAL PRIMARY KEY,
    "transfer_no" VARCHAR(50) NOT NULL,
    "from_account_id" INTEGER,
    "to_account_id" INTEGER,
    "amount" DECIMAL(18, 2) NOT NULL,
    "transfer_type" VARCHAR(20) NOT NULL,
    "transfer_date" DATE NOT NULL,
    "purpose" TEXT,
    "status" VARCHAR(20),
    "applied_by" INTEGER,
    "approved_by" INTEGER,
    "approved_at" TIMESTAMP,
    "executed_at" TIMESTAMP,
    "remark" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_fund_transfers_from_account" FOREIGN KEY ("from_account_id") REFERENCES "fund_accounts" ("id"),
    CONSTRAINT "fk_fund_transfers_to_account" FOREIGN KEY ("to_account_id") REFERENCES "fund_accounts" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_fund_transfers_from" ON "fund_transfers" ("from_account_id");
CREATE INDEX IF NOT EXISTS "idx_fund_transfers_to" ON "fund_transfers" ("to_account_id");
COMMENT ON TABLE "fund_transfers" IS '资金转账记录表';

-- 25. 客户信用评级表
CREATE TABLE IF NOT EXISTS "customer_credit_ratings" (
    "id" SERIAL PRIMARY KEY,
    "customer_id" INTEGER NOT NULL,
    "customer_name" VARCHAR(200),
    "credit_level" VARCHAR(20),
    "credit_score" INTEGER,
    "credit_limit" DECIMAL(15, 2) NOT NULL,
    "used_credit" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "available_credit" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "credit_days" INTEGER,
    "last_assessment_date" DATE,
    "next_assessment_date" DATE,
    "status" VARCHAR(20) NOT NULL,
    "created_by" INTEGER,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_customer_credit_ratings_customer" ON "customer_credit_ratings" ("customer_id");
COMMENT ON TABLE "customer_credit_ratings" IS '客户信用评级表';
