-- AI 质量预测历史表 - P2-4 AI 分析深化
-- 持久化 AI 质量预测结果（风险评分 / 趋势 / 问题归因），支持历史回溯与质量看板
-- 创建时间: 2026-06-17
-- 关联 spec: doto.md P2-4 任务清单

CREATE TABLE IF NOT EXISTS "ai_quality_predictions" (
    "id" BIGSERIAL PRIMARY KEY,
    "request_id" VARCHAR(64) NOT NULL UNIQUE,
    "product_id" BIGINT REFERENCES "products"("id"),
    "inspection_type" VARCHAR(32) NOT NULL DEFAULT 'all',
    "window_days" INTEGER NOT NULL DEFAULT 90,
    "total_inspections" BIGINT NOT NULL DEFAULT 0,
    "avg_qualification_rate" DECIMAL(5,2) NOT NULL,
    "trend" VARCHAR(16) NOT NULL,
    "trend_rate" DECIMAL(6,3) NOT NULL,
    "risk_score" SMALLINT NOT NULL,
    "risk_level" VARCHAR(8) NOT NULL,
    "confidence" DECIMAL(4,3) NOT NULL,
    "top_issues_json" JSONB,
    "recommendations_json" JSONB,
    "period_breakdown_json" JSONB,
    "source" VARCHAR(16) NOT NULL,
    "is_acknowledged" BOOLEAN NOT NULL DEFAULT false,
    "acknowledged_at" TIMESTAMPTZ,
    "acknowledged_by" BIGINT REFERENCES "users"("id"),
    "tenant_id" BIGINT NOT NULL,
    "created_by" BIGINT REFERENCES "users"("id"),
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "chk_ai_qual_type" CHECK ("inspection_type" IN ('all', 'incoming', 'inprocess', 'final', 'outgoing')),
    CONSTRAINT "chk_ai_qual_trend" CHECK ("trend" IN ('up', 'flat', 'down', 'nodata')),
    CONSTRAINT "chk_ai_qual_level" CHECK ("risk_level" IN ('low', 'medium', 'high')),
    CONSTRAINT "chk_ai_qual_source" CHECK ("source" IN ('history', 'fallback')),
    CONSTRAINT "chk_ai_qual_risk" CHECK ("risk_score" >= 0 AND "risk_score" <= 100),
    CONSTRAINT "chk_ai_qual_confidence" CHECK ("confidence" >= 0.0 AND "confidence" <= 1.0),
    CONSTRAINT "chk_ai_qual_window" CHECK ("window_days" >= 1 AND "window_days" <= 365)
);

-- 索引：列表查询（按租户 + 创建时间倒序）
CREATE INDEX IF NOT EXISTS "idx_ai_qual_tenant_created" ON "ai_quality_predictions"("tenant_id", "created_at" DESC);
-- 索引：按产品查询历史预测
CREATE INDEX IF NOT EXISTS "idx_ai_qual_product" ON "ai_quality_predictions"("tenant_id", "product_id", "created_at" DESC);
-- 索引：按风险等级（看板）
CREATE INDEX IF NOT EXISTS "idx_ai_qual_risk" ON "ai_quality_predictions"("tenant_id", "risk_level", "created_at" DESC);
-- 索引：按确认状态
CREATE INDEX IF NOT EXISTS "idx_ai_qual_ack" ON "ai_quality_predictions"("tenant_id", "is_acknowledged");

-- 注释
COMMENT ON TABLE "ai_quality_predictions" IS 'AI 质量预测历史表（P2-4）：持久化质量风险评分 / 趋势 / 归因 / 建议';
COMMENT ON COLUMN "ai_quality_predictions"."trend" IS '趋势：up（上升）/ flat（平稳）/ down（下降）/ nodata（无数据）';
COMMENT ON COLUMN "ai_quality_predictions"."trend_rate" IS '趋势变化率（百分点，例 +12.5 表示合格率上升 12.5 个百分点）';
COMMENT ON COLUMN "ai_quality_predictions"."risk_score" IS '风险评分 0-100，越高越危险';
COMMENT ON COLUMN "ai_quality_predictions"."risk_level" IS '风险等级：low（低）/ medium（中）/ high（高）';
COMMENT ON COLUMN "ai_quality_predictions"."top_issues_json" IS '主要问题归因 JSON（top 3：颜色差异 / 色牢度 / 克重偏差 / 纬密偏差 / 强度不足 / 其他）';
COMMENT ON COLUMN "ai_quality_predictions"."recommendations_json" IS '建议措施 JSON（按风险等级 1-3 条）';
COMMENT ON COLUMN "ai_quality_predictions"."period_breakdown_json" IS '按月分段统计 JSON（period / inspections / avg_qualification_rate）';
COMMENT ON COLUMN "ai_quality_predictions"."source" IS '预测来源：history（≥ 5 条历史）/ fallback（< 5 条保守兜底）';
COMMENT ON COLUMN "ai_quality_predictions"."is_acknowledged" IS '是否已被质量管理员确认查看';
