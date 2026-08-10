use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 长任务状态模型 - batch-21 P2 25.4-I: 长任务处理机制
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "long_running_tasks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 任务类型（如：system_update, data_export, data_import）
    pub task_type: String,
    /// 任务状态（pending/running/completed/failed/cancelled）
    pub status: String,
    /// 任务参数（JSON 格式）
    pub params: Option<serde_json::Value>,
    /// 任务进度（0-100）
    pub progress: i32,
    /// 任务结果（JSON 格式）
    pub result: Option<serde_json::Value>,
    /// 错误信息
    pub error_message: Option<String>,
    /// 开始时间
    pub started_at: Option<DateTime<Utc>>,
    /// 完成时间
    pub completed_at: Option<DateTime<Utc>>,
    /// 创建人
    pub created_by: Option<i32>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// 任务状态常量
pub mod status {
    pub const PENDING: &str = "pending";
    pub const RUNNING: &str = "running";
    pub const COMPLETED: &str = "completed";
    pub const FAILED: &str = "failed";
    pub const CANCELLED: &str = "cancelled";
}
