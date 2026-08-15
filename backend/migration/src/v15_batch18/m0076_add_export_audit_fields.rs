use sea_orm_migration::prelude::*;

// V15 P1 batch-11 缺陷 3-3 修复：
// 为 audit_logs 表新增导出操作专属字段，支持导出审计完整性矩阵落地：
// - export_record_count：导出数据行数，用于大批量导出识别（>80% 上限触发告警）
// - export_query_filter：导出时的筛选条件 JSON，用于追溯导出数据范围
// - export_file_format：导出文件格式（xlsx/csv/pdf），格式合规审计
// - export_approval_token：二级审批 token（敏感数据导出），10 分钟有效期
// - export_watermark_user：导出文件水印中的用户名，二次泄露追溯
//
// 关联文件：
//   - models/audit_log.rs（新增对应字段）
//   - migrations/20260729000001_add_export_audit_fields/up.sql（SQL 文件版本，本迁移整合执行）
//   - handlers 中的导出 handler 可在 AuditEvent 中填充这些字段

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- ============================================================
                -- V15 P1-3-3：audit_logs 表导出专属字段
                -- ============================================================

                ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_record_count" INTEGER;
                ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_query_filter" TEXT;
                ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_file_format" VARCHAR(20);
                ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_approval_token" VARCHAR(128);
                ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "export_watermark_user" VARCHAR(100);

                -- 索引：按导出条数筛选大批量导出（合规审查常用）
                CREATE INDEX IF NOT EXISTS "idx_audit_log_export_count"
                    ON "audit_logs"("export_record_count");
                -- 索引：按审批 token 查询敏感数据导出追溯
                CREATE INDEX IF NOT EXISTS "idx_audit_log_approval_token"
                    ON "audit_logs"("export_approval_token");

                COMMENT ON COLUMN "audit_logs"."export_record_count" IS 'V15 P1-3-3：导出数据行数，用于大批量导出识别（>80% 上限触发告警）';
                COMMENT ON COLUMN "audit_logs"."export_query_filter" IS 'V15 P1-3-3：导出时的筛选条件 JSON，用于追溯导出数据范围';
                COMMENT ON COLUMN "audit_logs"."export_file_format" IS 'V15 P1-3-3：导出文件格式（xlsx/csv/pdf），格式合规审计';
                COMMENT ON COLUMN "audit_logs"."export_approval_token" IS 'V15 P1-3-3：二级审批 token（敏感数据导出），10 分钟有效期';
                COMMENT ON COLUMN "audit_logs"."export_watermark_user" IS 'V15 P1-3-3：导出文件水印中的用户名，二次泄露追溯';
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
                DROP INDEX IF EXISTS "idx_audit_log_approval_token";
                DROP INDEX IF EXISTS "idx_audit_log_export_count";
                ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "export_watermark_user";
                ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "export_approval_token";
                ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "export_file_format";
                ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "export_query_filter";
                ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "export_record_count";
                "#,
            )
            .await?;
        Ok(())
    }
}
