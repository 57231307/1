#![allow(dead_code)]
//! 导入任务记录 Model
//!
//! 记录每一次数据导入的执行情况：导入前建 task（status=running），
//! 导入完成后回填 imported_rows / failed_rows 并置终态，供导入历史列表查询与失败追溯。

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
    /// 导入源文件名（m0054 补列，上传时记录）
    pub file_name: Option<String>,
    /// 关联导入模板 ID（m0054 补列，历史任务无归属为空）
    pub template_id: Option<i32>,
    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
