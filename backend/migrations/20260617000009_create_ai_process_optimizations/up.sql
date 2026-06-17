-- AI 工艺优化历史表 - P2-4 AI 分析深化
-- 持久化 AI 染色工艺优化推荐结果，支持历史追溯与应用率统计
-- 创建时间: 2026-06-17
-- 关联 spec: doto.md P2-4 任务清单

CREATE TABLE IF NOT EXISTS "ai_process_optimizations" (
    "id" BIGSERIAL PRIMARY KEY,
    "request_id" VARCHAR(64) NOT NULL UNIQUE,
    "color_no" VARCHAR(64) NOT NULL,
    "color_name" VARCHAR(128),
    "fabric_type" VARCHAR(64) NOT NULL,
    "dye_type" VARCHAR(64),
    "recommended_temperature" DECIMAL(5,2) NOT NULL,
    "recommended_time_minutes" INTEGER NOT NULL,
    "recommended_ph_value" DECIMAL(4,2) NOT NULL,
    "recommended_liquor_ratio" DECIMAL(5,2) NOT NULL,
    "similar_cases" INTEGER NOT NULL DEFAULT 0,
    "confidence" DECIMAL(4,3) NOT NULL,
    "source" VARCHAR(16) NOT NULL,
    "reason" TEXT,
    "candidates_json" JSONB,
    "is_applied" BOOLEAN NOT NULL DEFAULT false,
    "applied_at" TIMESTAMPTZ,
    "applied_by" BIGINT REFERENCES "users"("id"),
    "feedback_score" SMALLINT,
    "feedback_remark" TEXT,
    "tenant_id" BIGINT NOT NULL,
    "created_by" BIGINT REFERENCES "users"("id"),
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "chk_ai_proc_source" CHECK ("source" IN ('knn', 'fallback')),
    CONSTRAINT "chk_ai_proc_confidence" CHECK ("confidence" >= 0.0 AND "confidence" <= 1.0),
    CONSTRAINT "chk_ai_proc_feedback" CHECK ("feedback_score" IS NULL OR ("feedback_score" >= 1 AND "feedback_score" <= 5))
);

-- 索引：列表查询（按租户 + 创建时间倒序）
CREATE INDEX IF NOT EXISTS "idx_ai_proc_tenant_created" ON "ai_process_optimizations"("tenant_id", "created_at" DESC);
-- 索引：按色号 + 布类查询历史推荐
CREATE INDEX IF NOT EXISTS "idx_ai_proc_color_fabric" ON "ai_process_optimizations"("tenant_id", "color_no", "fabric_type");
-- 索引：按应用状态统计应用率
CREATE INDEX IF NOT EXISTS "idx_ai_proc_applied" ON "ai_process_optimizations"("tenant_id", "is_applied");
-- 索引：按 k-NN / fallback 来源统计
CREATE INDEX IF NOT EXISTS "idx_ai_proc_source" ON "ai_process_optimizations"("tenant_id", "source");

-- 注释
COMMENT ON TABLE "ai_process_optimizations" IS 'AI 工艺优化历史表（P2-4）：持久化 k-NN 染色工艺推荐 + 应用反馈';
COMMENT ON COLUMN "ai_process_optimizations"."request_id" IS '请求唯一 ID（UUID），用于幂等防重';
COMMENT ON COLUMN "ai_process_optimizations"."similar_cases" IS '命中相似历史配方数（k-NN 路径）';
COMMENT ON COLUMN "ai_process_optimizations"."confidence" IS '推荐置信度（0.0-1.0）';
COMMENT ON COLUMN "ai_process_optimizations"."source" IS '推荐来源：knn（k-NN 加权）/ fallback（典型参数表）';
COMMENT ON COLUMN "ai_process_optimizations"."candidates_json" IS '候选案例 JSON（最多 10 条）';
COMMENT ON COLUMN "ai_process_optimizations"."is_applied" IS '是否已被工艺员采纳并下发生产';
COMMENT ON COLUMN "ai_process_optimizations"."feedback_score" IS '采纳后质量反馈（1-5 星，null=未反馈）';
