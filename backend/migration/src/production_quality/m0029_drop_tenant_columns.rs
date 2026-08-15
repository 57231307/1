//! 完整删除租户功能迁移
//!
//! 创建时间: 2026-06-28
//! 关联计划: 租户功能下线
//!
//! 本迁移用于完整删除系统中的多租户功能：
//! - 删除所有业务表上的 tenant_id 列及其相关索引
//! - 删除全部租户管理表（tenants / tenant_users / tenant_configs 等）
//!
//! 执行顺序：先删索引 → 再删业务表 tenant_id 列 → 最后删租户管理表。
//! 该迁移不可逆，回滚（down）不会恢复已删除的列与表。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260628000001_drop_tenant_columns/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260628000001_drop_tenant_columns/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
