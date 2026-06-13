#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 系统日志 Model
//!
//! 系统日志模块

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 系统日志 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "log_system")]
pub struct Model {
    /// 日志 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 用户 ID
    pub user_id: Option<i32>,

    /// 用户名
    pub username: Option<String>,

    /// 操作模块
    pub module: String,

    /// 操作类型
    pub operation: String,

    /// 请求方法
    pub method: String,

    /// 请求路径
    pub path: String,

    /// 请求参数
    pub params: Option<String>,

    /// 响应状态码
    pub status_code: Option<i32>,

    /// IP 地址
    pub ip_address: Option<String>,

    /// 用户代理
    pub user_agent: Option<String>,

    /// 错误信息
    pub error_message: Option<String>,

    /// 执行时间（毫秒）
    pub execution_time: Option<i64>,

    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 系统日志关联关系
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
