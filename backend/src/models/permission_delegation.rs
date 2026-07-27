//! 权限委托 Model（V15 P1 12.6）
//!
//! 支持时限化临时权限委托：
//! - 委托人（delegator）将权限码委托给被委托人（delegatee）
//! - 必须有时限（valid_from + valid_until），过期自动失效
//! - 禁止链式委托（is_chain_allowed 默认 false）
//! - 委托必须记录审计日志

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 权限委托实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "permission_delegations")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 委托人用户 ID
    pub delegator_id: i32,
    /// 被委托人用户 ID
    pub delegatee_id: i32,
    /// 委托的权限码（如 "sales:approve"）
    pub permission_code: String,
    /// 委托生效时间
    pub valid_from: DateTime<Utc>,
    /// 委托失效时间
    pub valid_until: DateTime<Utc>,
    /// 是否允许被委托人再委托（默认 false，禁止链式委托）
    pub is_chain_allowed: bool,
    /// 委托状态：pending / active / expired / revoked
    pub status: String,
    /// 委托原因
    pub reason: Option<String>,
    /// 撤销时间
    pub revoked_at: Option<DateTime<Utc>>,
    /// 撤销人用户 ID
    pub revoked_by: Option<i32>,
    /// 撤销原因
    pub revoke_reason: Option<String>,
    /// 创建人用户 ID
    pub created_by: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::DelegatorId",
        to = "super::user::Column::Id"
    )]
    Delegator,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::DelegateeId",
        to = "super::user::Column::Id"
    )]
    Delegatee,
}

// 注：Delegator 和 Delegatee 均指向 user::Entity，SeaORM 仅允许一个 Related<user::Entity> 实现。
// 此处不实现 Related trait，需要关联查询时通过手写 JOIN 实现。
// 基础 CRUD 操作（find/insert/update/delete）不受影响。

impl ActiveModelBehavior for ActiveModel {}

/// 委托状态常量
pub mod delegation_status {
    pub const PENDING: &str = "pending";
    pub const ACTIVE: &str = "active";
    pub const EXPIRED: &str = "expired";
    pub const REVOKED: &str = "revoked";
}
