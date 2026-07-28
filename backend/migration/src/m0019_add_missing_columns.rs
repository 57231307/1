use sea_orm_migration::prelude::*;

/// P0-A 数据库迁移根治：修复模型与 schema 不一致（inventory_stocks 重命名+22 面料字段 / bpm_task 17 字段 / currencies 3 字段 / omni_audit_logs 14 字段）
/// 编号逻辑：m0019 后移 m0020~m0028，全新部署一次性成功，已部署按新名重新执行
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260613000001_add_missing_columns/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260613000001_add_missing_columns/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
