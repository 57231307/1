//! BPM 流程定义 Model
//!
//! BPM 流程定义模块

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// BPM 流程定义 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "bpm_process_definitions")]
pub struct Model {
    /// 流程定义 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 流程名称
    pub name: String,

    /// 流程编码
    pub code: String,

    /// 流程描述
    pub description: Option<String>,

    /// 流程分类
    pub category: Option<String>,

    /// 流程版本
    pub version: Option<String>,

    /// 流程配置（JSON）
    pub config: Option<serde_json::Value>,

    /// 状态：DRAFT=草稿，ACTIVE=激活，INACTIVE=停用
    pub status: String,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// BPM 流程定义关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
