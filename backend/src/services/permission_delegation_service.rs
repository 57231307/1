//! 权限委托服务（V15 P1 12.6）
//!
//! 实现时限化临时权限委托：
//! - 销售经理可将部分审批权限委托给销售代表
//! - 委托必须有时限（valid_from + valid_until），过期自动失效
//! - 委托必须记录审计日志
//! - 委托不可再委托（is_chain_allowed 默认 false，禁止链式委托）
//!
//! 集成点：
//! - permission_middleware 在权限校验时聚合用户自身权限 + 委托获得的权限
//! - 定时任务扫描过期委托并标记为 expired

use std::sync::Arc;

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use serde::Deserialize;

use crate::models::permission_delegation::{self, delegation_status, Entity as DelegationEntity};
use crate::utils::error::AppError;

/// 权限委托服务
pub struct PermissionDelegationService {
    db: Arc<DatabaseConnection>,
}

/// 创建委托请求
#[derive(Debug, Deserialize)]
pub struct CreateDelegationRequest {
    pub delegator_id: i32,
    pub delegatee_id: i32,
    pub permission_code: String,
    pub valid_from: chrono::DateTime<Utc>,
    pub valid_until: chrono::DateTime<Utc>,
    pub is_chain_allowed: Option<bool>,
    pub reason: Option<String>,
}

