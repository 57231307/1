//! processed_events 事件幂等去重表迁移（批次 365 v13 复审 B-P1-8）
//!
//! 创建时间: 2026-07-13
//! 关联修复: B-P1-8 — 事件重复消费无幂等处理，重复生成凭证/重复更新状态
//!
//! 新增 processed_events 表，主键 (consumer_id, event_key) 保证同一消费者对同一事件键
//! 仅处理一次。用于 InventoryTransactionCreated 等带副作用的事件消费者幂等校验。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!(
            "../../migrations/20260713000001_create_processed_events/up.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!(
            "../../migrations/20260713000001_create_processed_events/down.sql"
        );
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
