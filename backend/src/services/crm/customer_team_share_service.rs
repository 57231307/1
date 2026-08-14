//! 客户团队协作与数据共享服务（crm/customer_team_share）
//!
//! V15 P1 18.4-D2/D3 修复：
//! - 18.4-D2：团队协作机制 — 多人协作跟进同一客户
//! - 18.4-D3：数据共享时效 — 共享带过期时间和撤销机制
//!
//! 设计依据：V15 审计报告 batch-15 维度 18.4 缺陷 D2/D3（P1）
//!
//! 团队协作（18.4-D2）：
//! - add_team_member：添加团队成员（主负责人/成员/协助人员）
//! - remove_team_member：移除团队成员（设置 left_at + is_active=false）
//! - list_team_members：列出客户的活跃团队成员
//! - list_user_teams：列出用户参与的客户团队
//! - is_team_member：校验用户是否为客户的团队成员（用于权限校验）
//!
//! 数据共享（18.4-D3）：
//! - share_customer：共享客户给其他用户（带时效和权限）
//! - revoke_share：撤销共享（主动收回权限）
//! - list_customer_shares：列出客户的共享记录
//! - list_user_shares：列出用户收到的共享
//! - check_share_permission：校验用户的共享权限（用于数据范围过滤）
//! - expire_overdue_shares：过期超期共享（定时清理）

