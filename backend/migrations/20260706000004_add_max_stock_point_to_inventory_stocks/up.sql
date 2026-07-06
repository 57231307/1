-- v11 批次 144 P1-4：为 inventory_stocks 添加 max_stock_point 字段
--
-- 背景：
--   stock_alert.rs 中 AlertType::OverStock 此前标注为 dead_code，
--   原因是 inventory_stocks 表无 max_stock_point（库存上限）字段，
--   compute_alert_type 无法判定"高于上限"告警。
--
-- 修复：
--   1. 添加 max_stock_point DECIMAL(12,2) NOT NULL DEFAULT 0
--      （与 reorder_point 保持一致的类型与默认值语义，0 表示未设置上限）
--   2. 添加注释
--
-- 关联：
--   - P1-5 移除 stock_alert.rs 中 OverStock / SlowMoving 的 dead_code 标注
--   - compute_alert_type 扩展 OverStock / SlowMoving 告警判定
--   - last_movement_date 字段已在 20260613000001_add_missing_columns 中添加，无需重复

ALTER TABLE "inventory_stocks" ADD COLUMN "max_stock_point" DECIMAL(12, 2) NOT NULL DEFAULT 0;

COMMENT ON COLUMN "inventory_stocks"."max_stock_point" IS '库存上限（高于此值触发 OverStock 告警，0 表示未设置）';
