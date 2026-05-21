#![allow(dead_code)]

//! 邮件发送记录 Model
//!
//! 存储邮件发送历史记录

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 邮件发送状态
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum EmailStatus {
    /// 待发送
    #[sea_orm(string_value = "PENDING")]
    Pending,
    /// 发送中
    #[sea_orm(string_value = "SENDING")]
    Sending,
    /// 已发送
    #[sea_orm(string_value = "SENT")]
    Sent,
    /// 发送失败
    #[sea_orm(string_value = "FAILED")]
    Failed,
}

/// 邮件发送记录 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "email_logs")]
pub struct Model {
    /// 记录 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 租户 ID
    pub tenant_id: i32,

    /// 发送人用户 ID
    pub user_id: Option<i32>,

    /// 收件人（多个用逗号分隔）
    pub recipients: String,

    /// 抄送（多个用逗号分隔）
    pub cc: Option<String>,

    /// 密送（多个用逗号分隔）
    pub bcc: Option<String>,

    /// 邮件主题
    pub subject: String,

    /// 邮件正文
    pub body: Option<String>,

    /// 使用的模板ID
    pub template_id: Option<i32>,

    /// 发送状态
    pub status: String,

    /// 错误信息
    pub error_message: Option<String>,

    /// 外部消息ID（第三方邮件服务返回）
    pub external_message_id: Option<String>,

    /// 发送时间
    pub sent_at: Option<DateTime<Utc>>,

    /// 重试次数
    pub retry_count: i32,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 邮件发送记录关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
