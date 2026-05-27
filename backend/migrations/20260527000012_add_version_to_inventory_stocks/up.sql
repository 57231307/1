-- 添加乐观锁版本号到库存表
-- 用于并发控制，防止库存超卖

ALTER TABLE "inventory_stocks" ADD COLUMN "version" INTEGER NOT NULL DEFAULT 0;

COMMENT ON COLUMN "inventory_stocks"."version" IS '乐观锁版本号，用于并发控制';
