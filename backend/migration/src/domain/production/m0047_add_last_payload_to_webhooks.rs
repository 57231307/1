//! webhooks 表 last_payload + last_event 列迁移（批次 251 v14 中风险修复）
//!
//! 创建时间: 2026-07-10
//! 关联修复: v14 中风险 — webhook retry 未持久化 payload
//!
//! 向 webhooks 表添加 last_payload（TEXT）和 last_event（VARCHAR(100)）列，
//! 用于持久化最后一次发送的业务负载和事件类型，支持 retry 重投原始数据。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- 批次 251 v14 中风险修复：webhooks 表添加 last_payload + last_event 列
-- 原 webhook 发送时 payload 仅存内存，发送失败后丢失，retry 重构假 payload 无法重投原始数据。
-- 新增 last_payload（原始业务负载）+ last_event（原始事件类型）列，支持真实重试。

ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "last_payload" TEXT;
ALTER TABLE "webhooks" ADD COLUMN IF NOT EXISTS "last_event" VARCHAR(100);

COMMENT ON COLUMN "webhooks"."last_payload" IS '最后一次发送的原始业务负载（批次 251 修复：retry 重投原始数据用）';
COMMENT ON COLUMN "webhooks"."last_event" IS '最后一次发送的事件类型（批次 251 修复：retry 重投原始事件用）';"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- 批次 251 回滚：移除 webhooks.last_payload + last_event 列
ALTER TABLE "webhooks" DROP COLUMN IF EXISTS "last_payload";
ALTER TABLE "webhooks" DROP COLUMN IF EXISTS "last_event";"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
