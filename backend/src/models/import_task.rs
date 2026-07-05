//! 导入任务记录 Model
//!
//! 批次 127 v8 复审 P2 修复：替代 import_export_handler list_import_tasks 空列表占位 +
//! import_csv/import_excel 不落库任务记录。handler 在导入前创建 task 记录（status=running），
//! 导入完成后更新 imported_rows / failed_rows / status。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 导入任务记录 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "import_tasks")]
pub struct Model {
    /// 任务 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 导入类型（products/customers/inventory）
    pub import_type: String,
    /// 任务状态（running/success/failed/partial）
    pub status: String,
    /// 总行数
    pub total_rows: i64,
    /// 成功导入行数
    pub imported_rows: i64,
    /// 失败行数
    pub failed_rows: i64,
    /// 操作用户 ID
    pub user_id: Option<i32>,
    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
