//! 审计日志增强迁移
//!
//! 创建时间: 2026-06-18
//! 关联计划: 2026-06-18-p13-batch1-comprehensive-plan.md §2.1
//!
//! 在 main 已有的 audit_logs 表（m0001 创建）基础上，
//! 增量添加 operation_type / severity / request_id / before_snapshot / after_snapshot 五列，
//! 全部使用 ADD COLUMN IF NOT EXISTS 防止迁移重入。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- 审计日志增强迁移（P13 批 1 P3-2）
-- 创建时间: 2026-06-18
-- 关联计划: 2026-06-18-p13-batch1-comprehensive-plan.md §2.1
--
-- 在 main 已有的 audit_logs 表（m0001 创建，12 字段）基础上，
-- 增量添加操作类型/严重级别/请求追踪/差异快照等新列。
-- 全部使用 ADD COLUMN IF NOT EXISTS 防止迁移重入。

ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "operation_type" VARCHAR(20);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "severity" VARCHAR(20) NOT NULL DEFAULT 'INFO';
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "request_id" VARCHAR(64);
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "before_snapshot" JSONB;
ALTER TABLE "audit_logs" ADD COLUMN IF NOT EXISTS "after_snapshot" JSONB;

-- 索引优化：操作类型/严重级别/请求追踪查询
CREATE INDEX IF NOT EXISTS "idx_audit_log_op_type" ON "audit_logs"("operation_type");
CREATE INDEX IF NOT EXISTS "idx_audit_log_severity" ON "audit_logs"("severity");
CREATE INDEX IF NOT EXISTS "idx_audit_log_request_id" ON "audit_logs"("request_id");
-- 注意：audit_logs 表自 m0001 创建以来从未包含 tenant_id 列，
-- 原语句 CREATE INDEX ... ON "audit_logs"("tenant_id", ...) 会导致
-- "column tenant_id does not exist" 错误，已移除。
-- 租户功能已在 20260628000001_drop_tenant_columns 中完整下线，
-- 此索引无需创建。

COMMENT ON COLUMN "audit_logs"."operation_type" IS '操作类型枚举：CREATE/UPDATE/DELETE/LOGIN/EXPORT/LOGOUT/QUERY/OTHER';
COMMENT ON COLUMN "audit_logs"."severity" IS '严重级别：INFO/WARN/ERROR/CRITICAL';
COMMENT ON COLUMN "audit_logs"."request_id" IS '请求追踪 ID（关联 trace_context middleware 生成的 trace_id）';
COMMENT ON COLUMN "audit_logs"."before_snapshot" IS '变更前快照 JSON（与 old_value 同义，新 API 推荐使用）';
COMMENT ON COLUMN "audit_logs"."after_snapshot" IS '变更后快照 JSON（与 new_value 同义，新 API 推荐使用）';"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- 审计日志增强迁移回滚（P13 批 1 P3-2）
--
-- 反向删除扩展字段与索引。
-- 注意：使用 IF EXISTS 保护，对应 m0001 创建的原始列不会被误删。

DROP INDEX IF EXISTS "idx_audit_log_tenant_created";
DROP INDEX IF EXISTS "idx_audit_log_request_id";
DROP INDEX IF EXISTS "idx_audit_log_severity";
DROP INDEX IF EXISTS "idx_audit_log_op_type";

ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "after_snapshot";
ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "before_snapshot";
ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "request_id";
ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "severity";
ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "operation_type";"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
