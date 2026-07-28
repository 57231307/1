//! 客户数据共享 Model
//!
//! V15 P1 18.4-D3 修复：CRM 数据共享时效机制
//! 业务背景：共享客户无时效控制，权限收回困难。
//! 设计：共享记录表，支持时效控制（expire_at）和主动撤销。
//!
//! 共享权限类型：
//! - view    只读（可查看客户信息）
//! - edit    编辑（可修改客户信息）
//! - full    完全（含团队管理权限）
//!
//! 状态：
//! - active  生效中
//! - expired 已过期（时效到达自动过期）
//! - revoked 已撤销（被共享人或管理员主动撤销）
//!
//! 对应迁移：m0082_create_customer_team_and_share

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 共享权限：只读
pub const SHARE_PERMISSION_VIEW: &str = "view";
/// 共享权限：编辑
pub const SHARE_PERMISSION_EDIT: &str = "edit";
/// 共享权限：完全
pub const SHARE_PERMISSION_FULL: &str = "full";

/// 共享状态：生效中
pub const SHARE_STATUS_ACTIVE: &str = "active";
/// 共享状态：已过期
pub const SHARE_STATUS_EXPIRED: &str = "expired";
/// 共享状态：已撤销
pub const SHARE_STATUS_REVOKED: &str = "revoked";

/// 客户数据共享 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "customer_shares")]
pub struct Model {
    /// 共享 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 客户 ID（外键 customers.id）
    pub customer_id: i32,

    /// 共享方用户 ID（共享客户的人）
    pub shared_by_user_id: i32,

    /// 共享方姓名（冗余字段）
    pub shared_by_user_name: Option<String>,

    /// 被共享方用户 ID（接收共享的人）
    pub shared_to_user_id: i32,

    /// 被共享方姓名（冗余字段）
    pub shared_to_user_name: Option<String>,

    /// 共享权限：view / edit / full
    pub permission: String,

    /// 共享状态：active / expired / revoked
    pub status: String,

    /// 共享生效时间
    pub shared_at: DateTime<Utc>,

    /// 共享过期时间（NULL 表示永久共享，但建议设置时效）
    pub expire_at: Option<DateTime<Utc>>,

    /// 撤销时间（NULL 表示未撤销）
    pub revoked_at: Option<DateTime<Utc>>,

    /// 撤销人 ID
    pub revoked_by: Option<i32>,

    /// 撤销原因
    pub revoke_reason: Option<String>,

    /// 共享原因
    pub share_reason: Option<String>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 客户数据共享关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::SharedByUserId",
        to = "super::user::Column::Id"
    )]
    SharedByUser,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::SharedToUserId",
        to = "super::user::Column::Id"
    )]
    SharedToUser,
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
