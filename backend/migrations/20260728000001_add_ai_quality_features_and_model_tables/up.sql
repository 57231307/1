-- V15 P1 批次14：AI 模块 schema 扩展
-- 涵盖缺陷：2.1（准确率监控 actual_*）/ 2.2（质量预测特征列）/ 2.4+8.3（对账表）/
--           3.1+10.2（模型版本管理 + 变更审计）/ 3.4（模型评估指标）/
--           8.2（工艺优化→生产执行关联）/ 10.1（AI 决策审计日志专用表）/
--           5.1+9.1+9.5（source 增加 degraded 标识降级结果）

-- ============================================
-- 0. 放宽 source CHECK 约束允许 degraded（P1 5.1+9.1+9.5）
-- ============================================
ALTER TABLE "ai_process_optimizations" DROP CONSTRAINT IF EXISTS "chk_ai_proc_source";
ALTER TABLE "ai_process_optimizations"
    ADD CONSTRAINT "chk_ai_proc_source"
    CHECK ("source" IN ('knn', 'fallback', 'degraded'));

ALTER TABLE "ai_quality_predictions" DROP CONSTRAINT IF EXISTS "chk_ai_qual_source";
ALTER TABLE "ai_quality_predictions"
    ADD CONSTRAINT "chk_ai_qual_source"
    CHECK ("source" IN ('history', 'fallback', 'degraded'));

-- ============================================
-- 1. quality_inspection_records 增加面料行业特征列（P1 2.2）
-- ============================================
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "dye_type" VARCHAR(50);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "auxiliary_type" VARCHAR(50);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "temperature" NUMERIC(10,2);
ALTER TABLE "quality_inspection_records" ADD COLUMN IF NOT EXISTS "fabric_source" VARCHAR(100);

COMMENT ON COLUMN "quality_inspection_records"."dye_type" IS '染料类型（活性/分散/酸性/还原等，P1 2.2 质量预测特征）';
COMMENT ON COLUMN "quality_inspection_records"."auxiliary_type" IS '助剂类型（P1 2.2 质量预测特征）';
COMMENT ON COLUMN "quality_inspection_records"."temperature" IS '染色温度°C（P1 2.2 质量预测特征）';
COMMENT ON COLUMN "quality_inspection_records"."fabric_source" IS '胚布来源（P1 2.2 质量预测特征）';

-- ============================================
-- 2. ai_quality_predictions 增加准确率监控字段（P1 2.1）
-- ============================================
ALTER TABLE "ai_quality_predictions" ADD COLUMN IF NOT EXISTS "actual_risk_level" VARCHAR(20);
ALTER TABLE "ai_quality_predictions" ADD COLUMN IF NOT EXISTS "actual_avg_qualification_rate" NUMERIC(10,2);
ALTER TABLE "ai_quality_predictions" ADD COLUMN IF NOT EXISTS "actual_recorded_at" TIMESTAMPTZ;
ALTER TABLE "ai_quality_predictions" ADD COLUMN IF NOT EXISTS "model_version_id" INTEGER;
ALTER TABLE "ai_quality_predictions" ADD COLUMN IF NOT EXISTS "inference_latency_ms" INTEGER;

COMMENT ON COLUMN "ai_quality_predictions"."actual_risk_level" IS '实际风险等级（P1 2.1 准确率监控回填）';
COMMENT ON COLUMN "ai_quality_predictions"."actual_avg_qualification_rate" IS '实际平均合格率（P1 2.1 准确率监控回填）';
COMMENT ON COLUMN "ai_quality_predictions"."actual_recorded_at" IS '实际结果记录时间（P1 2.1）';
COMMENT ON COLUMN "ai_quality_predictions"."model_version_id" IS 'AI 模型版本 ID（P1 3.1 关联 ai_model_versions）';
COMMENT ON COLUMN "ai_quality_predictions"."inference_latency_ms" IS '推理耗时毫秒（P1 10.3）';

-- ============================================
-- 3. ai_process_optimizations 增加模型版本 + 生产执行关联（P1 3.1/8.2）
-- ============================================
ALTER TABLE "ai_process_optimizations" ADD COLUMN IF NOT EXISTS "model_version_id" INTEGER;
ALTER TABLE "ai_process_optimizations" ADD COLUMN IF NOT EXISTS "production_recipe_id" INTEGER;
ALTER TABLE "ai_process_optimizations" ADD COLUMN IF NOT EXISTS "inference_latency_ms" INTEGER;

COMMENT ON COLUMN "ai_process_optimizations"."model_version_id" IS 'AI 模型版本 ID（P1 3.1 关联 ai_model_versions）';
COMMENT ON COLUMN "ai_process_optimizations"."production_recipe_id" IS '关联生产配方 ID（P1 8.2 工艺优化→生产执行）';
COMMENT ON COLUMN "ai_process_optimizations"."inference_latency_ms" IS '推理耗时毫秒（P1 10.3）';

