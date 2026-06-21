use sea_orm_migration::prelude::*;

/// P0-A 数据库迁移根治 — 修复模型与数据库 schema 不一致
///
/// 引用 20260613000001_add_missing_columns SQL 目录：
/// - inventory_stocks：重命名 quantity → quantity_on_hand、reserved_quantity → quantity_reserved，
///   并新增 22 个面料行业特色字段
/// - bpm_task：补齐 17 个流程实例相关字段
/// - currencies：补齐 precision / is_active / is_deleted 字段
/// - omni_audit_logs：补齐 14 个审计相关字段（含 tenant_id 多租户列）
///
/// 编号逻辑：原 m0019_fix_schema_model_sync（引用 20260613000002）后移为 m0020，
/// 当前的 m0020~m0027 依次后移为 m0021~m0028。
/// 部署时新代码会按编号顺序执行：
///   m0019_add_missing_columns（新增列）
///   → m0020_fix_schema_model_sync（其他表重命名 + TOTP 字段）
///   → m0021~m0028（业务表 + 慢查询）
///
/// 全新部署：数据库为空，所有 ALTER / RENAME 一次性成功
/// 已部署：m0019 已记录 m0019_fix_schema_model_sync，SeaORM 会按新名 m0019_add_missing_columns 重新执行
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
