//! CRM 标签字典表迁移（批次 122 v8 复审 P1）
//!
//! 创建时间: 2026-07-05
//! 关联修复: v8 复审 P1 — crm_customer_handler list_tags 硬编码 + create_tag/delete_tag 假实现
//!
//! 创建 crm_tag 表存储标签字典（id/name/color/category/created_by/created_at/updated_at），
//! 替代原 list_tags 返回的硬编码 5 个标签。初始化预定义标签保证向后兼容。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260705000002_create_crm_tags/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260705000002_create_crm_tags/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