-- ============================================
-- 4. ai_model_versions 模型版本管理表（P1 3.1 + 10.2 模型变更审计）
-- ============================================
CREATE TABLE IF NOT EXISTS "ai_model_versions" (
    "id" SERIAL PRIMARY KEY,
    "model_name" VARCHAR(100) NOT NULL,
    "version" VARCHAR(50) NOT NULL,
    "algorithm" VARCHAR(100) NOT NULL,
    "parameters_json" JSONB,
    "training_date" DATE,
    "training_dataset_size" INTEGER,
    "accuracy_metrics_json" JSONB,
    "status" VARCHAR(20) NOT NULL DEFAULT 'draft',
    "changed_by" INTEGER,
    "change_reason" TEXT,
    "approval_status" VARCHAR(20) NOT NULL DEFAULT 'pending',
    "approved_by" INTEGER,
    "approved_at" TIMESTAMPTZ,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "uk_ai_model_versions_name_ver" UNIQUE ("model_name", "version"),
    CONSTRAINT "chk_ai_model_status" CHECK ("status" IN ('draft', 'active', 'retired', 'archived')),
    CONSTRAINT "chk_ai_model_approval" CHECK ("approval_status" IN ('pending', 'approved', 'rejected'))
);

CREATE INDEX IF NOT EXISTS "idx_ai_model_versions_name" ON "ai_model_versions" ("model_name");
CREATE INDEX IF NOT EXISTS "idx_ai_model_versions_status" ON "ai_model_versions" ("status");
COMMENT ON TABLE "ai_model_versions" IS 'AI 模型版本管理表（P1 3.1 + 10.2 模型变更审计）';

-- ============================================
-- 5. ai_model_evaluations 模型评估指标表（P1 3.4）
-- ============================================
CREATE TABLE IF NOT EXISTS "ai_model_evaluations" (
    "id" SERIAL PRIMARY KEY,
    "model_version_id" INTEGER NOT NULL,
    "evaluation_date" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "accuracy" NUMERIC(10,4),
    "precision" NUMERIC(10,4),
    "recall" NUMERIC(10,4),
    "f1_score" NUMERIC(10,4),
    "sample_count" INTEGER NOT NULL DEFAULT 0,
    "evaluation_report" TEXT,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_ai_model_eval_version" FOREIGN KEY ("model_version_id") REFERENCES "ai_model_versions" ("id"),
    CONSTRAINT "chk_ai_model_eval_accuracy" CHECK ("accuracy" >= 0.0 AND "accuracy" <= 1.0)
);

CREATE INDEX IF NOT EXISTS "idx_ai_model_evaluations_version" ON "ai_model_evaluations" ("model_version_id");
COMMENT ON TABLE "ai_model_evaluations" IS 'AI 模型评估指标表（P1 3.4）';

-- ============================================
-- 6. ai_decision_logs AI 决策审计日志专用表（P1 10.1）
-- ============================================
CREATE TABLE IF NOT EXISTS "ai_decision_logs" (
    "id" SERIAL PRIMARY KEY,
    "decision_type" VARCHAR(50) NOT NULL,
    "model_version_id" INTEGER,
    "input_json" JSONB,
    "output_json" JSONB,
    "user_id" INTEGER,
    "ip_address" VARCHAR(50),
    "latency_ms" INTEGER,
    "confidence" NUMERIC(10,4),
    "source" VARCHAR(20),
    "degraded" BOOLEAN NOT NULL DEFAULT false,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "fk_ai_decision_logs_version" FOREIGN KEY ("model_version_id") REFERENCES "ai_model_versions" ("id"),
    CONSTRAINT "chk_ai_decision_type" CHECK ("decision_type" IN ('process_optimization', 'quality_prediction', 'sales_forecast', 'inventory_optimization', 'anomaly_detection', 'recommendation'))
);

CREATE INDEX IF NOT EXISTS "idx_ai_decision_logs_type" ON "ai_decision_logs" ("decision_type");
CREATE INDEX IF NOT EXISTS "idx_ai_decision_logs_user" ON "ai_decision_logs" ("user_id");
CREATE INDEX IF NOT EXISTS "idx_ai_decision_logs_created" ON "ai_decision_logs" ("created_at");
COMMENT ON TABLE "ai_decision_logs" IS 'AI 决策审计日志专用表（P1 10.1）';

-- ============================================
-- 7. ai_quality_accuracy_reports 质量预测准确率报告表（P1 2.4 + 8.3）
-- ============================================
CREATE TABLE IF NOT EXISTS "ai_quality_accuracy_reports" (
    "id" SERIAL PRIMARY KEY,
    "report_period" VARCHAR(20) NOT NULL,
    "total_predictions" INTEGER NOT NULL DEFAULT 0,
    "correct_predictions" INTEGER NOT NULL DEFAULT 0,
    "accuracy_rate" NUMERIC(10,4),
    "precision_score" NUMERIC(10,4),
    "recall_score" NUMERIC(10,4),
    "f1_score" NUMERIC(10,4),
    "mismatch_cases_json" JSONB,
    "generated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "uk_ai_quality_acc_period" UNIQUE ("report_period")
);

CREATE INDEX IF NOT EXISTS "idx_ai_quality_acc_period" ON "ai_quality_accuracy_reports" ("report_period");
COMMENT ON TABLE "ai_quality_accuracy_reports" IS '质量预测准确率报告表（P1 2.4 + 8.3 对账）';
