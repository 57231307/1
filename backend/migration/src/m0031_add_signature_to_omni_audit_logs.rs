//! omni_audit_logs 签名列迁移
//!
//! 创建时间: 2026-07-01
//! 关联修复: 八维度审计 P0 8-2（批次 53）— 审计日志签名计算后丢弃，无防篡改
//!
//! 向 omni_audit_logs 表添加 signature 列（VARCHAR(128)，可选），
//! 存储 HMAC-SHA256 防篡改签名（签名材料：trace_id|event_type|action|payload）。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260701000001_add_signature_to_omni_audit_logs/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260701000001_add_signature_to_omni_audit_logs/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
