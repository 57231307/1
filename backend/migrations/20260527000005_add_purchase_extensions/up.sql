-- 批次5：采购管理扩展
-- 创建时间: 2026-05-27
-- 描述: 创建采购管理扩展表

-- ============================================
-- 1. 采购合同表
-- ============================================
CREATE TABLE IF NOT EXISTS "purchase_contracts" (
    "id" SERIAL PRIMARY KEY,
    "contract_no" VARCHAR(50) NOT NULL UNIQUE,
    "contract_name" VARCHAR(200) NOT NULL,
    "contract_type" VARCHAR(50),
    "supplier_id" INTEGER NOT NULL,
    "supplier_name" VARCHAR(200),
    "total_amount" DECIMAL(15, 2),
    "signed_date" DATE,
    "effective_date" DATE,
    "expiry_date" DATE,
    "payment_terms" TEXT,
    "payment_method" VARCHAR(50),
    "delivery_date" DATE,
    "delivery_location" VARCHAR(200),
    "status" VARCHAR(20) NOT NULL DEFAULT 'DRAFT',
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_purchase_contracts_supplier" ON "purchase_contracts" ("supplier_id");
CREATE INDEX IF NOT EXISTS "idx_purchase_contracts_status" ON "purchase_contracts" ("status");
COMMENT ON TABLE "purchase_contracts" IS '采购合同表';

-- ============================================
-- 2. 采购合同执行表
-- ============================================
CREATE TABLE IF NOT EXISTS "purchase_contract_executions" (
    "id" SERIAL PRIMARY KEY,
    "execution_no" VARCHAR(50) NOT NULL UNIQUE,
    "contract_id" INTEGER NOT NULL,
    "execution_date" DATE NOT NULL,
    "execution_type" VARCHAR(20) NOT NULL,
    "quantity" DECIMAL(12, 2) NOT NULL,
    "amount" DECIMAL(15, 2) NOT NULL,
    "status" VARCHAR(20) NOT NULL DEFAULT 'DRAFT',
    "remarks" TEXT,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_purchase_contract_executions_contract" FOREIGN KEY ("contract_id") REFERENCES "purchase_contracts" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_purchase_contract_executions_contract" ON "purchase_contract_executions" ("contract_id");
COMMENT ON TABLE "purchase_contract_executions" IS '采购合同执行表';

-- ============================================
-- 3. 采购入库单表
-- ============================================
CREATE TABLE IF NOT EXISTS "purchase_receipt" (
    "id" SERIAL PRIMARY KEY,
    "receipt_no" VARCHAR(50) NOT NULL UNIQUE,
    "order_id" INTEGER,
    "supplier_id" INTEGER NOT NULL,
    "receipt_date" DATE NOT NULL,
    "warehouse_id" INTEGER NOT NULL,
    "department_id" INTEGER,
    "receiver_id" INTEGER,
    "inspector_id" INTEGER,
    "inspection_status" VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    "receipt_status" VARCHAR(20) NOT NULL DEFAULT 'DRAFT',
    "total_quantity" DECIMAL(18, 4) NOT NULL DEFAULT 0,
    "total_quantity_alt" DECIMAL(18, 4) NOT NULL DEFAULT 0,
    "total_amount" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "notes" TEXT,
    "attachment_urls" TEXT,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_by" INTEGER,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "confirmed_at" TIMESTAMP,
    "confirmed_by" INTEGER,
    CONSTRAINT "fk_purchase_receipt_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id"),
    CONSTRAINT "fk_purchase_receipt_warehouse" FOREIGN KEY ("warehouse_id") REFERENCES "warehouses" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_purchase_receipt_supplier" ON "purchase_receipt" ("supplier_id");
CREATE INDEX IF NOT EXISTS "idx_purchase_receipt_order" ON "purchase_receipt" ("order_id");
CREATE INDEX IF NOT EXISTS "idx_purchase_receipt_date" ON "purchase_receipt" ("receipt_date");
COMMENT ON TABLE "purchase_receipt" IS '采购入库单表';

-- ============================================
-- 4. 采购入库明细表
-- ============================================
CREATE TABLE IF NOT EXISTS "purchase_receipt_item" (
    "id" SERIAL PRIMARY KEY,
    "receipt_id" INTEGER NOT NULL,
    "order_item_id" INTEGER,
    "line_no" INTEGER NOT NULL,
    "product_id" INTEGER NOT NULL,
    "material_code" VARCHAR(100) NOT NULL,
    "material_name" VARCHAR(200) NOT NULL,
    "batch_no" VARCHAR(50),
    "color_code" VARCHAR(50),
    "lot_no" VARCHAR(50),
    "grade" VARCHAR(20),
    "gram_weight" DECIMAL(8, 2),
    "width" DECIMAL(8, 2),
    "quantity" DECIMAL(18, 4) NOT NULL,
    "quantity_alt" DECIMAL(18, 4),
    "unit_master" VARCHAR(20) NOT NULL,
    "unit_alt" VARCHAR(20),
    "unit_price" DECIMAL(18, 6),
    "amount" DECIMAL(18, 2),
    "location_code" VARCHAR(50),
    "package_no" VARCHAR(50),
    "production_date" DATE,
    "shelf_life" INTEGER,
    "notes" TEXT,
    "created_at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    "internal_dye_lot_id" INTEGER,
    "internal_dye_lot_no" VARCHAR(50),
    "internal_piece_ids" TEXT,
    "internal_piece_nos" TEXT,
    "supplier_dye_lot_no" VARCHAR(50),
    "supplier_piece_nos" TEXT,
    "batch_conversion_log_id" INTEGER,
    CONSTRAINT "fk_purchase_receipt_item_receipt" FOREIGN KEY ("receipt_id") REFERENCES "purchase_receipt" ("id"),
    CONSTRAINT "fk_purchase_receipt_item_product" FOREIGN KEY ("product_id") REFERENCES "products" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_purchase_receipt_item_receipt" ON "purchase_receipt_item" ("receipt_id");
CREATE INDEX IF NOT EXISTS "idx_purchase_receipt_item_product" ON "purchase_receipt_item" ("product_id");
COMMENT ON TABLE "purchase_receipt_item" IS '采购入库明细表';

-- ============================================
-- 5. 采购质检表
-- ============================================
CREATE TABLE IF NOT EXISTS "purchase_inspection" (
    "id" SERIAL PRIMARY KEY,
    "inspection_no" VARCHAR(50) NOT NULL,
    "receipt_id" INTEGER,
    "order_id" INTEGER,
    "supplier_id" INTEGER NOT NULL,
    "inspection_date" DATE NOT NULL,
    "inspector_id" INTEGER,
    "inspection_type" VARCHAR(50),
    "sample_size" DECIMAL(12, 2),
    "defect_count" INTEGER,
    "pass_quantity" DECIMAL(12, 2),
    "reject_quantity" DECIMAL(12, 2),
    "inspection_status" VARCHAR(20),
    "inspection_result" VARCHAR(20),
    "quality_score" DECIMAL(5, 2),
    "defect_description" TEXT,
    "attachment_urls" TEXT,
    "notes" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "completed_at" TIMESTAMP,
    "completed_by" INTEGER
);

CREATE INDEX IF NOT EXISTS "idx_purchase_inspection_receipt" ON "purchase_inspection" ("receipt_id");
CREATE INDEX IF NOT EXISTS "idx_purchase_inspection_supplier" ON "purchase_inspection" ("supplier_id");
COMMENT ON TABLE "purchase_inspection" IS '采购质检表';

-- ============================================
-- 6. 采购退货表
-- ============================================
CREATE TABLE IF NOT EXISTS "purchase_return" (
    "id" SERIAL PRIMARY KEY,
    "return_no" VARCHAR(50) NOT NULL,
    "receipt_id" INTEGER,
    "order_id" INTEGER,
    "supplier_id" INTEGER NOT NULL,
    "return_date" DATE NOT NULL,
    "warehouse_id" INTEGER,
    "department_id" INTEGER,
    "reason_type" VARCHAR(50),
    "reason_detail" TEXT,
    "return_status" VARCHAR(20),
    "total_quantity" DECIMAL(18, 4),
    "total_quantity_alt" DECIMAL(18, 4),
    "total_amount" DECIMAL(15, 2),
    "notes" TEXT,
    "created_by" INTEGER,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_by" INTEGER,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "approved_by" INTEGER,
    "approved_at" TIMESTAMP,
    "rejected_reason" TEXT
);

CREATE INDEX IF NOT EXISTS "idx_purchase_return_supplier" ON "purchase_return" ("supplier_id");
CREATE INDEX IF NOT EXISTS "idx_purchase_return_receipt" ON "purchase_return" ("receipt_id");
COMMENT ON TABLE "purchase_return" IS '采购退货表';

-- ============================================
-- 7. 采购退货明细表
-- ============================================
CREATE TABLE IF NOT EXISTS "purchase_return_item" (
    "id" SERIAL PRIMARY KEY,
    "return_id" INTEGER NOT NULL,
    "line_no" INTEGER NOT NULL,
    "product_id" INTEGER NOT NULL,
    "quantity" DECIMAL(18, 4) NOT NULL,
    "quantity_alt" DECIMAL(18, 4) NOT NULL,
    "unit_price" DECIMAL(18, 6) NOT NULL,
    "unit_price_foreign" DECIMAL(18, 6) NOT NULL,
    "discount_percent" DECIMAL(5, 2) NOT NULL,
    "tax_percent" DECIMAL(5, 2) NOT NULL,
    "subtotal" DECIMAL(18, 2) NOT NULL,
    "tax_amount" DECIMAL(18, 2) NOT NULL,
    "discount_amount" DECIMAL(18, 2) NOT NULL,
    "total_amount" DECIMAL(18, 2) NOT NULL,
    "notes" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_purchase_return_item_return" FOREIGN KEY ("return_id") REFERENCES "purchase_return" ("id"),
    CONSTRAINT "fk_purchase_return_item_product" FOREIGN KEY ("product_id") REFERENCES "products" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_purchase_return_item_return" ON "purchase_return_item" ("return_id");
CREATE INDEX IF NOT EXISTS "idx_purchase_return_item_product" ON "purchase_return_item" ("product_id");
COMMENT ON TABLE "purchase_return_item" IS '采购退货明细表';

-- ============================================
-- 8. 采购价格表
-- ============================================
CREATE TABLE IF NOT EXISTS "purchase_prices" (
    "id" SERIAL PRIMARY KEY,
    "product_id" INTEGER NOT NULL,
    "supplier_id" INTEGER NOT NULL,
    "price" DECIMAL(18, 6) NOT NULL,
    "currency" VARCHAR(10) NOT NULL DEFAULT 'CNY',
    "unit" VARCHAR(20) NOT NULL,
    "min_order_qty" DECIMAL(12, 2) NOT NULL,
    "price_type" VARCHAR(20) NOT NULL,
    "effective_date" DATE NOT NULL,
    "expiry_date" DATE,
    "status" VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
    "approved_by" INTEGER,
    "approved_at" TIMESTAMP,
    "created_by" INTEGER,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_purchase_prices_product" ON "purchase_prices" ("product_id");
CREATE INDEX IF NOT EXISTS "idx_purchase_prices_supplier" ON "purchase_prices" ("supplier_id");
COMMENT ON TABLE "purchase_prices" IS '采购价格表';
