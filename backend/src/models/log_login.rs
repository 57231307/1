//! 登录日志 Model
//!
//! 登录日志模块

use sea_orm::entity::prelude::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 登录日志 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "log_logins")]
pub struct Model {
    /// 日志 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 用户 ID
    pub user_id: Option<i32>,

    /// 用户名
    pub username: String,

    /// 登录类型：LOGIN=登录，LOGOUT=登出
    pub login_type: String,

    /// IP 地址
    pub ip_address: String,

    /// 用户代理
    pub user_agent: Option<String>,

    /// 登录状态：SUCCESS=成功，FAILED=失败
    pub status: String,

    /// 失败原因
    pub fail_reason: Option<String>,

    /// 登录时间
    pub login_time: DateTime<Utc>,
}

/// 登录日志关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
