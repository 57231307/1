-- v11 批次 145 P1-8：回滚 budget_items 扩展字段

ALTER TABLE "budget_items" DROP COLUMN IF EXISTS "budget_year";
ALTER TABLE "budget_items" DROP COLUMN IF EXISTS "planned_amount";
ALTER TABLE "budget_items" DROP COLUMN IF EXISTS "remark";
