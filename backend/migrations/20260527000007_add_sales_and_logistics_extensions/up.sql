-- 批次7：销售管理扩展 + 物流
-- 创建时间: 2026-05-27
-- 描述: 创建销售管理扩展表和物流表

-- ============================================
-- 1. 销售合同表
-- ============================================
CREATE TABLE IF NOT EXISTS "sales_contracts" (
    "id" SERIAL PRIMARY KEY,
    "contract_no" VARCHAR(50) NOT NULL UNIQUE,
    "contract_name" VARCHAR(200) NOT NULL,
    "contract_type" VARCHAR(50),
    "customer_id" INTEGER NOT NULL,
    "customer_name" VARCHAR(200),
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

CREATE INDEX IF NOT EXISTS "idx_sales_contracts_customer" ON "sales_contracts" ("customer_id");
CREATE INDEX IF NOT EXISTS "idx_sales_contracts_status" ON "sales_contracts" ("status");
COMMENT ON TABLE "sales_contracts" IS '销售合同表';

-- ============================================
-- 2. 销售交货表
-- ============================================
CREATE TABLE IF NOT EXISTS "sales_delivery" (
    "id" SERIAL PRIMARY KEY,
    "delivery_no" VARCHAR(50) NOT NULL UNIQUE,
    "order_id" INTEGER NOT NULL,
    "customer_id" INTEGER NOT NULL,
    "delivery_date" DATE NOT NULL,
    "warehouse_id" INTEGER NOT NULL,
    "status" VARCHAR(20) NOT NULL DEFAULT 'DRAFT',
    "total_quantity" DECIMAL(12, 2) NOT NULL,
    "total_amount" DECIMAL(15, 2) NOT NULL,
    "remarks" TEXT,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_sales_delivery_order" FOREIGN KEY ("order_id") REFERENCES "sales_orders" ("id"),
    CONSTRAINT "fk_sales_delivery_customer" FOREIGN KEY ("customer_id") REFERENCES "customers" ("id"),
    CONSTRAINT "fk_sales_delivery_warehouse" FOREIGN KEY ("warehouse_id") REFERENCES "warehouses" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_sales_delivery_order" ON "sales_delivery" ("order_id");
CREATE INDEX IF NOT EXISTS "idx_sales_delivery_customer" ON "sales_delivery" ("customer_id");
CREATE INDEX IF NOT EXISTS "idx_sales_delivery_date" ON "sales_delivery" ("delivery_date");
COMMENT ON TABLE "sales_delivery" IS '销售交货表';

-- ============================================
-- 3. 销售交货明细表
-- ============================================
CREATE TABLE IF NOT EXISTS "sales_delivery_item" (
    "id" SERIAL PRIMARY KEY,
    "delivery_id" INTEGER NOT NULL,
    "product_id" INTEGER NOT NULL,
    "batch_no" VARCHAR(50),
    "color_no" VARCHAR(50),
    "quantity" DECIMAL(12, 2) NOT NULL,
    "unit_price" DECIMAL(18, 6) NOT NULL,
    "amount" DECIMAL(18, 2) NOT NULL,
    "remarks" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_sales_delivery_item_delivery" FOREIGN KEY ("delivery_id") REFERENCES "sales_delivery" ("id"),
    CONSTRAINT "fk_sales_delivery_item_product" FOREIGN KEY ("product_id") REFERENCES "products" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_sales_delivery_item_delivery" ON "sales_delivery_item" ("delivery_id");
CREATE INDEX IF NOT EXISTS "idx_sales_delivery_item_product" ON "sales_delivery_item" ("product_id");
COMMENT ON TABLE "sales_delivery_item" IS '销售交货明细表';

-- ============================================
-- 4. 销售订单变更历史表
-- ============================================
CREATE TABLE IF NOT EXISTS "sales_order_change_history" (
    "id" SERIAL PRIMARY KEY,
    "order_id" INTEGER NOT NULL,
    "change_type" VARCHAR(50) NOT NULL,
    "field_name" VARCHAR(100),
    "old_value" TEXT,
    "new_value" TEXT,
    "changed_by" INTEGER NOT NULL,
    "changed_at" TIMESTAMP NOT NULL,
    "change_reason" TEXT,
    "ip_address" VARCHAR(50),
    "user_agent" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_sales_order_change_history_order" FOREIGN KEY ("order_id") REFERENCES "sales_orders" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_sales_order_change_history_order" ON "sales_order_change_history" ("order_id");
COMMENT ON TABLE "sales_order_change_history" IS '销售订单变更历史表';

-- ============================================
-- 5. 销售退货表
-- ============================================
CREATE TABLE IF NOT EXISTS "sales_return" (
    "id" SERIAL PRIMARY KEY,
    "return_no" VARCHAR(50) NOT NULL UNIQUE,
    "sales_order_id" INTEGER,
    "customer_id" INTEGER NOT NULL,
    "return_date" DATE NOT NULL,
    "warehouse_id" INTEGER NOT NULL,
    "reason" TEXT NOT NULL,
    "status" VARCHAR(20) NOT NULL DEFAULT 'DRAFT',
    "total_amount" DECIMAL(15, 2) NOT NULL,
    "remarks" TEXT,
    "approved_by" INTEGER,
    "approved_at" TIMESTAMP,
    "rejected_reason" TEXT,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_sales_return_order" FOREIGN KEY ("sales_order_id") REFERENCES "sales_orders" ("id"),
    CONSTRAINT "fk_sales_return_customer" FOREIGN KEY ("customer_id") REFERENCES "customers" ("id"),
    CONSTRAINT "fk_sales_return_warehouse" FOREIGN KEY ("warehouse_id") REFERENCES "warehouses" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_sales_return_order" ON "sales_return" ("sales_order_id");
CREATE INDEX IF NOT EXISTS "idx_sales_return_customer" ON "sales_return" ("customer_id");
CREATE INDEX IF NOT EXISTS "idx_sales_return_date" ON "sales_return" ("return_date");
COMMENT ON TABLE "sales_return" IS '销售退货表';

-- ============================================
-- 6. 销售退货明细表
-- ============================================
CREATE TABLE IF NOT EXISTS "sales_return_item" (
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
    CONSTRAINT "fk_sales_return_item_return" FOREIGN KEY ("return_id") REFERENCES "sales_return" ("id"),
    CONSTRAINT "fk_sales_return_item_product" FOREIGN KEY ("product_id") REFERENCES "products" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_sales_return_item_return" ON "sales_return_item" ("return_id");
CREATE INDEX IF NOT EXISTS "idx_sales_return_item_product" ON "sales_return_item" ("product_id");
COMMENT ON TABLE "sales_return_item" IS '销售退货明细表';

-- ============================================
-- 7. 销售价格表
-- ============================================
CREATE TABLE IF NOT EXISTS "sales_prices" (
    "id" SERIAL PRIMARY KEY,
    "product_id" INTEGER NOT NULL,
    "customer_id" INTEGER,
    "customer_type" VARCHAR(50),
    "price" DECIMAL(18, 6) NOT NULL,
    "currency" VARCHAR(10) NOT NULL DEFAULT 'CNY',
    "unit" VARCHAR(20) NOT NULL,
    "min_order_qty" DECIMAL(12, 2) NOT NULL,
    "price_type" VARCHAR(20) NOT NULL,
    "price_level" VARCHAR(20),
    "effective_date" DATE NOT NULL,
    "expiry_date" DATE,
    "status" VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
    "approved_by" INTEGER,
    "approved_at" TIMESTAMP,
    "created_by" INTEGER,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_sales_prices_product" ON "sales_prices" ("product_id");
CREATE INDEX IF NOT EXISTS "idx_sales_prices_customer" ON "sales_prices" ("customer_id");
COMMENT ON TABLE "sales_prices" IS '销售价格表';

-- ============================================
-- 8. 销售统计分析表
-- ============================================
CREATE TABLE IF NOT EXISTS "sales_statistics" (
    "id" SERIAL PRIMARY KEY,
    "statistic_type" VARCHAR(50) NOT NULL,
    "period" VARCHAR(20) NOT NULL,
    "dimension_type" VARCHAR(50) NOT NULL,
    "dimension_id" INTEGER,
    "dimension_name" VARCHAR(200),
    "order_count" INTEGER NOT NULL DEFAULT 0,
    "total_amount" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "total_qty" DECIMAL(12, 2) NOT NULL DEFAULT 0,
    "total_cost" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "gross_profit" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "gross_profit_rate" DECIMAL(5, 2) NOT NULL DEFAULT 0,
    "avg_order_value" DECIMAL(15, 2) NOT NULL DEFAULT 0,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_sales_statistics_type" ON "sales_statistics" ("statistic_type");
CREATE INDEX IF NOT EXISTS "idx_sales_statistics_period" ON "sales_statistics" ("period");
COMMENT ON TABLE "sales_statistics" IS '销售统计分析表';

-- ============================================
-- 9. 物流运单表
-- ============================================
CREATE TABLE IF NOT EXISTS "logistics_waybills" (
    "id" SERIAL PRIMARY KEY,
    "order_id" INTEGER NOT NULL,
    "logistics_company" VARCHAR(100) NOT NULL,
    "tracking_number" VARCHAR(100) NOT NULL,
    "driver_name" VARCHAR(50),
    "driver_phone" VARCHAR(20),
    "freight_fee" DECIMAL(10, 2),
    "status" VARCHAR(20),
    "expected_arrival" TIMESTAMP,
    "actual_arrival" TIMESTAMP,
    "notes" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_logistics_waybills_order" FOREIGN KEY ("order_id") REFERENCES "sales_orders" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_logistics_waybills_order" ON "logistics_waybills" ("order_id");
CREATE INDEX IF NOT EXISTS "idx_logistics_waybills_tracking" ON "logistics_waybills" ("tracking_number");
COMMENT ON TABLE "logistics_waybills" IS '物流运单表';
