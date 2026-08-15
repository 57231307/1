//! 导入任务记录表迁移（批次 127 v8 复审 P2）
//!
//! 创建时间: 2026-07-05
//! 关联修复: v8 复审 P2 — import_export_handler list_import_tasks 空列表占位 +
//! import_csv/import_excel 不落库任务记录
//!
//! 创建 import_tasks 表存储导入任务记录（id/import_type/status/total_rows/
//! imported_rows/failed_rows/user_id/created_at），替代 list_import_tasks 返回的空列表。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260705000003_create_import_tasks/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260705000003_create_import_tasks/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
