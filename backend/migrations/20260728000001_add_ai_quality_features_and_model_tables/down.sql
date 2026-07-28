-- 回滚：移除 AI 模块 schema 扩展
DROP TABLE IF EXISTS "ai_quality_accuracy_reports";
DROP TABLE IF EXISTS "ai_decision_logs";
DROP TABLE IF EXISTS "ai_model_evaluations";
DROP TABLE IF EXISTS "ai_model_versions";

ALTER TABLE "ai_process_optimizations" DROP COLUMN IF EXISTS "model_version_id";
ALTER TABLE "ai_process_optimizations" DROP COLUMN IF EXISTS "production_recipe_id";
ALTER TABLE "ai_process_optimizations" DROP COLUMN IF EXISTS "inference_latency_ms";

ALTER TABLE "ai_quality_predictions" DROP COLUMN IF EXISTS "actual_risk_level";
ALTER TABLE "ai_quality_predictions" DROP COLUMN IF EXISTS "actual_avg_qualification_rate";
ALTER TABLE "ai_quality_predictions" DROP COLUMN IF EXISTS "actual_recorded_at";
ALTER TABLE "ai_quality_predictions" DROP COLUMN IF EXISTS "model_version_id";
ALTER TABLE "ai_quality_predictions" DROP COLUMN IF EXISTS "inference_latency_ms";

ALTER TABLE "quality_inspection_records" DROP COLUMN IF EXISTS "dye_type";
ALTER TABLE "quality_inspection_records" DROP COLUMN IF EXISTS "auxiliary_type";
ALTER TABLE "quality_inspection_records" DROP COLUMN IF EXISTS "temperature";
ALTER TABLE "quality_inspection_records" DROP COLUMN IF EXISTS "fabric_source";
