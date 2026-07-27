-- ============================================================================
-- Migration 051: V15 P1 batch_trace_log 表扩展字段
-- 依据：V15 审计报告 类四 P1（batch-04 维度 2：缸号/批号全链路追溯）
-- 业务背景：batch_trace_log 表作为批次追溯日志主表，原仅含 batch_no + 3 种 operation_type，
--   缺少 dye_lot_no/color_no/product_id 字段，无法支持按缸号/色号/产品维度追溯；
--   operation_type 仅 CREATE/TRANSFER/ADJUST 3 种，无法覆盖面料行业全链路操作
--   （dyeing/inspection/grade/ship/rework/merge/split 等）。
-- 修复策略：
--   1. 新增 dye_lot_no/color_no/product_id/from_status/to_status 字段
--   2. 创建索引加速按缸号/色号/产品查询
--   3. 应用层（model）同步接入字段
--   4. operation_type 由应用层常量扩展，不改 DB 约束（保持字符串灵活性）
-- 关联文件：backend/src/models/batch_trace_log.rs
-- ============================================================================

-- 1. 新增追溯字段
ALTER TABLE "batch_trace_log"
    ADD COLUMN IF NOT EXISTS "dye_lot_no" VARCHAR(50);
ALTER TABLE "batch_trace_log"
    ADD COLUMN IF NOT EXISTS "color_no" VARCHAR(50);
ALTER TABLE "batch_trace_log"
    ADD COLUMN IF NOT EXISTS "product_id" INTEGER;
ALTER TABLE "batch_trace_log"
    ADD COLUMN IF NOT EXISTS "from_status" VARCHAR(50);
ALTER TABLE "batch_trace_log"
    ADD COLUMN IF NOT EXISTS "to_status" VARCHAR(50);

-- 2. 创建索引：按缸号/色号/产品维度查询追溯日志（高频场景）
CREATE INDEX IF NOT EXISTS "idx_batch_trace_log_dye_lot_no"
    ON "batch_trace_log" ("dye_lot_no");
CREATE INDEX IF NOT EXISTS "idx_batch_trace_log_color_no"
    ON "batch_trace_log" ("color_no");
CREATE INDEX IF NOT EXISTS "idx_batch_trace_log_product_id"
    ON "batch_trace_log" ("product_id");

-- 3. 字段注释
COMMENT ON COLUMN "batch_trace_log"."dye_lot_no" IS '染色批号（dye_lot_no），面料行业四维标识之一，按染色批号追溯';
COMMENT ON COLUMN "batch_trace_log"."color_no" IS '色号（color_no），按色号追溯';
COMMENT ON COLUMN "batch_trace_log"."product_id" IS '产品 ID，按产品追溯';
COMMENT ON COLUMN "batch_trace_log"."from_status" IS '流转前状态（from_status），记录操作前后状态变化';
COMMENT ON COLUMN "batch_trace_log"."to_status" IS '流转后状态（to_status），记录操作前后状态变化';
