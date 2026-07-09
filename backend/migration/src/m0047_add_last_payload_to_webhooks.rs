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
        let sql = include_str!(
            "../../migrations/20260710000001_add_last_payload_to_webhooks/up.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!(
            "../../migrations/20260710000001_add_last_payload_to_webhooks/down.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
