//! import_tasks 表补 file_name / template_id 列（导入任务生命周期真实接入）
//!
//! 背景：handlers/import_export_handler.rs 的 import_task_to_frontend_json 将
//! template_id 恒返 0、file_name 恒返空串（任务表未落库所致）。前端
//! api/data-import.ts 的任务详情/错误日志读取这两个字段。本迁移补列后，
//! 创建任务时落库真实值，消除占位返回。
//!
//! 蓝绿部署规范：file_name NULLABLE；template_id NULLABLE（历史任务无归属）。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
ALTER TABLE import_tasks ADD COLUMN IF NOT EXISTS file_name VARCHAR(255);
ALTER TABLE import_tasks ADD COLUMN IF NOT EXISTS template_id INTEGER;
COMMENT ON COLUMN "import_tasks"."file_name" IS '导入源文件名（上传时记录）';
COMMENT ON COLUMN "import_tasks"."template_id" IS '关联导入模板 ID（可空，历史任务无归属）';
"#;
        manager
            .get_connection()
            .execute_unprepared(sql)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
ALTER TABLE import_tasks DROP COLUMN IF EXISTS template_id;
ALTER TABLE import_tasks DROP COLUMN IF EXISTS file_name;
"#;
        manager
            .get_connection()
            .execute_unprepared(sql)
            .await?;
        Ok(())
    }
}
