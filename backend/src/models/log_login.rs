#![allow(dead_code)]
//! 登录日志 Model
//!
//! 登录日志模块

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 登录日志 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "log_login")]
pub struct Model {
    /// 日志 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i64,

    /// 日志编号
    pub log_no: String,

    /// 用户 ID
    pub user_id: Option<i32>,

    /// 用户名
    pub username: String,

    /// 真实姓名
    pub real_name: Option<String>,

    /// 登录状态：SUCCESS=成功，FAILED=失败
    pub status: String,

    /// 失败原因
    pub fail_reason: Option<String>,

    /// 登录类型：LOGIN=登录，LOGOUT=登出
    pub login_type: Option<String>,

    /// IP 地址
    pub ip_address: Option<String>,

    /// IP 归属地
    pub ip_location: Option<String>,

    /// 用户代理
    pub user_agent: Option<String>,

    /// 设备类型
    pub device_type: Option<String>,

    /// 浏览器
    pub browser: Option<String>,

    /// 操作系统
    pub os: Option<String>,

    /// 登录时间
    pub login_time: Option<DateTimeUtc>,

    /// 登出时间
    pub logout_time: Option<DateTimeUtc>,

    /// 会话时长（秒）
    pub session_duration_seconds: Option<i64>,

    /// 创建时间
    pub created_at: Option<DateTimeUtc>,
}

/// 登录日志关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
