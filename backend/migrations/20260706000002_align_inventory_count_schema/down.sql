-- v11 批次 143 P1-1：回滚 inventory_count schema 对齐
-- 注意：旧字段数据迁移不可逆（system_quantity/actual_quantity 等已删除）

-- 删除新增索引
DROP INDEX IF EXISTS "idx_inventory_count_items_warehouse";
DROP INDEX IF EXISTS "idx_inventory_count_items_stock";
DROP INDEX IF EXISTS "idx_inventory_count_items_count";
DROP INDEX IF EXISTS "idx_inventory_counts_count_date";
DROP INDEX IF EXISTS "idx_inventory_counts_status";
DROP INDEX IF EXISTS "idx_inventory_counts_warehouse";

-- 删除新增外键约束
ALTER TABLE "inventory_count_items" DROP CONSTRAINT IF EXISTS "fk_inventory_count_items_warehouse";
ALTER TABLE "inventory_count_items" DROP CONSTRAINT IF EXISTS "fk_inventory_count_items_stock";

-- 删除 inventory_count_items 新增字段
ALTER TABLE "inventory_count_items" DROP COLUMN IF EXISTS "notes";
ALTER TABLE "inventory_count_items" DROP COLUMN IF EXISTS "total_cost";
ALTER TABLE "inventory_count_items" DROP COLUMN IF EXISTS "quantity_difference";
ALTER TABLE "inventory_count_items" DROP COLUMN IF EXISTS "quantity_actual";
ALTER TABLE "inventory_count_items" DROP COLUMN IF EXISTS "quantity_before";
ALTER TABLE "inventory_count_items" DROP COLUMN IF EXISTS "warehouse_id";
ALTER TABLE "inventory_count_items" DROP COLUMN IF EXISTS "stock_id";
ALTER TABLE "inventory_count_items" DROP COLUMN IF EXISTS "updated_at";

-- 恢复 inventory_count_items 旧字段
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "batch_no" VARCHAR(50);
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "system_quantity" INTEGER DEFAULT 0;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "actual_quantity" INTEGER DEFAULT 0;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "discrepancy_quantity" INTEGER DEFAULT 0;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "discrepancy_amount" DECIMAL(14, 2) DEFAULT 0;
ALTER TABLE "inventory_count_items" ADD COLUMN IF NOT EXISTS "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE;

-- 删除 inventory_counts 新增字段
ALTER TABLE "inventory_counts" DROP COLUMN IF EXISTS "completed_at";
ALTER TABLE "inventory_counts" DROP COLUMN IF EXISTS "approved_at";
ALTER TABLE "inventory_counts" DROP COLUMN IF EXISTS "variance_items";
ALTER TABLE "inventory_counts" DROP COLUMN IF EXISTS "counted_items";
ALTER TABLE "inventory_counts" DROP COLUMN IF EXISTS "total_items";

-- 恢复 inventory_counts 旧字段
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "total_discrepancy" DECIMAL(14, 2) DEFAULT 0;
ALTER TABLE "inventory_counts" ADD COLUMN IF NOT EXISTS "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE;

-- 恢复 count_date 为 DATE 类型
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_counts' AND column_name = 'count_date' AND data_type = 'timestamp with time zone') THEN
        ALTER TABLE "inventory_counts" ALTER COLUMN "count_date" TYPE DATE USING "count_date"::date;
    END IF;
END $$;

-- 恢复 status 默认值为 'draft'
ALTER TABLE "inventory_counts" ALTER COLUMN "status" SET DEFAULT 'draft';
