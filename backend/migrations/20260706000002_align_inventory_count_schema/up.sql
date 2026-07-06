-- v11 批次 143 P1-1：库存盘点模块 schema 对齐
-- 将 inventory_counts / inventory_count_items 表结构对齐到 SeaORM 模型定义
-- 模型位置：backend/src/models/inventory_count.rs / inventory_count_item.rs

-- ============================================
-- 1. inventory_counts 表：新增模型字段 + 类型对齐
-- ============================================

-- count_date 由 DATE 升级为 TIMESTAMPTZ（与模型 DateTime<Utc> 对齐）
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_counts' AND column_name = 'count_date' AND data_type = 'date') THEN
        ALTER TABLE "inventory_counts" ALTER COLUMN "count_date" TYPE TIMESTAMPTZ USING "count_date" AT TIME ZONE 'UTC';
    END IF;
END $$;

-- status 默认值由 'draft' 调整为 'pending'（与盘点流程入口一致）
ALTER TABLE "inventory_counts" ALTER COLUMN "status" SET DEFAULT 'pending';

-- 新增盘点单字段
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "total_items" INTEGER NOT NULL DEFAULT 0;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "counted_items" INTEGER NOT NULL DEFAULT 0;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "variance_items" INTEGER NOT NULL DEFAULT 0;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "approved_at" TIMESTAMPTZ;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "completed_at" TIMESTAMPTZ;

-- 删除旧字段（已被 total_items/counted_items/variance_items 取代）
-- total_discrepancy 字段语义与 variance_items 重叠，且无业务引用
ALTER TABLE "inventory_counts" DROP COLUMN IF EXISTS "total_discrepancy";
-- is_deleted 字段无业务引用（盘点单通过状态字段管理生命周期）
ALTER TABLE "inventory_counts" DROP COLUMN IF EXISTS "is_deleted";

COMMENT ON COLUMN "inventory_counts"."total_items" IS '盘点单总明细数';
COMMENT ON COLUMN "inventory_counts"."counted_items" IS '已盘点明细数';
COMMENT ON COLUMN "inventory_counts"."variance_items" IS '存在差异的明细数';
COMMENT ON COLUMN "inventory_counts"."approved_at" IS '审批时间';
COMMENT ON COLUMN "inventory_counts"."completed_at" IS '完成时间';

-- ============================================
-- 2. inventory_count_items 表：新增模型字段 + 类型对齐
-- ============================================

-- 新增盘点明细字段（stock_id/warehouse_id 使用 NOT NULL DEFAULT 0 以兼容已有行）
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "stock_id" INTEGER NOT NULL DEFAULT 0;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "warehouse_id" INTEGER NOT NULL DEFAULT 0;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "quantity_before" DECIMAL(10, 2) NOT NULL DEFAULT 0;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "quantity_actual" DECIMAL(10, 2) NOT NULL DEFAULT 0;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "quantity_difference" DECIMAL(10, 2) NOT NULL DEFAULT 0;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "total_cost" DECIMAL(12, 2) NOT NULL DEFAULT 0;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "notes" TEXT;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP;

-- 旧字段数据迁移到新字段（若旧字段存在且有数据）
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_count_items' AND column_name = 'system_quantity') THEN
        UPDATE "inventory_count_items" SET "quantity_before" = "system_quantity" WHERE "quantity_before" = 0 AND "system_quantity" IS NOT NULL;
    END IF;
END $$;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_count_items' AND column_name = 'actual_quantity') THEN
        UPDATE "inventory_count_items" SET "quantity_actual" = "actual_quantity" WHERE "quantity_actual" = 0 AND "actual_quantity" IS NOT NULL;
    END IF;
END $$;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_count_items' AND column_name = 'discrepancy_quantity') THEN
        UPDATE "inventory_count_items" SET "quantity_difference" = "discrepancy_quantity" WHERE "quantity_difference" = 0 AND "discrepancy_quantity" IS NOT NULL;
    END IF;
END $$;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_count_items' AND column_name = 'discrepancy_amount') THEN
        UPDATE "inventory_count_items" SET "total_cost" = "discrepancy_amount" WHERE "total_cost" = 0 AND "discrepancy_amount" IS NOT NULL;
    END IF;
END $$;

-- 删除旧字段（数据已迁移到新字段）
ALTER TABLE "inventory_count_items" DROP COLUMN IF EXISTS "batch_no";
ALTER TABLE "inventory_count_items" DROP COLUMN IF EXISTS "system_quantity";
ALTER TABLE "inventory_count_items" DROP COLUMN IF EXISTS "actual_quantity";
ALTER TABLE "inventory_count_items" DROP COLUMN IF EXISTS "discrepancy_quantity";
ALTER TABLE "inventory_count_items" DROP COLUMN IF EXISTS "discrepancy_amount";
ALTER TABLE "inventory_count_items" DROP COLUMN IF EXISTS "is_deleted";

-- unit_cost 字段类型保持 DECIMAL(12, 2) 不变（与模型一致）
COMMENT ON COLUMN "inventory_count_items"."stock_id" IS '库存 ID（关联 inventory_stocks.id）';
COMMENT ON COLUMN "inventory_count_items"."warehouse_id" IS '仓库 ID';
COMMENT ON COLUMN "inventory_count_items"."quantity_before" IS '盘点前账面数量';
COMMENT ON COLUMN "inventory_count_items"."quantity_actual" IS '实际盘点数量';
COMMENT ON COLUMN "inventory_count_items"."quantity_difference" IS '差异数量（实际 - 账面）';
COMMENT ON COLUMN "inventory_count_items"."total_cost" IS '总成本差异';
COMMENT ON COLUMN "inventory_count_items"."notes" IS '明细备注';

-- 添加外键约束
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_inventory_count_items_stock') THEN
        ALTER TABLE "inventory_count_items" ADD CONSTRAINT "fk_inventory_count_items_stock" FOREIGN KEY ("stock_id") REFERENCES "inventory_stocks" ("id");
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_inventory_count_items_warehouse') THEN
        ALTER TABLE "inventory_count_items" ADD CONSTRAINT "fk_inventory_count_items_warehouse" FOREIGN KEY ("warehouse_id") REFERENCES "warehouses" ("id");
    END IF;
END $$;

-- 添加索引
CREATE INDEX IF NOT EXISTS "idx_inventory_counts_warehouse" ON "inventory_counts" ("warehouse_id");
CREATE INDEX IF NOT EXISTS "idx_inventory_counts_status" ON "inventory_counts" ("status");
CREATE INDEX IF NOT EXISTS "idx_inventory_counts_count_date" ON "inventory_counts" ("count_date");
CREATE INDEX IF NOT EXISTS "idx_inventory_count_items_count" ON "inventory_count_items" ("count_id");
CREATE INDEX IF NOT EXISTS "idx_inventory_count_items_stock" ON "inventory_count_items" ("stock_id");
CREATE INDEX IF NOT EXISTS "idx_inventory_count_items_warehouse" ON "inventory_count_items" ("warehouse_id");
