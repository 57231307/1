-- 回滚：移除 sales_orders 表的 opportunity_id 字段

-- 删除索引
DROP INDEX IF EXISTS "idx_sales_orders_opportunity_id";

-- 删除外键约束
ALTER TABLE "sales_orders" DROP CONSTRAINT IF EXISTS "fk_sales_orders_opportunity";

-- 删除字段
ALTER TABLE "sales_orders" DROP COLUMN IF EXISTS "opportunity_id";
