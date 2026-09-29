//! 系统更新任务与备份记录 Entity（m0055 落库，替代内存存储）
//!
//! - `system_update_tasks`：下载/安装/取消等更新任务记录
//! - `system_update_backups`：系统备份记录

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// ---------------- 更新任务 ----------------

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "system_update_tasks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 任务编号（UPD-yyyyMMddHHmmss 格式）
    pub task_code: String,
    /// 任务类型：download / install / cancel
    pub task_type: String,
    /// 源版本（当前版本）
    #[sea_orm(column_default = "")]
    pub source_version: String,
    /// 目标版本
    #[sea_orm(column_default = "")]
    pub target_version: String,
    /// 状态：pending/downloaded/installing/completed/failed/rolled_back
    #[sea_orm(column_default = "pending")]
    pub status: String,
    /// 进度百分比 0-100
    #[sea_orm(column_default = "0")]
    pub progress: i32,
    /// 错误信息（失败/取消时）
    pub error_message: Option<String>,
    /// 创建人用户 ID
    #[sea_orm(column_default = "0")]
    pub created_by: i32,
    /// 创建人用户名
    #[sea_orm(column_default = "")]
    pub created_by_name: String,
    /// 完成时间
    pub completed_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
