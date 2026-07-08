-- 批次 202 P1-2 回滚：重新创建 audit_alert_rules 表
-- 创建时间: 2026-07-08
-- 关联修复: v12 复审 P1-2 — audit_alert_rule 模型死代码清理
--
-- 回滚时重新创建 audit_alert_rules 表（结构同 m0005 原始定义）。

CREATE TABLE IF NOT EXISTS "audit_alert_rules" (
    "id" SERIAL PRIMARY KEY,
    "rule_name" VARCHAR(200) NOT NULL,
    "event_type" VARCHAR(100) NOT NULL,
    "condition_expr" JSONB,
    "alert_level" VARCHAR(20) NOT NULL,
    "is_active" BOOLEAN NOT NULL DEFAULT true,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "audit_alert_rules" IS '审计告警规则表 - 存储告警规则配置';
