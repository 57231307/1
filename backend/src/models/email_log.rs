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

    /// 缺陷 6.2 修复：下次重试时间（指数退避：1min/5min/30min，NULL 表示立即可重试）
    pub next_retry_at: Option<DateTime<Utc>>,

    /// 缺陷 6.3 修复：附件 JSON 数组 [{filename, content_base64, content_type}]
    pub attachments: Option<Json>,

    /// 缺陷 6.1 修复：HTML 正文（与 body 区分，body 保留为兼容字段）
    pub html_content: Option<String>,

    /// 缺陷 6.1 修复：纯文本正文
    pub text_content: Option<String>,
}

/// 邮件发送记录关联关系
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
