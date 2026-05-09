//! 通知消息模型
//!
//! 存储系统内的通知消息，支持站内信、邮件、短信等多种通知类型

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 通知类型
#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum NotificationType {
    /// 站内信
    #[sea_orm(string_value = "INTERNAL")]
    Internal,
    /// 邮件
    #[sea_orm(string_value = "EMAIL")]
    Email,
    /// 短信
    #[sea_orm(string_value = "SMS")]
    Sms,
    /// 系统通知
    #[sea_orm(string_value = "SYSTEM")]
    System,
}

/// 通知优先级
#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(10))")]
pub enum NotificationPriority {
    /// 低
    #[sea_orm(string_value = "LOW")]
    Low,
    /// 普通
    #[sea_orm(string_value = "NORMAL")]
    Normal,
    /// 高
    #[sea_orm(string_value = "HIGH")]
    High,
    /// 紧急
    #[sea_orm(string_value = "URGENT")]
    Urgent,
}

/// 通知状态
#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum NotificationStatus {
    /// 未读
    #[sea_orm(string_value = "UNREAD")]
    Unread,
    /// 已读
    #[sea_orm(string_value = "READ")]
    Read,
    /// 已处理
    #[sea_orm(string_value = "PROCESSED")]
    Processed,
    /// 已删除
    #[sea_orm(string_value = "DELETED")]
    Deleted,
}

/// 通知消息实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "notifications")]
pub struct Model {
    /// 通知 ID
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 接收用户 ID
    pub user_id: i32,
    /// 通知类型
    pub notification_type: NotificationType,
    /// 通知标题
    pub title: String,
    /// 通知内容
    pub content: String,
    /// 优先级
    pub priority: NotificationPriority,
    /// 状态
    pub status: NotificationStatus,
    /// 业务类型（如：ORDER、APPROVAL、INVENTORY 等）
    pub business_type: Option<String>,
    /// 业务 ID
    pub business_id: Option<i32>,
    /// 跳转链接
    pub action_url: Option<String>,
    /// 发送人 ID（系统通知为 0）
    pub sender_id: Option<i32>,
    /// 发送人名称
    pub sender_name: Option<String>,
    /// 阅读时间
    pub read_at: Option<DateTime<Utc>>,
    /// 处理时间
    pub processed_at: Option<DateTime<Utc>>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

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
