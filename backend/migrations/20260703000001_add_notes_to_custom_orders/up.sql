-- custom_orders 备注列迁移（批次 88 PH-1）
-- 创建时间: 2026-07-03
-- 关联修复: 占位符 PH-1 — DTO 有 notes 字段但 service 层 `let _ = v;` 丢弃
--
-- 向 custom_orders 表添加 notes 列（TEXT，可选），存储订单备注。
-- 使用 ADD COLUMN IF NOT EXISTS 防止迁移重入。

ALTER TABLE "custom_orders" ADD COLUMN IF NOT EXISTS "notes" TEXT;
COMMENT ON COLUMN "custom_orders"."notes" IS '订单备注（批次 88 PH-1 占位符实现）';
