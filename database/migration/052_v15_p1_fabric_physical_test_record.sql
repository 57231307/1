-- ============================================================================
-- Migration 052: V15 P1 面料物理指标检测记录表
-- 依据：V15 审计报告 类四 P1（batch-04 维度 7：验布打卷十项指标）
-- 业务背景：面料行业质检不仅看外观疵点（四分制/十分制），还需检验物理指标（十项）
--   原实现仅 fabric_inspection_record 有 fabric_width_inches 和 qualification_rate，
--   缺少纬斜/缩水率/起毛起球/手感/拉伸强度/撕裂强度/克重/色牢度/密度等物理指标字段
-- 修复策略：
--   1. 新建 fabric_physical_test_record 表，每条记录对应一个检测项目
--   2. 通过 inspection_id 关联 fabric_inspection_record
--   3. test_item 枚举由应用层校验（保持字符串灵活性）
--   4. A 级判定需外观合格率 ≥95% 且 十项指标全部 pass（应用层 determine_quality_grade 增强）
-- 关联文件：backend/src/models/fabric_physical_test_record.rs
-- ============================================================================

CREATE TABLE IF NOT EXISTS "fabric_physical_test_record" (
    "id" SERIAL PRIMARY KEY,
    "inspection_id" INTEGER NOT NULL REFERENCES "fabric_inspection_record"("id") ON DELETE CASCADE,
    "test_item" VARCHAR(50) NOT NULL,
    "test_value" DECIMAL(12, 2) NOT NULL,
    "standard_value" DECIMAL(12, 2),
    "test_result" VARCHAR(10) NOT NULL,
    "tested_by" INTEGER,
    "tested_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "remarks" TEXT,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 索引：按验布记录查询物理指标（高频）
CREATE INDEX IF NOT EXISTS "idx_fabric_physical_test_inspection_id"
    ON "fabric_physical_test_record" ("inspection_id");
-- 索引：按检测项目筛选（统计场景）
CREATE INDEX IF NOT EXISTS "idx_fabric_physical_test_test_item"
    ON "fabric_physical_test_record" ("test_item");
-- 索引：按检测结果筛选（不合格指标统计）
CREATE INDEX IF NOT EXISTS "idx_fabric_physical_test_test_result"
    ON "fabric_physical_test_record" ("test_result");

COMMENT ON TABLE "fabric_physical_test_record" IS '面料物理指标检测记录（十项指标：纬斜/缩水率/起毛起球/手感/拉伸强度/撕裂强度/克重/色牢度/门幅/密度）';
COMMENT ON COLUMN "fabric_physical_test_record"."test_item" IS '检测项目：skewness/shrinkage/pilling/handfeel/tensile_strength/tear_strength/weight_gsm/color_fastness/width/density';
COMMENT ON COLUMN "fabric_physical_test_record"."test_result" IS '检测结果：pass(合格) / fail(不合格)';
