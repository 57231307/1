-- 回滚：移除库存表的乐观锁版本号

ALTER TABLE "inventory_stocks" DROP COLUMN "version";