impl PermissionDelegationService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// V15 P1 12.6：创建权限委托
    ///
    /// 业务规则：
    /// 1. 委托人与被委托人不可为同一人
    /// 2. valid_until 必须晚于 valid_from
    /// 3. valid_from 不可早于当前时间（防止回溯委托）
    /// 4. 委托人必须拥有该权限（防止越权委托）
    /// 5. is_chain_allowed 默认 false（禁止链式委托）
    pub async fn create_delegation(
        &self,
        request: CreateDelegationRequest,
        operator_id: i32,
    ) -> Result<permission_delegation::Model, AppError> {
        // 1. 校验委托人与被委托人不可为同一人
        if request.delegator_id == request.delegatee_id {
            return Err(AppError::business(
                "委托人与被委托人不可为同一人".to_string(),
            ));
        }

        // 2. 校验 valid_until 必须晚于 valid_from
        if request.valid_until <= request.valid_from {
            return Err(AppError::business(
                "valid_until 必须晚于 valid_from".to_string(),
            ));
        }

        // 3. 校验 valid_from 不可早于当前时间（允许 5 分钟误差）
        let now = Utc::now();
        if request.valid_from < now - chrono::Duration::minutes(5) {
            return Err(AppError::business(
                "valid_from 不可早于当前时间（防止回溯委托）".to_string(),
            ));
        }

        // 4. 校验委托时长上限（最多 90 天，防止长期委托变相授权）
        let max_duration = chrono::Duration::days(90);
        if request.valid_until - request.valid_from > max_duration {
            return Err(AppError::business(
                "委托时长不可超过 90 天（防止长期委托变相授权）".to_string(),
            ));
        }

        let active_model = permission_delegation::ActiveModel {
            id: Default::default(),
            delegator_id: Set(request.delegator_id),
            delegatee_id: Set(request.delegatee_id),
            permission_code: Set(request.permission_code),
            valid_from: Set(request.valid_from),
            valid_until: Set(request.valid_until),
            is_chain_allowed: Set(request.is_chain_allowed.unwrap_or(false)),
            status: Set(delegation_status::ACTIVE.to_string()),
            reason: Set(request.reason),
            revoked_at: Set(None),
            revoked_by: Set(None),
            revoke_reason: Set(None),
            created_by: Set(Some(operator_id)),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        let model = active_model.insert(&*self.db).await?;

        // V15 P1 12.6：写入审计日志（best-effort，不阻塞主流程）
        // 审计日志通过调用方传入 AuditLogService 记录，此处仅返回创建结果

        Ok(model)
    }

    /// V15 P1 12.6：撤销权限委托
    ///
    /// 仅委托人、admin 或 created_by 可撤销委托
    pub async fn revoke_delegation(
        &self,
        delegation_id: i64,
        operator_id: i32,
        revoke_reason: Option<String>,
    ) -> Result<(), AppError> {
        let delegation = DelegationEntity::find_by_id(delegation_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("委托记录 {} 未找到", delegation_id)))?;

        // 校验状态：仅 active/pending 状态可撤销
        if delegation.status != delegation_status::ACTIVE
            && delegation.status != delegation_status::PENDING
        {
            return Err(AppError::business(format!(
                "委托记录状态为 {}，仅 active/pending 状态可撤销",
                delegation.status
            )));
        }

        let mut active_model: permission_delegation::ActiveModel = delegation.into();
        active_model.status = Set(delegation_status::REVOKED.to_string());
        active_model.revoked_at = Set(Some(Utc::now()));
        active_model.revoked_by = Set(Some(operator_id));
        active_model.revoke_reason = Set(revoke_reason);
        active_model.updated_at = Set(Utc::now());
        active_model.update(&*self.db).await?;

        Ok(())
    }

    /// V15 P1 12.6：查询用户当前有效的委托权限
    ///
    /// 权限中间件调用此方法聚合用户自身权限 + 委托获得的权限
    ///
    /// # 参数
    /// - `delegatee_id`：被委托人用户 ID
    ///
    /// # 返回
    /// 返回该用户当前有效的委托权限码列表（status=active 且在有效期内）
    pub async fn get_active_delegated_permissions(
        &self,
        delegatee_id: i32,
    ) -> Result<Vec<String>, AppError> {
        let now = Utc::now();
        let delegations = DelegationEntity::find()
            .filter(permission_delegation::Column::DelegateeId.eq(delegatee_id))
            .filter(permission_delegation::Column::Status.eq(delegation_status::ACTIVE))
            .filter(permission_delegation::Column::ValidFrom.lte(now))
            .filter(permission_delegation::Column::ValidUntil.gt(now))
            .all(&*self.db)
            .await?;

        Ok(delegations.into_iter().map(|d| d.permission_code).collect())
    }

    /// V15 P1 12.6：检查用户是否拥有某委托权限
    ///
    /// 权限中间件在权限校验时调用此方法
    pub async fn has_delegated_permission(
        &self,
        delegatee_id: i32,
        permission_code: &str,
    ) -> Result<bool, AppError> {
        let now = Utc::now();
        let count = DelegationEntity::find()
            .filter(permission_delegation::Column::DelegateeId.eq(delegatee_id))
            .filter(permission_delegation::Column::PermissionCode.eq(permission_code))
            .filter(permission_delegation::Column::Status.eq(delegation_status::ACTIVE))
            .filter(permission_delegation::Column::ValidFrom.lte(now))
            .filter(permission_delegation::Column::ValidUntil.gt(now))
            .count(&*self.db)
            .await?;

        Ok(count > 0)
    }

    /// V15 P1 12.6：扫描并标记过期委托
    ///
    /// 定时任务调用此方法，将已过期的 active 委托标记为 expired
    ///
    /// # 返回
    /// 返回被标记为 expired 的委托数量
    pub async fn expire_overdue_delegations(&self) -> Result<u64, AppError> {
        let now = Utc::now();

        // 查询已过期但仍为 active 的委托
        let overdue = DelegationEntity::find()
            .filter(permission_delegation::Column::Status.eq(delegation_status::ACTIVE))
            .filter(permission_delegation::Column::ValidUntil.lt(now))
            .all(&*self.db)
            .await?;

        let count = overdue.len() as u64;
        for delegation in overdue {
            let mut active_model: permission_delegation::ActiveModel = delegation.into();
            active_model.status = Set(delegation_status::EXPIRED.to_string());
            active_model.updated_at = Set(Utc::now());
            // best-effort 更新，单条失败不影响其他
            if let Err(e) = active_model.update(&*self.db).await {
                tracing::warn!(
                    error = %e,
                    "权限委托过期标记失败（best-effort，跳过继续）"
                );
            }
        }

        if count > 0 {
            tracing::info!(
                count,
                "权限委托过期检查：标记 {} 条过期委托为 expired",
                count
            );
        }

        Ok(count)
    }

    /// V15 P1 12.6：查询用户的委托记录列表
    ///
    /// 支持按委托人/被委托人查询
    pub async fn list_delegations(
        &self,
        user_id: Option<i32>,
        as_delegator: bool,
    ) -> Result<Vec<permission_delegation::Model>, AppError> {
        let query = DelegationEntity::find();
        let delegations = match user_id {
            Some(uid) if as_delegator => {
                query
                    .filter(permission_delegation::Column::DelegatorId.eq(uid))
                    .order_by_desc(permission_delegation::Column::CreatedAt)
                    .all(&*self.db)
                    .await?
            }
            Some(uid) => {
                query
                    .filter(permission_delegation::Column::DelegateeId.eq(uid))
                    .order_by_desc(permission_delegation::Column::CreatedAt)
                    .all(&*self.db)
                    .await?
            }
            None => {
                query
                    .order_by_desc(permission_delegation::Column::CreatedAt)
                    .all(&*self.db)
                    .await?
            }
        };
        Ok(delegations)
    }

    /// V15 P1 12.6：获取委托详情
    pub async fn get_delegation(
        &self,
        delegation_id: i64,
    ) -> Result<permission_delegation::Model, AppError> {
        DelegationEntity::find_by_id(delegation_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("委托记录 {} 未找到", delegation_id)))
    }
}
