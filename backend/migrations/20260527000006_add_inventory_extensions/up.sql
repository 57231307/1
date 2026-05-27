-- 批次6：库存管理扩展
-- 创建时间: 2026-05-27
-- 描述: 创建库存管理扩展表

-- ============================================
-- 1. 库存流水表
-- ============================================
CREATE TABLE IF NOT EXISTS "inventory_transactions" (
    "id" SERIAL PRIMARY KEY,
    "transaction_type" VARCHAR(50) NOT NULL,
    "product_id" INTEGER NOT NULL,
    "warehouse_id" INTEGER NOT NULL,
    "batch_no" VARCHAR(50) NOT NULL,
    "color_no" VARCHAR(50) NOT NULL,
    "dye_lot_no" VARCHAR(50),
    "grade" VARCHAR(20) NOT NULL,
    "quantity_meters" DECIMAL(12, 2) NOT NULL,
    "quantity_kg" DECIMAL(12, 2) NOT NULL,
    "source_bill_type" VARCHAR(50),
    "source_bill_no" VARCHAR(50),
    "source_bill_id" INTEGER,
    "quantity_before_meters" DECIMAL(12, 2),
    "quantity_before_kg" DECIMAL(12, 2),
    "quantity_after_meters" DECIMAL(12, 2),
    "quantity_after_kg" DECIMAL(12, 2),
    "notes" TEXT,
    "created_by" INTEGER,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_inventory_transactions_product" FOREIGN KEY ("product_id") REFERENCES "products" ("id"),
    CONSTRAINT "fk_inventory_transactions_warehouse" FOREIGN KEY ("warehouse_id") REFERENCES "warehouses" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_inventory_transactions_product" ON "inventory_transactions" ("product_id");
CREATE INDEX IF NOT EXISTS "idx_inventory_transactions_warehouse" ON "inventory_transactions" ("warehouse_id");
CREATE INDEX IF NOT EXISTS "idx_inventory_transactions_batch" ON "inventory_transactions" ("batch_no");
CREATE INDEX IF NOT EXISTS "idx_inventory_transactions_date" ON "inventory_transactions" ("created_at");
COMMENT ON TABLE "inventory_transactions" IS '库存流水表 - 记录库存变动';

-- ============================================
-- 2. 库存预留表
-- ============================================
CREATE TABLE IF NOT EXISTS "inventory_reservations" (
    "id" SERIAL PRIMARY KEY,
    "order_id" INTEGER NOT NULL,
    "product_id" INTEGER NOT NULL,
    "warehouse_id" INTEGER NOT NULL,
    "quantity" DECIMAL(10, 2) NOT NULL,
    "status" VARCHAR(20) NOT NULL DEFAULT 'pending',
    "reserved_at" TIMESTAMP NOT NULL,
    "released_at" TIMESTAMP,
    "notes" TEXT,
    "created_by" INTEGER,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_inventory_reservations_order" FOREIGN KEY ("order_id") REFERENCES "sales_orders" ("id"),
    CONSTRAINT "fk_inventory_reservations_product" FOREIGN KEY ("product_id") REFERENCES "products" ("id"),
    CONSTRAINT "fk_inventory_reservations_warehouse" FOREIGN KEY ("warehouse_id") REFERENCES "warehouses" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_inventory_reservations_order" ON "inventory_reservations" ("order_id");
CREATE INDEX IF NOT EXISTS "idx_inventory_reservations_product" ON "inventory_reservations" ("product_id");
CREATE INDEX IF NOT EXISTS "idx_inventory_reservations_status" ON "inventory_reservations" ("status");
COMMENT ON TABLE "inventory_reservations" IS '库存预留表 - 记录库存预留';

-- ============================================
-- 3. 库存调整主表
-- ============================================
CREATE TABLE IF NOT EXISTS "inventory_adjustments" (
    "id" SERIAL PRIMARY KEY,
    "adjustment_no" VARCHAR(50) NOT NULL UNIQUE,
    "warehouse_id" INTEGER NOT NULL,
    "adjustment_date" TIMESTAMP NOT NULL,
    "adjustment_type" VARCHAR(20) NOT NULL,
    "reason_type" VARCHAR(50) NOT NULL,
    "reason_description" TEXT,
    "total_quantity" DECIMAL(12, 2) NOT NULL,
    "notes" TEXT,
    "created_by" INTEGER,
    "approved_by" INTEGER,
    "approved_at" TIMESTAMP,
    "status" VARCHAR(20) NOT NULL DEFAULT 'pending',
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_inventory_adjustments_warehouse" FOREIGN KEY ("warehouse_id") REFERENCES "warehouses" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_inventory_adjustments_warehouse" ON "inventory_adjustments" ("warehouse_id");
CREATE INDEX IF NOT EXISTS "idx_inventory_adjustments_date" ON "inventory_adjustments" ("adjustment_date");
CREATE INDEX IF NOT EXISTS "idx_inventory_adjustments_status" ON "inventory_adjustments" ("status");
COMMENT ON TABLE "inventory_adjustments" IS '库存调整主表';

-- ============================================
-- 4. 库存调整明细表
-- ============================================
CREATE TABLE IF NOT EXISTS "inventory_adjustment_items" (
    "id" SERIAL PRIMARY KEY,
    "adjustment_id" INTEGER NOT NULL,
    "stock_id" INTEGER NOT NULL,
    "quantity" DECIMAL(10, 2) NOT NULL,
    "quantity_before" DECIMAL(10, 2) NOT NULL,
    "quantity_after" DECIMAL(10, 2) NOT NULL,
    "unit_cost" DECIMAL(12, 2),
    "amount" DECIMAL(12, 2),
    "notes" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_inventory_adjustment_items_adjustment" FOREIGN KEY ("adjustment_id") REFERENCES "inventory_adjustments" ("id"),
    CONSTRAINT "fk_inventory_adjustment_items_stock" FOREIGN KEY ("stock_id") REFERENCES "inventory_stocks" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_inventory_adjustment_items_adjustment" ON "inventory_adjustment_items" ("adjustment_id");
CREATE INDEX IF NOT EXISTS "idx_inventory_adjustment_items_stock" ON "inventory_adjustment_items" ("stock_id");
COMMENT ON TABLE "inventory_adjustment_items" IS '库存调整明细表';

-- ============================================
-- 5. 库存匹数表
-- ============================================
CREATE TABLE IF NOT EXISTS "inventory_piece" (
    "id" SERIAL PRIMARY KEY,
    "batch_no" VARCHAR(50) NOT NULL,
    "parent_piece_id" INTEGER,
    "product_id" INTEGER NOT NULL,
    "warehouse_id" INTEGER NOT NULL,
    "location_id" INTEGER,
    "piece_no" VARCHAR(50) NOT NULL,
    "length" DECIMAL(12, 2) NOT NULL,
    "weight" DECIMAL(12, 2),
    "status" VARCHAR(20) NOT NULL DEFAULT 'AVAILABLE',
    "remarks" TEXT,
    "barcode" VARCHAR(100),
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_inventory_piece_product" FOREIGN KEY ("product_id") REFERENCES "products" ("id"),
    CONSTRAINT "fk_inventory_piece_warehouse" FOREIGN KEY ("warehouse_id") REFERENCES "warehouses" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_inventory_piece_product" ON "inventory_piece" ("product_id");
CREATE INDEX IF NOT EXISTS "idx_inventory_piece_warehouse" ON "inventory_piece" ("warehouse_id");
CREATE INDEX IF NOT EXISTS "idx_inventory_piece_batch" ON "inventory_piece" ("batch_no");
CREATE INDEX IF NOT EXISTS "idx_inventory_piece_status" ON "inventory_piece" ("status");
COMMENT ON TABLE "inventory_piece" IS '库存匹数表 - 记录面料匹号信息';

-- ============================================
-- 6. 库位表
-- ============================================
CREATE TABLE IF NOT EXISTS "warehouse_locations" (
    "id" SERIAL PRIMARY KEY,
    "warehouse_id" INTEGER NOT NULL,
    "location_code" VARCHAR(50) NOT NULL,
    "location_type" VARCHAR(50),
    "max_weight" DECIMAL(12, 2),
    "max_height" DECIMAL(12, 2),
    "is_batch_managed" BOOLEAN DEFAULT false,
    "is_color_managed" BOOLEAN DEFAULT false,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_warehouse_locations_warehouse" FOREIGN KEY ("warehouse_id") REFERENCES "warehouses" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_warehouse_locations_warehouse" ON "warehouse_locations" ("warehouse_id");
CREATE UNIQUE INDEX IF NOT EXISTS "idx_warehouse_locations_code" ON "warehouse_locations" ("warehouse_id", "location_code");
COMMENT ON TABLE "warehouse_locations" IS '库位表 - 存储仓库库位信息';
