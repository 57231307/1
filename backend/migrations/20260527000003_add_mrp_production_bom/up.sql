-- 批次3：MRP/生产计划 + BOM
-- 创建时间: 2026-05-27
-- 描述: 创建生产管理相关表

-- ============================================
-- 1. BOM物料清单表
-- ============================================
CREATE TABLE IF NOT EXISTS "boms" (
    "id" SERIAL PRIMARY KEY,
    "product_id" INTEGER NOT NULL,
    "version" INTEGER NOT NULL DEFAULT 1,
    "is_default" BOOLEAN NOT NULL DEFAULT false,
    "status" VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
    "remarks" TEXT,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_boms_product" ON "boms" ("product_id");
CREATE INDEX IF NOT EXISTS "idx_boms_status" ON "boms" ("status");
COMMENT ON TABLE "boms" IS 'BOM物料清单表 - 存储产品物料清单';

-- ============================================
-- 2. BOM明细表
-- ============================================
CREATE TABLE IF NOT EXISTS "bom_items" (
    "id" SERIAL PRIMARY KEY,
    "bom_id" INTEGER NOT NULL,
    "material_id" INTEGER NOT NULL,
    "quantity" DECIMAL(12, 4) NOT NULL,
    "unit" VARCHAR(20),
    "scrap_rate" DECIMAL(5, 4) DEFAULT 0,
    "sort_order" INTEGER DEFAULT 0,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_bom_items_bom" FOREIGN KEY ("bom_id") REFERENCES "boms" ("id")
);

CREATE INDEX IF NOT EXISTS "idx_bom_items_bom" ON "bom_items" ("bom_id");
CREATE INDEX IF NOT EXISTS "idx_bom_items_material" ON "bom_items" ("material_id");
COMMENT ON TABLE "bom_items" IS 'BOM明细表 - 存储物料清单明细';

-- ============================================
-- 3. 工作中心表
-- ============================================
CREATE TABLE IF NOT EXISTS "work_centers" (
    "id" SERIAL PRIMARY KEY,
    "code" VARCHAR(50) NOT NULL UNIQUE,
    "name" VARCHAR(100) NOT NULL,
    "work_center_type" VARCHAR(50),
    "daily_capacity" DECIMAL(12, 2),
    "capacity_unit" VARCHAR(20),
    "status" VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
    "remarks" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "work_centers" IS '工作中心表 - 存储生产工作中心信息';

-- ============================================
-- 4. 生产订单表
-- ============================================
CREATE TABLE IF NOT EXISTS "production_orders" (
    "id" SERIAL PRIMARY KEY,
    "order_no" VARCHAR(50) NOT NULL UNIQUE,
    "sales_order_id" INTEGER,
    "product_id" INTEGER NOT NULL,
    "planned_quantity" DECIMAL(12, 2) NOT NULL,
    "actual_quantity" DECIMAL(12, 2),
    "planned_start_date" DATE,
    "planned_end_date" DATE,
    "actual_start_date" DATE,
    "actual_end_date" DATE,
    "status" VARCHAR(20) NOT NULL DEFAULT 'DRAFT',
    "priority" INTEGER NOT NULL DEFAULT 5,
    "work_center_id" INTEGER,
    "remarks" TEXT,
    "created_by" INTEGER NOT NULL,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_production_orders_sales_order" FOREIGN KEY ("sales_order_id") REFERENCES "sales_orders" ("id") ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS "idx_production_orders_sales_order" ON "production_orders" ("sales_order_id");
CREATE INDEX IF NOT EXISTS "idx_production_orders_product" ON "production_orders" ("product_id");
CREATE INDEX IF NOT EXISTS "idx_production_orders_status" ON "production_orders" ("status");
CREATE INDEX IF NOT EXISTS "idx_production_orders_work_center" ON "production_orders" ("work_center_id");
COMMENT ON TABLE "production_orders" IS '生产订单表 - 存储生产工单';

-- ============================================
-- 5. MRP计算结果表
-- ============================================
CREATE TABLE IF NOT EXISTS "mrp_results" (
    "id" SERIAL PRIMARY KEY,
    "calculation_no" VARCHAR(50) NOT NULL UNIQUE,
    "product_id" INTEGER NOT NULL,
    "required_quantity" DECIMAL(12, 2) NOT NULL,
    "required_date" DATE,
    "source_type" VARCHAR(20) NOT NULL,
    "source_id" INTEGER,
    "planned_order_quantity" DECIMAL(12, 2),
    "planned_order_date" DATE,
    "status" VARCHAR(20) NOT NULL DEFAULT 'PLANNED',
    "remarks" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_mrp_results_product" ON "mrp_results" ("product_id");
CREATE INDEX IF NOT EXISTS "idx_mrp_results_status" ON "mrp_results" ("status");
CREATE INDEX IF NOT EXISTS "idx_mrp_results_date" ON "mrp_results" ("required_date");
COMMENT ON TABLE "mrp_results" IS 'MRP计算结果表 - 存储物料需求计划计算结果';

-- ============================================
-- 6. 排程结果表
-- ============================================
CREATE TABLE IF NOT EXISTS "scheduling_result" (
    "id" SERIAL PRIMARY KEY,
    "batch_no" VARCHAR(50) NOT NULL,
    "strategy" VARCHAR(50) NOT NULL,
    "status" VARCHAR(20) NOT NULL DEFAULT 'DRAFT',
    "total_orders" INTEGER NOT NULL DEFAULT 0,
    "scheduled_orders" INTEGER NOT NULL DEFAULT 0,
    "unscheduled_orders" INTEGER NOT NULL DEFAULT 0,
    "conflict_count" INTEGER NOT NULL DEFAULT 0,
    "schedule_start_date" DATE NOT NULL,
    "schedule_end_date" DATE NOT NULL,
    "schedule_details" JSONB,
    "gantt_data" JSONB,
    "conflicts" JSONB,
    "created_by" INTEGER NOT NULL,
    "created_by_name" VARCHAR(100),
    "remarks" TEXT,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_scheduling_result_batch" ON "scheduling_result" ("batch_no");
CREATE INDEX IF NOT EXISTS "idx_scheduling_result_status" ON "scheduling_result" ("status");
COMMENT ON TABLE "scheduling_result" IS '排程结果表 - 存储生产排程结果';
