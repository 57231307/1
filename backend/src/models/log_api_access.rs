#![allow(dead_code, unused_imports, unused_variables)]
//! API 访问日志 Model
//!
//! API 访问日志模块

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// API 访问日志 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "log_api_accesses")]
pub struct Model {
    /// 日志 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 用户 ID
    pub user_id: Option<i32>,

    /// 用户名
    pub username: Option<String>,

    /// 请求方法
    pub method: String,

    /// 请求路径
    pub path: String,

    /// 查询参数
    pub query_params: Option<String>,

    /// 请求体
    pub request_body: Option<String>,

    /// 响应状态码
    pub status_code: Option<i32>,

    /// 响应体大小（字节）
    pub response_size: Option<i64>,

    /// 执行时间（毫秒）
    pub execution_time: i64,

    /// IP 地址
    pub ip_address: Option<String>,

    /// 用户代理
    pub user_agent: Option<String>,

    /// 错误信息
    pub error_message: Option<String>,

    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// API 访问日志关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
