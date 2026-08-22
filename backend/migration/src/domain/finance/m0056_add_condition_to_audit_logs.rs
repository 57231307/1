use sea_orm_migration::prelude::*;

// Batch 473 P0-S19 修复：审计日志补齐 condition 字段
//
// V15 审计 13.6.1 矩阵要求 8 个核心审计字段，复审发现 6/8 已实现：
//   ✅ user_id / resource_type / resource_id / operation_type / ip_address / timestamp(created_at)
//   ❌ condition（请求条件/查询条件，独立字段，目前仅 request_body 可部分覆盖，语义不明确）
//   ⚠️ result 字段虽不存在，但 response_status 可视为 result（HTTP 状态码承载操作结果）
//
// 本批次补齐 condition 字段：
//   1. audit_logs 表新增 condition TEXT 列
//   2. omni_audit_logs 表新增 condition TEXT 列
//   3. OmniAuditMessage 结构体新增 condition 字段，中间件提取 query_string 作为 condition 写入
//
// 设计依据：V15 审计报告 类十三 P0-S19
// 关联文件：models/audit_log.rs / models/omni_audit_log.rs / services/omni_audit_service.rs / middleware/omni_audit.rs

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- audit_logs 表新增 condition 字段（请求条件/查询条件）
                -- 与 request_body 区分：request_body 记录完整请求体，condition 仅记录查询条件（query string）
                -- 用于快速筛选特定条件下的导出/查询审计记录
                ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS condition TEXT;

                -- omni_audit_logs 表新增 condition 字段
                ALTER TABLE omni_audit_logs ADD COLUMN IF NOT EXISTS condition TEXT;
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE audit_logs DROP COLUMN IF EXISTS condition;
                ALTER TABLE omni_audit_logs DROP COLUMN IF EXISTS condition;
                "#,
            )
            .await?;
        Ok(())
    }
}
