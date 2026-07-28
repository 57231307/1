//! 客户团队成员关联 Model
//!
//! V15 P1 18.4-D2 修复：CRM 团队协作机制
//! 业务背景：大客户需多人跟进时无法协作，仅 owner_id 单人负责。
//! 设计：客户-团队成员关联表，支持多人协作跟进同一客户。
//!
//! 角色类型：
//! - primary   主负责人（owner，唯一）
//! - member    团队成员（多人）
//! - assistant 协助人员（临时支持）
//!
//! 对应迁移：m0082_create_customer_team_and_share

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 团队角色：主负责人
pub const TEAM_ROLE_PRIMARY: &str = "primary";
/// 团队角色：团队成员
pub const TEAM_ROLE_MEMBER: &str = "member";
/// 团队角色：协助人员
pub const TEAM_ROLE_ASSISTANT: &str = "assistant";

/// 客户团队成员关联 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "customer_team_members")]
pub struct Model {
    /// 关联 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 客户 ID（外键 customers.id）
    pub customer_id: i32,

    /// 团队成员用户 ID
    pub user_id: i32,

    /// 团队成员姓名（冗余字段）
    pub user_name: Option<String>,

    /// 团队角色：primary / member / assistant
    pub team_role: String,

    /// 是否活跃
    pub is_active: bool,

    /// 加入时间
    pub joined_at: DateTime<Utc>,

    /// 退出时间（NULL 表示仍在团队中）
    pub left_at: Option<DateTime<Utc>>,

    /// 备注
    pub notes: Option<String>,

    /// 操作人（添加该成员的用户）
    pub created_by: Option<i32>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 客户团队成员关联关系
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
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
