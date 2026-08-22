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
        let sql = r#"-- 批次 127 v8 复审 P2 修复：导入任务记录表
-- 原 import_export_handler list_import_tasks 返回空列表 vec![]，import_csv/import_excel 不落库任务记录。
-- 现新增 import_tasks 表存储导入任务（id/import_type/status/total_rows/imported_rows/failed_rows/user_id/created_at），
-- handler 在导入前创建 task 记录（status=running），导入完成后更新统计 + 状态。

CREATE TABLE IF NOT EXISTS "import_tasks" (
    "id" SERIAL PRIMARY KEY,
    "import_type" VARCHAR(50) NOT NULL,
    "status" VARCHAR(20) NOT NULL DEFAULT 'running',
    "total_rows" BIGINT NOT NULL DEFAULT 0,
    "imported_rows" BIGINT NOT NULL DEFAULT 0,
    "failed_rows" BIGINT NOT NULL DEFAULT 0,
    "user_id" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "import_tasks" IS '导入任务记录表（批次 127 v8 复审 P2 修复：替代 list_import_tasks 空列表占位）';
COMMENT ON COLUMN "import_tasks"."import_type" IS '导入类型（products/customers/inventory）';
COMMENT ON COLUMN "import_tasks"."status" IS '任务状态（running/success/failed/partial）';
COMMENT ON COLUMN "import_tasks"."total_rows" IS '总行数';
COMMENT ON COLUMN "import_tasks"."imported_rows" IS '成功导入行数';
COMMENT ON COLUMN "import_tasks"."failed_rows" IS '失败行数';
COMMENT ON COLUMN "import_tasks"."user_id" IS '操作用户 ID';

-- 创建按创建时间倒序查询的索引（list_import_tasks 默认按时间倒序）
CREATE INDEX IF NOT EXISTS "idx_import_tasks_created_at" ON "import_tasks" ("created_at" DESC);
-- 创建按用户查询的索引（后续可能支持按用户筛选任务）
CREATE INDEX IF NOT EXISTS "idx_import_tasks_user_id" ON "import_tasks" ("user_id");"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- 批次 127 v8 复审 P2 修复：回滚导入任务记录表
DROP TABLE IF EXISTS "import_tasks";"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