use chrono::{DateTime, Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::models::customer_share as share_model;
use crate::models::customer_team_member as team_model;
use crate::models::{customer, customer_share, customer_team_member, user};
use crate::utils::error::AppError;

// =====================================================
// 请求/响应 DTO
// =====================================================

/// 添加团队成员请求
#[derive(Debug, Clone, Deserialize)]
pub struct AddTeamMemberRequest {
    pub customer_id: i32,
    pub user_id: i32,
    /// 团队角色：primary/member/assistant（默认 member）
    pub team_role: Option<String>,
    pub notes: Option<String>,
}

/// 共享客户请求
#[derive(Debug, Clone, Deserialize)]
pub struct ShareCustomerRequest {
    pub customer_id: i32,
    /// 被共享方用户 ID
    pub shared_to_user_id: i32,
    /// 共享权限：view/edit/full（默认 view）
    pub permission: Option<String>,
    /// 共享时效（天），None 表示永久共享（建议设置时效）
    pub duration_days: Option<i32>,
    /// 共享原因
    pub share_reason: Option<String>,
}

/// 撤销共享请求
#[derive(Debug, Clone, Deserialize)]
pub struct RevokeShareRequest {
    pub share_id: i32,
    /// 撤销原因
    pub revoke_reason: Option<String>,
}

/// 团队成员 DTO
#[derive(Debug, Clone, Serialize)]
pub struct TeamMemberDto {
    pub id: i32,
    pub customer_id: i32,
    pub user_id: i32,
    pub user_name: Option<String>,
    pub team_role: String,
    pub is_active: bool,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}

impl From<customer_team_member::Model> for TeamMemberDto {
    fn from(m: customer_team_member::Model) -> Self {
        Self {
            id: m.id,
            customer_id: m.customer_id,
            user_id: m.user_id,
            user_name: m.user_name,
            team_role: m.team_role,
            is_active: m.is_active,
            joined_at: m.joined_at,
            left_at: m.left_at,
            notes: m.notes,
        }
    }
}

/// 共享记录 DTO
#[derive(Debug, Clone, Serialize)]
pub struct CustomerShareDto {
    pub id: i32,
    pub customer_id: i32,
    pub shared_by_user_id: i32,
    pub shared_by_user_name: Option<String>,
    pub shared_to_user_id: i32,
    pub shared_to_user_name: Option<String>,
    pub permission: String,
    pub status: String,
    pub shared_at: DateTime<Utc>,
    pub expire_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoked_by: Option<i32>,
    pub revoke_reason: Option<String>,
    pub share_reason: Option<String>,
}

impl From<customer_share::Model> for CustomerShareDto {
    fn from(m: customer_share::Model) -> Self {
        Self {
            id: m.id,
            customer_id: m.customer_id,
            shared_by_user_id: m.shared_by_user_id,
            shared_by_user_name: m.shared_by_user_name,
            shared_to_user_id: m.shared_to_user_id,
            shared_to_user_name: m.shared_to_user_name,
            permission: m.permission,
            status: m.status,
            shared_at: m.shared_at,
            expire_at: m.expire_at,
            revoked_at: m.revoked_at,
            revoked_by: m.revoked_by,
            revoke_reason: m.revoke_reason,
            share_reason: m.share_reason,
        }
    }
}

/// 过期清理结果
#[derive(Debug, Clone, Serialize)]
pub struct ExpireResult {
    /// 本次过期的共享数
    pub expired_count: u64,
}

// =====================================================
// 服务实现
// =====================================================

/// 客户团队协作与共享服务
pub struct CustomerTeamShareService {
    db: Arc<DatabaseConnection>,
}

impl CustomerTeamShareService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // =====================================================
    // 18.4-D2：团队协作
    // =====================================================

    /// 添加团队成员
    /// 业务规则：1. 客户必须存在；2. 被添加的用户必须存在且活跃；3. 同一客户不能重复添加同一用户（唯一约束）；4. primary 角色唯一：若已存在 primary，本次添加必须为 member/assistant；5. 操作人必须为客户负责人或拥有团队管理权限
    pub async fn add_team_member(
        &self,
        req: AddTeamMemberRequest,
        operator_id: i32,
    ) -> Result<TeamMemberDto, AppError> {
        // 1. 校验客户存在
        let customer = customer::Entity::find_by_id(req.customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", req.customer_id)))?;

        // 2. 校验被添加用户存在且活跃
        let member_user = user::Entity::find_by_id(req.user_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::validation(format!("用户 {} 不存在", req.user_id)))?;
        if !member_user.is_active {
            return Err(AppError::validation(format!(
                "用户 {} 已停用，无法加入团队",
                req.user_id
            )));
        }

        // 3. 校验角色合法性
        let team_role = req
            .team_role
            .unwrap_or_else(|| team_model::TEAM_ROLE_MEMBER.to_string());
        Self::validate_team_role(&team_role)?;

        // 4. primary 唯一性校验
        if team_role == team_model::TEAM_ROLE_PRIMARY {
            let existing_primary = customer_team_member::Entity::find()
                .filter(customer_team_member::Column::CustomerId.eq(req.customer_id))
                .filter(customer_team_member::Column::TeamRole.eq(team_model::TEAM_ROLE_PRIMARY))
                .filter(customer_team_member::Column::IsActive.eq(true))
                .count(&*self.db)
                .await?;
            if existing_primary > 0 {
                return Err(AppError::business(
                    "添加团队成员失败：该客户已存在主负责人（primary），一个客户只能有一个主负责人",
                ));
            }
        }

        // 5. 校验操作人权限（客户 owner 或主负责人可操作）
        self.validate_team_management_permission(req.customer_id, &customer, operator_id)
            .await?;

        // 6. 唯一性校验：同一客户不能重复添加同一活跃用户
        let existing = customer_team_member::Entity::find()
            .filter(customer_team_member::Column::CustomerId.eq(req.customer_id))
            .filter(customer_team_member::Column::UserId.eq(req.user_id))
            .filter(customer_team_member::Column::IsActive.eq(true))
            .one(&*self.db)
            .await?;
        if existing.is_some() {
            return Err(AppError::business(format!(
                "添加团队成员失败：用户 {} 已是客户 {} 的团队成员",
                req.user_id, req.customer_id
            )));
        }

        let now = Utc::now();
        let member = customer_team_member::ActiveModel {
            id: Default::default(),
            customer_id: Set(req.customer_id),
            user_id: Set(req.user_id),
            user_name: Set(Some(member_user.username.clone())),
            team_role: Set(team_role),
            is_active: Set(true),
            joined_at: Set(now),
            left_at: Set(None),
            notes: Set(req.notes),
            created_by: Set(Some(operator_id)),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&*self.db)
        .await?;

        info!(
            "用户 {} 添加用户 {} 为客户 {} 的团队成员（角色={}）",
            operator_id, req.user_id, req.customer_id, member.team_role
        );

        Ok(member.into())
    }

    /// 移除团队成员（软删除：设置 left_at + is_active=false）
    pub async fn remove_team_member(
        &self,
        member_id: i32,
        operator_id: i32,
    ) -> Result<TeamMemberDto, AppError> {
        let member = customer_team_member::Entity::find_by_id(member_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("团队成员记录 {} 不存在", member_id)))?;

        if !member.is_active {
            return Err(AppError::business("移除失败：该成员已不在团队中"));
        }

        // 校验操作人权限
        let customer = customer::Entity::find_by_id(member.customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", member.customer_id)))?;
        self.validate_team_management_permission(member.customer_id, &customer, operator_id)
            .await?;

        // primary 不可移除（需先转移主负责人）
        if member.team_role == team_model::TEAM_ROLE_PRIMARY {
            return Err(AppError::business(
                "移除失败：主负责人（primary）不可直接移除，请先变更主负责人",
            ));
        }

        let now = Utc::now();
        let mut active: customer_team_member::ActiveModel = member.into();
        active.is_active = Set(false);
        active.left_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;

        info!(
            "用户 {} 移除团队成员 {}（记录 ID={}）",
            operator_id, updated.user_id, updated.id
        );

        Ok(updated.into())
    }

    /// 列出客户的活跃团队成员
    pub async fn list_team_members(
        &self,
        customer_id: i32,
    ) -> Result<Vec<TeamMemberDto>, AppError> {
        let members = customer_team_member::Entity::find()
            .filter(customer_team_member::Column::CustomerId.eq(customer_id))
            .filter(customer_team_member::Column::IsActive.eq(true))
            .order_by(customer_team_member::Column::JoinedAt, sea_orm::Order::Asc)
            .all(&*self.db)
            .await?;

        Ok(members.into_iter().map(Into::into).collect())
    }

    /// 列出用户参与的客户团队
    pub async fn list_user_teams(
        &self,
        user_id: i32,
        active_only: bool,
    ) -> Result<Vec<TeamMemberDto>, AppError> {
        let mut q = customer_team_member::Entity::find()
            .filter(customer_team_member::Column::UserId.eq(user_id));

        if active_only {
            q = q.filter(customer_team_member::Column::IsActive.eq(true));
        }

        let members = q
            .order_by(customer_team_member::Column::JoinedAt, sea_orm::Order::Desc)
            .all(&*self.db)
            .await?;

        Ok(members.into_iter().map(Into::into).collect())
    }

    /// 校验用户是否为客户的活跃团队成员（返回 Some(team_role) 表示是团队成员，None 表示不是）
    pub async fn is_team_member(
        &self,
        customer_id: i32,
        user_id: i32,
    ) -> Result<Option<String>, AppError> {
        let member = customer_team_member::Entity::find()
            .filter(customer_team_member::Column::CustomerId.eq(customer_id))
            .filter(customer_team_member::Column::UserId.eq(user_id))
            .filter(customer_team_member::Column::IsActive.eq(true))
            .one(&*self.db)
            .await?;

        Ok(member.map(|m| m.team_role))
    }

    /// 校验团队角色合法性
    fn validate_team_role(role: &str) -> Result<(), AppError> {
        match role {
            team_model::TEAM_ROLE_PRIMARY
            | team_model::TEAM_ROLE_MEMBER
            | team_model::TEAM_ROLE_ASSISTANT => Ok(()),
            _ => Err(AppError::validation(format!(
                "无效的团队角色：{}，必须是 primary/member/assistant",
                role
            ))),
        }
    }

    /// 校验团队管理权限（规则：客户 owner、客户的 primary 团队成员、或共享权限为 full 的用户可管理团队）
    async fn validate_team_management_permission(
        &self,
        customer_id: i32,
        customer: &customer::Model,
        operator_id: i32,
    ) -> Result<(), AppError> {
        // 客户 owner 拥有管理权限
        if customer.owner_id == operator_id {
            return Ok(());
        }

        // primary 团队成员拥有管理权限
        let operator_member = customer_team_member::Entity::find()
            .filter(customer_team_member::Column::CustomerId.eq(customer_id))
            .filter(customer_team_member::Column::UserId.eq(operator_id))
            .filter(customer_team_member::Column::IsActive.eq(true))
            .one(&*self.db)
            .await?;
        if let Some(m) = &operator_member {
            if m.team_role == team_model::TEAM_ROLE_PRIMARY {
                return Ok(());
            }
        }

        // full 共享权限用户拥有管理权限
        let full_share = customer_share::Entity::find()
            .filter(customer_share::Column::CustomerId.eq(customer_id))
            .filter(customer_share::Column::SharedToUserId.eq(operator_id))
            .filter(customer_share::Column::Permission.eq(share_model::SHARE_PERMISSION_FULL))
            .filter(customer_share::Column::Status.eq(share_model::SHARE_STATUS_ACTIVE))
            .one(&*self.db)
            .await?;
        if full_share.is_some() {
            return Ok(());
        }

        Err(AppError::permission_denied(format!(
            "用户 {} 无权管理客户 {} 的团队（仅 owner/primary/full 共享权限可操作）",
            operator_id, customer_id
        )))
    }

    // =====================================================
    // 18.4-D3：数据共享时效
    // =====================================================

    /// 共享客户给其他用户（带时效和权限）
    /// 业务规则：1. 客户必须存在；2. 被共享方用户必须存在且活跃；3. 不能共享给自己；4. 操作人必须是客户 owner / primary / full 共享权限；5. 同一客户不能对同一用户重复共享（active 状态）；6. duration_days 为 None 时永久共享（建议设置时效）
    pub async fn share_customer(
        &self,
        req: ShareCustomerRequest,
        operator_id: i32,
    ) -> Result<CustomerShareDto, AppError> {
        // 1. 校验客户存在
        let customer = customer::Entity::find_by_id(req.customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", req.customer_id)))?;

        // 2. 校验被共享方用户存在且活跃
        let to_user = user::Entity::find_by_id(req.shared_to_user_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::validation(format!("被共享方用户 {} 不存在", req.shared_to_user_id))
            })?;
        if !to_user.is_active {
            return Err(AppError::validation(format!(
                "被共享方用户 {} 已停用，无法共享",
                req.shared_to_user_id
            )));
        }

        // 3. 不能共享给自己
        if req.shared_to_user_id == operator_id {
            return Err(AppError::validation("共享失败：不能共享给自己"));
        }

        // 4. 校验操作人权限
        self.validate_share_permission(req.customer_id, &customer, operator_id)
            .await?;

        // 5. 校验权限类型合法性
        let permission = req
            .permission
            .unwrap_or_else(|| share_model::SHARE_PERMISSION_VIEW.to_string());
        Self::validate_share_permission_type(&permission)?;

        // 6. 唯一性校验：同一客户不能对同一用户重复 active 共享
        let existing_active = customer_share::Entity::find()
            .filter(customer_share::Column::CustomerId.eq(req.customer_id))
            .filter(customer_share::Column::SharedToUserId.eq(req.shared_to_user_id))
            .filter(customer_share::Column::Status.eq(share_model::SHARE_STATUS_ACTIVE))
            .one(&*self.db)
            .await?;
        if existing_active.is_some() {
            return Err(AppError::business(format!(
                "共享失败：客户 {} 已共享给用户 {} 且状态为 active，请先撤销原共享",
                req.customer_id, req.shared_to_user_id
            )));
        }

        // 7. 计算过期时间
        let now = Utc::now();
        let expire_at = req
            .duration_days
            .map(|days| now + Duration::days(days as i64));

        // 8. 查询操作人姓名
        let operator = user::Entity::find_by_id(operator_id).one(&*self.db).await?;
        let operator_name = operator
            .map(|u| u.username)
            .unwrap_or_else(|| format!("用户{}", operator_id));

        let share = customer_share::ActiveModel {
            id: Default::default(),
            customer_id: Set(req.customer_id),
            shared_by_user_id: Set(operator_id),
            shared_by_user_name: Set(Some(operator_name)),
            shared_to_user_id: Set(req.shared_to_user_id),
            shared_to_user_name: Set(Some(to_user.username.clone())),
            permission: Set(permission),
            status: Set(share_model::SHARE_STATUS_ACTIVE.to_string()),
            shared_at: Set(now),
            expire_at: Set(expire_at),
            revoked_at: Set(None),
            revoked_by: Set(None),
            revoke_reason: Set(None),
            share_reason: Set(req.share_reason),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&*self.db)
        .await?;

        info!(
            "用户 {} 共享客户 {} 给用户 {}（权限={}，过期={:?}）",
            operator_id, req.customer_id, req.shared_to_user_id, share.permission, share.expire_at
        );

        Ok(share.into())
    }

    /// 撤销共享（主动收回权限）（业务规则：1. 共享记录必须存在；2. 共享状态必须为 active；3. 操作人必须是共享方或拥有 full 权限）
    pub async fn revoke_share(
        &self,
        req: RevokeShareRequest,
        operator_id: i32,
    ) -> Result<CustomerShareDto, AppError> {
        let share = customer_share::Entity::find_by_id(req.share_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("共享记录 {} 不存在", req.share_id)))?;

        if share.status != share_model::SHARE_STATUS_ACTIVE {
            return Err(AppError::business(format!(
                "撤销失败：共享记录当前状态为 {}，非 active",
                share.status
            )));
        }

        // 权限校验：共享方本人、客户 owner、或 full 共享权限可撤销
        if share.shared_by_user_id != operator_id {
            let customer = customer::Entity::find_by_id(share.customer_id)
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", share.customer_id)))?;
            if customer.owner_id != operator_id {
                // 检查是否有 full 共享权限
                let full_share = customer_share::Entity::find()
                    .filter(customer_share::Column::CustomerId.eq(share.customer_id))
                    .filter(customer_share::Column::SharedToUserId.eq(operator_id))
                    .filter(
                        customer_share::Column::Permission.eq(share_model::SHARE_PERMISSION_FULL),
                    )
                    .filter(customer_share::Column::Status.eq(share_model::SHARE_STATUS_ACTIVE))
                    .one(&*self.db)
                    .await?;
                if full_share.is_none() {
                    return Err(AppError::permission_denied(format!(
                        "用户 {} 无权撤销共享 {}（仅共享方/客户 owner/full 权限可操作）",
                        operator_id, req.share_id
                    )));
                }
            }
        }

        let now = Utc::now();
        let mut active: customer_share::ActiveModel = share.into();
        active.status = Set(share_model::SHARE_STATUS_REVOKED.to_string());
        active.revoked_at = Set(Some(now));
        active.revoked_by = Set(Some(operator_id));
        active.revoke_reason = Set(req.revoke_reason);
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;

        info!(
            "用户 {} 撤销共享记录 {}（客户 {} 共享给用户 {}）",
            operator_id, updated.id, updated.customer_id, updated.shared_to_user_id
        );

        Ok(updated.into())
    }

    /// 列出客户的共享记录
    pub async fn list_customer_shares(
        &self,
        customer_id: i32,
        status: Option<String>,
    ) -> Result<Vec<CustomerShareDto>, AppError> {
        let mut q = customer_share::Entity::find()
            .filter(customer_share::Column::CustomerId.eq(customer_id));

        if let Some(s) = status {
            q = q.filter(customer_share::Column::Status.eq(s));
        }

        let shares = q
            .order_by(customer_share::Column::SharedAt, sea_orm::Order::Desc)
            .all(&*self.db)
            .await?;

        Ok(shares.into_iter().map(Into::into).collect())
    }

    /// 列出用户收到的共享
    pub async fn list_user_shares(
        &self,
        user_id: i32,
        active_only: bool,
    ) -> Result<Vec<CustomerShareDto>, AppError> {
        let mut q = customer_share::Entity::find()
            .filter(customer_share::Column::SharedToUserId.eq(user_id));

        if active_only {
            q = q.filter(customer_share::Column::Status.eq(share_model::SHARE_STATUS_ACTIVE));
        }

        let shares = q
            .order_by(customer_share::Column::SharedAt, sea_orm::Order::Desc)
            .all(&*self.db)
            .await?;

        Ok(shares.into_iter().map(Into::into).collect())
    }

    /// 校验用户的共享权限（返回 Some(permission) 表示有共享权限，None 表示无；自动过期检查：若 expire_at < now 则视为无权限）
    pub async fn check_share_permission(
        &self,
        customer_id: i32,
        user_id: i32,
    ) -> Result<Option<String>, AppError> {
        let share = customer_share::Entity::find()
            .filter(customer_share::Column::CustomerId.eq(customer_id))
            .filter(customer_share::Column::SharedToUserId.eq(user_id))
            .filter(customer_share::Column::Status.eq(share_model::SHARE_STATUS_ACTIVE))
            .one(&*self.db)
            .await?;

        if let Some(s) = share {
            // 检查是否已过期
            if let Some(expire_at) = s.expire_at {
                if Utc::now() > expire_at {
                    return Ok(None);
                }
            }
            return Ok(Some(s.permission));
        }

        Ok(None)
    }

    /// 过期超期共享（定时清理任务调用）（将所有 status=active 且 expire_at < now 的共享标记为 expired）
    pub async fn expire_overdue_shares(&self) -> Result<ExpireResult, AppError> {
        let now = Utc::now();

        // 查询所有需要过期的共享
        let overdue_shares = customer_share::Entity::find()
            .filter(customer_share::Column::Status.eq(share_model::SHARE_STATUS_ACTIVE))
            .filter(customer_share::Column::ExpireAt.is_not_null())
            .filter(customer_share::Column::ExpireAt.lt(now))
            .all(&*self.db)
            .await?;

        let count = overdue_shares.len() as u64;
        if count == 0 {
            return Ok(ExpireResult { expired_count: 0 });
        }

        // 批量更新为 expired
        let txn = (*self.db).begin().await?;
        for share in overdue_shares {
            let mut active: customer_share::ActiveModel = share.into();
            active.status = Set(share_model::SHARE_STATUS_EXPIRED.to_string());
            active.updated_at = Set(now);
            active.update(&txn).await?;
        }
        txn.commit().await?;

        info!("自动过期清理：本次过期 {} 条共享记录", count);

        Ok(ExpireResult {
            expired_count: count,
        })
    }

    /// 校验共享权限类型合法性
    fn validate_share_permission_type(permission: &str) -> Result<(), AppError> {
        match permission {
            share_model::SHARE_PERMISSION_VIEW
            | share_model::SHARE_PERMISSION_EDIT
            | share_model::SHARE_PERMISSION_FULL => Ok(()),
            _ => Err(AppError::validation(format!(
                "无效的共享权限：{}，必须是 view/edit/full",
                permission
            ))),
        }
    }

    /// 校验共享操作权限（规则：客户 owner、primary 团队成员、或 full 共享权限可共享）
    async fn validate_share_permission(
        &self,
        customer_id: i32,
        customer: &customer::Model,
        operator_id: i32,
    ) -> Result<(), AppError> {
        // 客户 owner 拥有共享权限
        if customer.owner_id == operator_id {
            return Ok(());
        }

        // primary 团队成员拥有共享权限
        let operator_member = customer_team_member::Entity::find()
            .filter(customer_team_member::Column::CustomerId.eq(customer_id))
            .filter(customer_team_member::Column::UserId.eq(operator_id))
            .filter(customer_team_member::Column::IsActive.eq(true))
            .one(&*self.db)
            .await?;
        if let Some(m) = &operator_member {
            if m.team_role == team_model::TEAM_ROLE_PRIMARY {
                return Ok(());
            }
        }

        // full 共享权限用户可继续共享
        let full_share = customer_share::Entity::find()
            .filter(customer_share::Column::CustomerId.eq(customer_id))
            .filter(customer_share::Column::SharedToUserId.eq(operator_id))
            .filter(customer_share::Column::Permission.eq(share_model::SHARE_PERMISSION_FULL))
            .filter(customer_share::Column::Status.eq(share_model::SHARE_STATUS_ACTIVE))
            .one(&*self.db)
            .await?;
        if full_share.is_some() {
            return Ok(());
        }

        Err(AppError::permission_denied(format!(
            "用户 {} 无权共享客户 {}（仅 owner/primary/full 共享权限可操作）",
            operator_id, customer_id
        )))
    }
}
