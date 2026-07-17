-- ============================================================================
-- Migration 048: V15 P0-F01 dye_batch 表新增 dye_lot_no 字段
-- 依据：V15 P0 审计报告 类四 P0-F01（batch-04 P0-04-1/2）
-- 业务背景：面料行业四维标识（product_id + color_no + dye_lot_no + batch_no）
--   dye_batch 主表历史缺失 dye_lot_no 字段，导致：
--   1. 四层级联断裂（与 batch_dye_lot/cost_collection/greige_fabric 等表无法关联）
--   2. 成本归集不完整（dye_batch_cost_bridge_service.rs:152 注释"dye_lot_no 暂为 None"）
--   3. 缸号追溯失效（仅靠 batch_no 无法定位到具体染色批号）
-- 术语说明：
--   - batch_no：缸号（与"染色批次号"是同一概念，仅叫法不同，指一次染色生产的一个缸的批次号）
--   - dye_lot_no：染色批号（面料行业的"lot"概念，指同一产品同一颜色在不同时间或不同染缸染出的批次标识，
--     用于库存/发货/追溯时区分不同染色批次，避免色差混批）
-- 修复策略：
--   1. 新增 dye_batch.dye_lot_no 字段（VARCHAR(50) NOT NULL DEFAULT 'DEFAULT'）
--      历史数据回填默认值 'DEFAULT'，避免 NOT NULL 约束导致迁移失败
--   2. 创建索引 idx_dye_batch_dye_lot_no 加速按染色批号查询
--   3. 应用层（model/handler/bridge service）同步接入字段
-- 关联文件：backend/src/models/dye_batch.rs / handlers/dye_batch_handler.rs /
--          services/dye_batch_cost_bridge_service.rs / 前端 dye_batch_view.vue
-- ============================================================================

-- 1. 新增 dye_lot_no 字段（NOT NULL DEFAULT 'DEFAULT'，兼容历史数据）
ALTER TABLE "dye_batch"
    ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(50) NOT NULL DEFAULT 'DEFAULT';

-- 2. 创建索引：按 dye_lot_no 查询（染色批号追溯/四维关联查询高频使用）
CREATE INDEX IF NOT EXISTS "idx_dye_batch_dye_lot_no" ON "dye_batch" ("dye_lot_no");

-- 3. 字段注释
COMMENT ON COLUMN "dye_batch"."dye_lot_no" IS '染色批号（dye_lot_no，面料行业 lot 概念，与缸号 batch_no 不同：缸号=染色批次号指一次染缸生产的批次，染色批号指同产品同颜色不同时间/染缸的批次标识，用于防色差混批）。历史数据回填 DEFAULT，新数据由创建接口传入。';

-- ============================================================================
-- 验证 SQL（手动验证用，迁移脚本不会执行）
-- ============================================================================
-- SELECT column_name, data_type, is_nullable, column_default
-- FROM information_schema.columns
-- WHERE table_name = 'dye_batch' AND column_name = 'dye_lot_no';
-- 预期：dye_lot_no | character varying(50) | NO | 'DEFAULT'::character varying
