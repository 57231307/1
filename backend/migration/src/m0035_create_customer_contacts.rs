//! 客户联系人表迁移（批次 90b P2-12）
//!
//! 创建时间: 2026-07-03
//! 关联修复: 前端 crm/detail.vue "新增联系人功能待实现" 占位符实现
//!
//! 新建 customer_contacts 表，记录客户的多个联系人信息（含主联系人标识）。
//! 替代 crm_customer_handler.rs:list_contacts 中从 crm_lead 拼接 JSON 的伪实现。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260703000004_create_customer_contacts/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260703000004_create_customer_contacts/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
