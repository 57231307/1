-- v11 批次 144 P1-4：回滚 inventory_stocks.max_stock_point 字段

ALTER TABLE "inventory_stocks" DROP COLUMN IF EXISTS "max_stock_point";
