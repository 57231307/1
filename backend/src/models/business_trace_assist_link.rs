//! 业务追溯辅助关联 Model
//!
//! 业务追溯辅助关联模块

use sea_orm::entity::prelude::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 业务追溯辅助关联 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "business_trace_assist_links")]
pub struct Model {
    /// 关联 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 追溯 ID（外键）
    pub trace_id: i32,

    /// 辅助类型
    pub assist_type: String,

    /// 辅助 ID
    pub assist_id: i32,

    /// 辅助编码
    pub assist_code: String,

    /// 辅助名称
    pub assist_name: String,

    /// 备注
    pub remarks: Option<String>,

    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 业务追溯辅助关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
