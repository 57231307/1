-- 批次 202 P1-2：删除 audit_alert_rules 表
-- 创建时间: 2026-07-08
-- 关联修复: v12 复审 P1-2 — audit_alert_rule 模型死代码清理
--
-- audit_alert_rules 表在 m0005（20260527000001）中创建，但对应的 Rust 模型
-- 从未被任何 handler/service/route 引用，且审计告警功能不在项目规划文档中，
-- 属于遗留死代码。本迁移删除该表。
--
-- 安全性确认：
-- 1. 无其他表通过外键引用 audit_alert_rules（grep REFERENCES 无匹配）
-- 2. 无业务代码引用 audit_alert_rule 模型（grep "use crate::models::audit_alert_rule" 无匹配）
-- 3. 审计告警功能不在项目规划文档中（.monkeycode/docs/ 无相关记录）

DROP TABLE IF EXISTS "audit_alert_rules" CASCADE;
