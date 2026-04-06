//! 报表定义 Model
//!
//! 报表定义模块

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 报表定义 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "report_definition")]
pub struct Model {
    /// 报表 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 报表编码
    pub report_code: String,

    /// 报表名称
    pub name: String,

    /// 报表类型
    pub report_type: String,

    /// 数据源
    pub data_source: String,

    /// SQL 查询
    pub sql_query: Option<String>,

    /// 报表配置（JSON）
    pub config: Option<serde_json::Value>,

    /// 报表描述
    pub description: Option<String>,

    /// 状态：DRAFT=草稿，ACTIVE=激活，INACTIVE=停用
    pub status: String,

    /// 创建人 ID
    pub created_by: i32,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 报表定义关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
