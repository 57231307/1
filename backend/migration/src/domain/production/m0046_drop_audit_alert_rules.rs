//! 删除 audit_alert_rules 表（批次 202 P1-2）
//!
//! 创建时间: 2026-07-08
//! 关联修复: v12 复审 P1-2 — audit_alert_rule 模型死代码清理
//!
//! audit_alert_rules 表在 m0005 中创建，但对应的模型从未被任何
//! handler/service/route 引用（grep "use crate::models::audit_alert_rule"
//! 无匹配），且审计告警功能不在项目规划文档中，属于遗留死代码。
//! 本迁移删除该表，同步删除对应的 Rust 模型文件。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- 批次 202 P1-2：删除 audit_alert_rules 表
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

DROP TABLE IF EXISTS "audit_alert_rules" CASCADE;"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql =
            r#"-- 批次 202 P1-2 回滚：重新创建 audit_alert_rules 表
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

COMMENT ON TABLE "audit_alert_rules" IS '审计告警规则表 - 存储告警规则配置';"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
