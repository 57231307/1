-- 回滚 CRM 和坯布表

ALTER TABLE "greige_fabric" DROP CONSTRAINT IF EXISTS "fk_greige_fabric_warehouse";
ALTER TABLE "greige_fabric" DROP CONSTRAINT IF EXISTS "fk_greige_fabric_supplier";

DROP TABLE IF EXISTS "greige_fabric" CASCADE;
