//! 用户通知偏好设置模型

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user_notification_setting")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    /// 是否启用邮件通知（全局开关）
    pub email_enabled: bool,
    /// 是否启用站内通知（全局开关）
    pub internal_enabled: bool,
    /// 订单通知方式：email, internal, both, none
    pub order_notification_type: String,
    /// 审批通知方式
    pub approval_notification_type: String,
    /// 库存预警通知方式
    pub inventory_notification_type: String,
    /// 采购通知方式
    pub purchase_notification_type: String,
    /// 财务通知方式
    pub finance_notification_type: String,
    /// 系统公告通知方式
    pub system_notification_type: String,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
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

/// 通知类型常量
pub mod notification_type {
    pub const EMAIL: &str = "email";
    pub const INTERNAL: &str = "internal";
    pub const BOTH: &str = "both";
    /// 批次 342 v11 复审 P2 修复：NONE 常量已在 should_send_email/should_send_internal 中显式检查
    pub const NONE: &str = "none";
}
