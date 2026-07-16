use sea_orm_migration::prelude::*;

// V15 P0-S01 修复：role 表新增 data_scope 字段（行级数据权限）
//
// data_scope 取值：
//   'all'  - 全部数据（管理员/总经理）
//   'dept' - 本部门数据（部门经理）
//   'self' - 仅本人数据（普通员工）
//
// 默认 'self'，确保最小权限原则。
// 配合 apply_data_scope 工具函数在 service 查询入口注入行级过滤。

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260716000001_add_data_scope_to_roles/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260716000001_add_data_scope_to_roles/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
