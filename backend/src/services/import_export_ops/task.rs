//! 导入任务记录管理子模块（import_export_ops::task）
//!
//! 批次 127 v8 复审 P2 修复：原 list_import_tasks 返回空列表 vec![]，
//! import_csv/import_excel 不落库任务记录。
//! 现新增 task 管理方法：create_import_task / update_import_task / list_import_tasks。
//! handler 在导入前创建 task 记录（status=running），导入完成后更新统计 + 状态。
//!
//! 从原 `import_export_service.rs` 迁移 3 个方法：
//! - `create_import_task`：导入开始时创建任务记录（status=running）
//! - `update_import_task`：导入完成时根据 ImportResult 更新任务状态与统计
//! - `list_import_tasks`：按创建时间倒序返回最近 100 条任务记录

use crate::models::status::import_task as import_status;
use crate::services::import_export_service::{ImportExportService, ImportResult};
use crate::utils::error::AppError;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, QueryOrder, QuerySelect};

impl ImportExportService {
    /// 创建导入任务记录（导入开始时调用）
    ///
    /// 批次 127 v8 复审 P2 修复：在 import_csv/import_excel 执行实际导入前创建任务记录，
    /// status 初始化为 "running"，total_rows 为待导入行数。
    /// 返回任务 ID 供后续 update_import_task 使用。
    pub async fn create_import_task(
        &self,
        import_type: &str,
        total_rows: u64,
        user_id: i32,
    ) -> Result<i32, AppError> {
        // 批次 357 v13 复审 baseline 清零：移除 unused import self（仅使用 ActiveModel）
        use crate::models::import_task::ActiveModel;
        use chrono::Utc;

        let now = Utc::now();
        let active_model = ActiveModel {
            import_type: Set(import_type.to_string()),
            status: Set(import_status::RUNNING.to_string()),
            total_rows: Set(total_rows as i64),
            imported_rows: Set(0),
            failed_rows: Set(0),
            user_id: Set(Some(user_id)),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        };

        let model = active_model.insert(&*self.db).await?;
        Ok(model.id)
    }

    /// 更新导入任务记录（导入完成时调用）
    ///
    /// 批次 127 v8 复审 P2 修复：根据 ImportResult 更新任务的 imported_rows / failed_rows / status。
    /// 状态判定规则：
    /// - failed == 0 && imported > 0 → "success"
    /// - imported == 0 && failed > 0 → "failed"
    /// - imported > 0 && failed > 0 → "partial"
    /// - 其他（imported == 0 && failed == 0）→ "success"（空导入视为成功）
    pub async fn update_import_task(
        &self,
        task_id: i32,
        result: &ImportResult,
    ) -> Result<(), AppError> {
        // 批次 357 v13 复审 baseline 清零：移除 unused import self（仅使用 ActiveModel）
        use crate::models::import_task::ActiveModel;
        use chrono::Utc;

        let status = if result.failed == 0 {
            import_status::SUCCESS
        } else if result.imported == 0 {
            import_status::FAILED
        } else {
            import_status::PARTIAL
        };

        let active_model = ActiveModel {
            id: Set(task_id),
            status: Set(status.to_string()),
            imported_rows: Set(result.imported as i64),
            failed_rows: Set(result.failed as i64),
            updated_at: Set(Utc::now().into()),
            ..Default::default()
        };

        active_model.update(&*self.db).await?;
        Ok(())
    }

    /// 获取导入任务列表（list_import_tasks handler 调用）
    ///
    /// 批次 127 v8 复审 P2 修复：替代原 list_import_tasks 返回的空列表 vec![]。
    /// 按创建时间倒序返回最近 100 条任务记录。
    pub async fn list_import_tasks(
        &self,
    ) -> Result<Vec<crate::models::import_task::Model>, AppError> {
        use crate::models::import_task;

        let tasks = import_task::Entity::find()
            .order_by(import_task::Column::CreatedAt, sea_orm::Order::Desc)
            .limit(100)
            .all(&*self.db)
            .await?;
        Ok(tasks)
    }
}
