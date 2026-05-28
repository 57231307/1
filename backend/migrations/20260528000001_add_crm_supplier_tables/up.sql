-- 批次2：CRM和供应商模块数据库迁移
-- 创建时间: 2026-05-28
-- 描述: 创建CRM线索、商机、客户信用、供应商联系人、资质、产品、评估、采购合同、销售合同表

-- ============================================
-- 1. CRM线索表
-- ============================================
CREATE TABLE IF NOT EXISTS "crm_lead" (
    "id" SERIAL PRIMARY KEY,
    "lead_no" VARCHAR(50) NOT NULL UNIQUE,
    "lead_source" VARCHAR(50) NOT NULL,
    "lead_status" VARCHAR(20) DEFAULT 'new',
    "company_name" VARCHAR(200),
    "contact_name" VARCHAR(100) NOT NULL,
    "contact_title" VARCHAR(100),
    "mobile_phone" VARCHAR(20),
    "tel_phone" VARCHAR(20),
    "email" VARCHAR(100),
    "wechat" VARCHAR(50),
    "qq" VARCHAR(30),
    "address" TEXT,
    "product_interest" TEXT,
    "estimated_quantity" DECIMAL(18,2),
    "estimated_amount" DECIMAL(18,2),
    "expected_delivery_date" DATE,
    "requirement_desc" TEXT,
    "owner_id" INTEGER NOT NULL,
    "owner_name" VARCHAR(100) NOT NULL,
    "last_follow_up_date" DATE,
    "next_follow_up_date" DATE,
    "follow_up_plan" TEXT,
    "converted_at" TIMESTAMPTZ,
    "converted_customer_id" INTEGER,
    "converted_opportunity_id" INTEGER,
    "lost_reason" TEXT,
    "priority" VARCHAR(20) DEFAULT 'medium',
    "rating" INTEGER,
    "tags" TEXT[],
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "created_by" INTEGER,
    "updated_by" INTEGER,
    CONSTRAINT "fk_crm_lead_customer" FOREIGN KEY ("converted_customer_id") REFERENCES "customers" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_crm_lead_owner" ON "crm_lead" ("owner_id");
CREATE INDEX IF NOT EXISTS "idx_crm_lead_status" ON "crm_lead" ("lead_status");
CREATE INDEX IF NOT EXISTS "idx_crm_lead_source" ON "crm_lead" ("lead_source");
COMMENT ON TABLE "crm_lead" IS 'CRM线索表';

-- ============================================
-- 2. CRM商机表
-- ============================================
CREATE TABLE IF NOT EXISTS "crm_opportunity" (
    "id" SERIAL PRIMARY KEY,
    "opportunity_no" VARCHAR(50) NOT NULL UNIQUE,
    "opportunity_name" VARCHAR(200) NOT NULL,
    "customer_id" INTEGER NOT NULL,
    "lead_id" INTEGER,
    "opportunity_type" VARCHAR(50),
    "opportunity_stage" VARCHAR(50),
    "win_probability" DECIMAL(5,2),
    "estimated_amount" DECIMAL(18,2),
    "actual_amount" DECIMAL(18,2),
    "currency" VARCHAR(10) DEFAULT 'CNY',
    "expected_close_date" DATE,
    "actual_close_date" DATE,
    "product_ids" INTEGER[],
    "product_names" TEXT[],
    "product_desc" TEXT,
    "owner_id" INTEGER NOT NULL,
    "owner_name" VARCHAR(100) NOT NULL,
    "last_follow_up_date" DATE,
    "next_follow_up_date" DATE,
    "follow_up_plan" TEXT,
    "competitor_names" TEXT[],
    "competitive_advantage" TEXT,
    "opportunity_status" VARCHAR(20),
    "won_reason" TEXT,
    "lost_reason" TEXT,
    "priority" VARCHAR(20) DEFAULT 'medium',
    "rating" INTEGER,
    "tags" TEXT[],
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "created_by" INTEGER,
    "updated_by" INTEGER,
    CONSTRAINT "fk_crm_opportunity_customer" FOREIGN KEY ("customer_id") REFERENCES "customers" ("id"),
    CONSTRAINT "fk_crm_opportunity_lead" FOREIGN KEY ("lead_id") REFERENCES "crm_lead" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_crm_opportunity_customer" ON "crm_opportunity" ("customer_id");
CREATE INDEX IF NOT EXISTS "idx_crm_opportunity_owner" ON "crm_opportunity" ("owner_id");
CREATE INDEX IF NOT EXISTS "idx_crm_opportunity_stage" ON "crm_opportunity" ("opportunity_stage");
COMMENT ON TABLE "crm_opportunity" IS 'CRM商机表';

-- ============================================
-- 3. 客户信用评级表
-- ============================================
CREATE TABLE IF NOT EXISTS "customer_credit_ratings" (
    "id" SERIAL PRIMARY KEY,
    "customer_id" INTEGER NOT NULL,
    "customer_name" VARCHAR(200),
    "credit_level" VARCHAR(20),
    "credit_score" INTEGER,
    "credit_limit" DECIMAL(18,2) NOT NULL DEFAULT 0.00,
    "used_credit" DECIMAL(18,2) NOT NULL DEFAULT 0.00,
    "available_credit" DECIMAL(18,2) NOT NULL DEFAULT 0.00,
    "credit_days" INTEGER,
    "last_assessment_date" DATE,
    "next_assessment_date" DATE,
    "status" VARCHAR(20) NOT NULL DEFAULT 'active',
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_customer_credit_ratings_customer" FOREIGN KEY ("customer_id") REFERENCES "customers" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_customer_credit_ratings_customer" ON "customer_credit_ratings" ("customer_id");
CREATE INDEX IF NOT EXISTS "idx_customer_credit_ratings_status" ON "customer_credit_ratings" ("status");
COMMENT ON TABLE "customer_credit_ratings" IS '客户信用评级表';

-- ============================================
-- 4. 供应商联系人表
-- ============================================
CREATE TABLE IF NOT EXISTS "supplier_contacts" (
    "id" SERIAL PRIMARY KEY,
    "supplier_id" INTEGER NOT NULL,
    "contact_name" VARCHAR(100) NOT NULL,
    "department" VARCHAR(100),
    "position" VARCHAR(100),
    "mobile_phone" VARCHAR(20) NOT NULL,
    "tel_phone" VARCHAR(20),
    "email" VARCHAR(100),
    "wechat" VARCHAR(50),
    "qq" VARCHAR(30),
    "is_primary" BOOLEAN NOT NULL DEFAULT false,
    "remarks" TEXT,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_supplier_contacts_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_supplier_contacts_supplier" ON "supplier_contacts" ("supplier_id");
COMMENT ON TABLE "supplier_contacts" IS '供应商联系人表';

-- ============================================
-- 5. 供应商资质表
-- ============================================
CREATE TABLE IF NOT EXISTS "supplier_qualifications" (
    "id" SERIAL PRIMARY KEY,
    "supplier_id" INTEGER NOT NULL,
    "qualification_name" VARCHAR(200) NOT NULL,
    "qualification_type" VARCHAR(50) NOT NULL,
    "qualification_no" VARCHAR(100) NOT NULL,
    "issuing_authority" VARCHAR(200) NOT NULL,
    "issue_date" DATE NOT NULL,
    "valid_until" DATE NOT NULL,
    "attachment_path" VARCHAR(500),
    "need_annual_check" BOOLEAN NOT NULL DEFAULT false,
    "annual_check_record" TEXT,
    "is_expired" BOOLEAN NOT NULL DEFAULT false,
    "remarks" TEXT,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_supplier_qualifications_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_supplier_qualifications_supplier" ON "supplier_qualifications" ("supplier_id");
COMMENT ON TABLE "supplier_qualifications" IS '供应商资质表';

-- ============================================
-- 6. 供应商产品表
-- ============================================
CREATE TABLE IF NOT EXISTS "supplier_products" (
    "id" SERIAL PRIMARY KEY,
    "supplier_id" INTEGER NOT NULL,
    "product_code" VARCHAR(100) NOT NULL,
    "product_name" VARCHAR(200) NOT NULL,
    "product_description" TEXT,
    "unit" VARCHAR(20) NOT NULL,
    "is_enabled" BOOLEAN NOT NULL DEFAULT true,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "created_by" INTEGER,
    "updated_by" INTEGER,
    "remarks" TEXT,
    CONSTRAINT "fk_supplier_products_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_supplier_products_supplier" ON "supplier_products" ("supplier_id");
COMMENT ON TABLE "supplier_products" IS '供应商产品表';

-- ============================================
-- 7. 供应商评估指标表
-- ============================================
CREATE TABLE IF NOT EXISTS "supplier_evaluation_indicators" (
    "id" SERIAL PRIMARY KEY,
    "indicator_name" VARCHAR(100) NOT NULL,
    "indicator_code" VARCHAR(50) NOT NULL UNIQUE,
    "category" VARCHAR(50) NOT NULL,
    "weight" DECIMAL(5,2) NOT NULL,
    "max_score" INTEGER NOT NULL,
    "evaluation_method" TEXT,
    "status" VARCHAR(20) NOT NULL DEFAULT 'active',
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_supplier_evaluation_indicators_status" ON "supplier_evaluation_indicators" ("status");
COMMENT ON TABLE "supplier_evaluation_indicators" IS '供应商评估指标表';

-- ============================================
-- 8. 采购合同表
-- ============================================
CREATE TABLE IF NOT EXISTS "purchase_contracts" (
    "id" SERIAL PRIMARY KEY,
    "contract_no" VARCHAR(50) NOT NULL UNIQUE,
    "contract_name" VARCHAR(200) NOT NULL,
    "contract_type" VARCHAR(50),
    "supplier_id" INTEGER NOT NULL,
    "supplier_name" VARCHAR(200),
    "total_amount" DECIMAL(18,2),
    "signed_date" DATE,
    "effective_date" DATE,
    "expiry_date" DATE,
    "payment_terms" TEXT,
    "payment_method" VARCHAR(50),
    "delivery_date" DATE,
    "delivery_location" TEXT,
    "status" VARCHAR(20) NOT NULL DEFAULT 'draft',
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_purchase_contracts_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_purchase_contracts_supplier" ON "purchase_contracts" ("supplier_id");
CREATE INDEX IF NOT EXISTS "idx_purchase_contracts_status" ON "purchase_contracts" ("status");
COMMENT ON TABLE "purchase_contracts" IS '采购合同表';

-- ============================================
-- 9. 销售合同表
-- ============================================
CREATE TABLE IF NOT EXISTS "sales_contracts" (
    "id" SERIAL PRIMARY KEY,
    "contract_no" VARCHAR(50) NOT NULL UNIQUE,
    "contract_name" VARCHAR(200) NOT NULL,
    "contract_type" VARCHAR(50),
    "customer_id" INTEGER NOT NULL,
    "customer_name" VARCHAR(200),
    "total_amount" DECIMAL(18,2),
    "signed_date" DATE,
    "effective_date" DATE,
    "expiry_date" DATE,
    "payment_terms" TEXT,
    "payment_method" VARCHAR(50),
    "delivery_date" DATE,
    "delivery_location" TEXT,
    "status" VARCHAR(20) NOT NULL DEFAULT 'draft',
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_sales_contracts_customer" FOREIGN KEY ("customer_id") REFERENCES "customers" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_sales_contracts_customer" ON "sales_contracts" ("customer_id");
CREATE INDEX IF NOT EXISTS "idx_sales_contracts_status" ON "sales_contracts" ("status");
COMMENT ON TABLE "sales_contracts" IS '销售合同表';