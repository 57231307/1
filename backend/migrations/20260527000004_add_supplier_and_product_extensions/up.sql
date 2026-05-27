-- 批次4：供应商管理扩展 + 产品扩展
-- 创建时间: 2026-05-27
-- 描述: 创建供应商管理扩展表和产品扩展表

-- ============================================
-- 1. 供应商联系人表
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
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_supplier_contacts_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_supplier_contacts_supplier" ON "supplier_contacts" ("supplier_id");
COMMENT ON TABLE "supplier_contacts" IS '供应商联系人表';

-- ============================================
-- 2. 供应商黑名单表
-- ============================================
CREATE TABLE IF NOT EXISTS "supplier_blacklists" (
    "id" SERIAL PRIMARY KEY,
    "supplier_id" INTEGER NOT NULL,
    "blacklist_date" DATE NOT NULL,
    "blacklist_reason" VARCHAR(200) NOT NULL,
    "detail_description" TEXT NOT NULL,
    "evidence" TEXT,
    "approver_id" INTEGER NOT NULL,
    "approval_date" DATE NOT NULL,
    "is_permanent" BOOLEAN NOT NULL DEFAULT false,
    "release_date" DATE,
    "release_condition" TEXT,
    "release_status" VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    "release_date_actual" DATE,
    "remarks" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "created_by" INTEGER,
    "updated_by" INTEGER,
    CONSTRAINT "fk_supplier_blacklists_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_supplier_blacklists_supplier" ON "supplier_blacklists" ("supplier_id");
COMMENT ON TABLE "supplier_blacklists" IS '供应商黑名单表';

-- ============================================
-- 3. 供应商资质表
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
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_supplier_qualifications_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_supplier_qualifications_supplier" ON "supplier_qualifications" ("supplier_id");
COMMENT ON TABLE "supplier_qualifications" IS '供应商资质表';

-- ============================================
-- 4. 供应商产品表
-- ============================================
CREATE TABLE IF NOT EXISTS "supplier_products" (
    "id" SERIAL PRIMARY KEY,
    "supplier_id" INTEGER NOT NULL,
    "product_code" VARCHAR(100) NOT NULL,
    "product_name" VARCHAR(200) NOT NULL,
    "product_description" TEXT,
    "unit" VARCHAR(20) NOT NULL,
    "is_enabled" BOOLEAN NOT NULL DEFAULT true,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "created_by" INTEGER,
    "updated_by" INTEGER,
    "remarks" TEXT,
    CONSTRAINT "fk_supplier_products_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_supplier_products_supplier" ON "supplier_products" ("supplier_id");
COMMENT ON TABLE "supplier_products" IS '供应商产品表';

-- ============================================
-- 5. 供应商产品颜色表
-- ============================================
CREATE TABLE IF NOT EXISTS "supplier_product_colors" (
    "id" SERIAL PRIMARY KEY,
    "supplier_product_id" INTEGER NOT NULL,
    "color_no" VARCHAR(50) NOT NULL,
    "color_name" VARCHAR(100) NOT NULL,
    "pantone_code" VARCHAR(50),
    "extra_cost" DECIMAL(10, 2) NOT NULL DEFAULT 0,
    "is_enabled" BOOLEAN NOT NULL DEFAULT true,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "remarks" TEXT,
    CONSTRAINT "fk_supplier_product_colors_product" FOREIGN KEY ("supplier_product_id") REFERENCES "supplier_products" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_supplier_product_colors_product" ON "supplier_product_colors" ("supplier_product_id");
COMMENT ON TABLE "supplier_product_colors" IS '供应商产品颜色表';

-- ============================================
-- 6. 产品颜色表
-- ============================================
CREATE TABLE IF NOT EXISTS "product_colors" (
    "id" SERIAL PRIMARY KEY,
    "product_id" INTEGER NOT NULL,
    "color_no" VARCHAR(50) NOT NULL,
    "color_name" VARCHAR(100) NOT NULL,
    "pantone_code" VARCHAR(50),
    "color_type" VARCHAR(20) NOT NULL DEFAULT 'STANDARD',
    "dye_formula" TEXT,
    "extra_cost" DECIMAL(10, 2) NOT NULL DEFAULT 0,
    "is_active" BOOLEAN NOT NULL DEFAULT true,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_product_colors_product" FOREIGN KEY ("product_id") REFERENCES "products" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_product_colors_product" ON "product_colors" ("product_id");
COMMENT ON TABLE "product_colors" IS '产品颜色表';

-- ============================================
-- 7. 产品编码映射表
-- ============================================
CREATE TABLE IF NOT EXISTS "product_code_mapping" (
    "id" SERIAL PRIMARY KEY,
    "product_id" INTEGER NOT NULL,
    "customer_code" VARCHAR(50) NOT NULL,
    "customer_name" VARCHAR(200),
    "customer_product_code" VARCHAR(100),
    "customer_product_name" VARCHAR(200),
    "is_active" BOOLEAN NOT NULL DEFAULT true,
    "remarks" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_product_code_mapping_product" FOREIGN KEY ("product_id") REFERENCES "products" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_product_code_mapping_product" ON "product_code_mapping" ("product_id");
COMMENT ON TABLE "product_code_mapping" IS '产品编码映射表';

-- ============================================
-- 8. 色号映射表
-- ============================================
CREATE TABLE IF NOT EXISTS "color_code_mapping" (
    "id" SERIAL PRIMARY KEY,
    "product_color_id" INTEGER NOT NULL,
    "customer_code" VARCHAR(50) NOT NULL,
    "customer_name" VARCHAR(200),
    "customer_color_code" VARCHAR(100),
    "customer_color_name" VARCHAR(200),
    "is_active" BOOLEAN NOT NULL DEFAULT true,
    "remarks" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_color_code_mapping_product_color" FOREIGN KEY ("product_color_id") REFERENCES "product_colors" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_color_code_mapping_product_color" ON "color_code_mapping" ("product_color_id");
COMMENT ON TABLE "color_code_mapping" IS '色号映射表';

-- ============================================
-- 9. 匹号映射表
-- ============================================
CREATE TABLE IF NOT EXISTS "piece_mapping" (
    "id" SERIAL PRIMARY KEY,
    "batch_no" VARCHAR(50) NOT NULL,
    "product_id" INTEGER NOT NULL,
    "piece_no" VARCHAR(50) NOT NULL,
    "length" DECIMAL(12, 2) NOT NULL,
    "weight" DECIMAL(12, 2),
    "status" VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
    "remarks" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_piece_mapping_product" FOREIGN KEY ("product_id") REFERENCES "products" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_piece_mapping_product" ON "piece_mapping" ("product_id");
CREATE INDEX IF NOT EXISTS "idx_piece_mapping_batch" ON "piece_mapping" ("batch_no");
COMMENT ON TABLE "piece_mapping" IS '匹号映射表';

-- ============================================
-- 10. 产品供应商映射表（枢纽表）
-- ============================================
CREATE TABLE IF NOT EXISTS "product_supplier_mappings" (
    "id" SERIAL PRIMARY KEY,
    "product_id" INTEGER NOT NULL,
    "product_color_id" INTEGER,
    "supplier_id" INTEGER NOT NULL,
    "supplier_product_id" INTEGER NOT NULL,
    "supplier_product_color_id" INTEGER,
    "is_primary" BOOLEAN NOT NULL DEFAULT false,
    "priority" INTEGER NOT NULL DEFAULT 1,
    "supplier_price" DECIMAL(12, 2),
    "min_order_quantity" DECIMAL(12, 2),
    "lead_time" INTEGER,
    "is_enabled" BOOLEAN NOT NULL DEFAULT true,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "created_by" INTEGER,
    "updated_by" INTEGER,
    "remarks" TEXT,
    CONSTRAINT "fk_product_supplier_mappings_product" FOREIGN KEY ("product_id") REFERENCES "products" ("id"),
    CONSTRAINT "fk_product_supplier_mappings_product_color" FOREIGN KEY ("product_color_id") REFERENCES "product_colors" ("id"),
    CONSTRAINT "fk_product_supplier_mappings_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id"),
    CONSTRAINT "fk_product_supplier_mappings_supplier_product" FOREIGN KEY ("supplier_product_id") REFERENCES "supplier_products" ("id"),
    CONSTRAINT "fk_product_supplier_mappings_supplier_product_color" FOREIGN KEY ("supplier_product_color_id") REFERENCES "supplier_product_colors" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_product_supplier_mappings_product" ON "product_supplier_mappings" ("product_id");
CREATE INDEX IF NOT EXISTS "idx_product_supplier_mappings_supplier" ON "product_supplier_mappings" ("supplier_id");
CREATE INDEX IF NOT EXISTS "idx_product_supplier_mappings_supplier_product" ON "product_supplier_mappings" ("supplier_product_id");
COMMENT ON TABLE "product_supplier_mappings" IS '产品供应商映射表 - 核心枢纽表';
