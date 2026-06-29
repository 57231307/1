-- CRM 公海回收规则表 migration - crm_recycle_rules
-- 创建时间: 2026-06-29
-- 关联修复: 批次 23 v5 P0-4 — CRM 回收规则内存存储导致重启丢失
--
-- 将原本存于 handlers/missing_handlers.rs 静态内存中的回收规则迁移至数据库持久化，
-- 避免进程重启后丢失运行时修改。

CREATE TABLE IF NOT EXISTS "crm_recycle_rules" (
    "id" SERIAL PRIMARY KEY,
    "name" VARCHAR(100) NOT NULL,
    "days" INT NOT NULL,
    "is_enabled" BOOLEAN NOT NULL DEFAULT TRUE,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "chk_crm_recycle_rules_days" CHECK ("days" >= 1 AND "days" <= 365)
);

-- 索引：按启用状态查询
CREATE INDEX IF NOT EXISTS "idx_crm_recycle_rules_enabled" ON "crm_recycle_rules"("is_enabled");

COMMENT ON TABLE "crm_recycle_rules" IS 'CRM 公海回收规则表 - 定义客户未跟进后自动回收到公海的天数阈值';
COMMENT ON COLUMN "crm_recycle_rules"."name" IS '规则名称';
COMMENT ON COLUMN "crm_recycle_rules"."days" IS '未跟进天数阈值，超过该值后自动回收（1-365）';
COMMENT ON COLUMN "crm_recycle_rules"."is_enabled" IS '是否启用';

-- 初始规则数据（与原内存存储保持一致，确保平滑迁移）
INSERT INTO "crm_recycle_rules" ("name", "days", "is_enabled") VALUES
    ('标准回收规则（30 天未跟进）', 30, TRUE),
    ('高价值客户延长（90 天未跟进）', 90, TRUE),
    ('快速回收（7 天未跟进）', 7, FALSE)
ON CONFLICT DO NOTHING;
