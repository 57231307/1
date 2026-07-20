//! CRM 分配服务（crm/assign）
//!
//! v10 P1 批次 140 修复：原模块为占位注释，根据规则 0（保留的功能扩展空间视为未实现功能）真实实现。
//! 提供客户/线索的进阶分配能力：
//! - 自动分配（轮询）：将未分配线索按 round-robin 策略均衡分配给销售团队
//! - 转移分配：将线索从当前归属人转移给新归属人，记录转移原因（带审计）
//!
//! 注：基础单条/批量分配已由 `crm_assignment_handler` + `assignment_history_service` 提供。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::models::{crm_lead, user};
// 批次 236 v13 P1-1：线索状态常量接入（规则 0）
use crate::models::status::crm_lead as lead_status;
use crate::services::assignment_history_service::{
    AssignmentHistoryService, CreateAssignmentHistoryRequest,
};
use crate::utils::error::AppError;

/// 自动分配请求
#[derive(Debug, Clone, Deserialize)]
pub struct AutoAssignRequest {
    /// 参与轮询的销售用户 ID 列表
    pub assignee_user_ids: Vec<i32>,
    /// 限制本次自动分配的线索数量（缺失时默认 100，上限 1000 防 DoS）
    pub limit: Option<u64>,
}

/// 抢单请求（单个销售主动认领）
#[derive(Debug, Clone, Deserialize)]
pub struct ClaimLeadRequest {
    /// 主动认领的销售用户 ID（通常由 handler 从 auth.user_id 注入，但保留参数以支持代他人认领）
    pub user_id: i32,
    /// 可选：指定认领的线索 ID（缺失时自动选取最早入库的未分配线索）
    pub lead_id: Option<i32>,
}

/// 自动分配结果
#[derive(Debug, Clone, Serialize)]
pub struct AutoAssignResult {
    /// 本次分配的线索总数
    pub assigned_count: u64,
    /// 参与轮询的销售人数
    pub assignee_count: usize,
    /// 各销售分配到的线索数
    pub distribution: Vec<AssigneeDistribution>,
}

/// 单个销售的分配分布
#[derive(Debug, Clone, Serialize)]
pub struct AssigneeDistribution {
    pub user_id: i32,
    pub username: String,
    pub assigned_count: u64,
}

/// 转移分配请求
#[derive(Debug, Clone, Deserialize)]
pub struct TransferLeadRequest {
    /// 线索 ID
    pub lead_id: i32,
    /// 新归属人用户 ID
    pub to_user_id: i32,
    /// 转移原因（必填，写入 assignment_history.reason）
    pub reason: String,
    /// 备注（可选）
    pub notes: Option<String>,
}

/// 转移分配结果
#[derive(Debug, Clone, Serialize)]
pub struct TransferLeadResult {
    pub lead_id: i32,
    pub lead_no: String,
    pub from_user_id: i32,
    pub from_user_name: String,
    pub to_user_id: i32,
    pub to_user_name: String,
    pub transferred_at: chrono::DateTime<chrono::Utc>,
}

/// CRM 分配服务
pub struct CrmAssignService {
    db: Arc<DatabaseConnection>,
    history_service: AssignmentHistoryService,
}

impl CrmAssignService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        let history_service = AssignmentHistoryService::new(db.clone());
        Self {
            db,
            history_service,
        }
    }

    /// 自动分配线索（轮询策略）
    ///
    /// 策略：
    /// 1. 查询所有 `lead_status = 'new'` 的未分配线索（按 id ASC 排序，limit 限制）
    /// 2. 查询 `assignee_user_ids` 中所有 is_active=true 的有效用户
    /// 3. 按 round-robin 轮询：第 i 条线索分配给 `assignees[i % assignees.len()]`
    /// 4. 事务内逐条更新线索归属人 + 写入分配历史
    ///
    /// 失败语义：单条线索分配失败时整体事务回滚（保证一致性）
    pub async fn auto_assign_leads(
        &self,
        req: AutoAssignRequest,
        operator_id: i32,
        operator_name: &str,
    ) -> Result<AutoAssignResult, AppError> {
        if req.assignee_user_ids.is_empty() {
            return Err(AppError::validation(
                "自动分配失败：参与轮询的销售用户列表不能为空",
            ));
        }

        let limit = req.limit.unwrap_or(100).clamp(1, 1000);

        let assignees = self.fetch_active_assignees(&req.assignee_user_ids).await?;
        if assignees.is_empty() {
            return Err(AppError::validation(
                "自动分配失败：指定的销售用户均不存在或已停用",
            ));
        }

        let pending_leads = self.fetch_pending_leads(limit).await?;
        if pending_leads.is_empty() {
            info!("自动分配：无 lead_status='new' 的待分配线索");
            return Ok(Self::build_empty_result(&assignees));
        }

        let txn = (*self.db).begin().await?;
        let (assigned_count, distribution_map) = Self::assign_leads_in_txn(
            &txn,
            &self.history_service,
            &pending_leads,
            &assignees,
            operator_id,
            operator_name,
        )
        .await?;
        txn.commit().await?;

        info!(
            "用户 {} 自动分配线索完成：分配 {} 条，参与销售 {} 人",
            operator_id,
            assigned_count,
            assignees.len()
        );

        Ok(Self::build_assign_result(assigned_count, &assignees, distribution_map))
    }

    async fn fetch_active_assignees(&self, user_ids: &[i32]) -> Result<Vec<user::Model>, AppError> {
        let assignees: Vec<user::Model> = user::Entity::find()
            .filter(user::Column::Id.is_in(user_ids.to_vec()))
            .filter(user::Column::IsActive.eq(true))
            .all(&*self.db)
            .await?;
        Ok(assignees)
    }

    async fn fetch_pending_leads(&self, limit: u64) -> Result<Vec<crm_lead::Model>, AppError> {
        let pending_leads: Vec<crm_lead::Model> = crm_lead::Entity::find()
            .filter(crm_lead::Column::LeadStatus.eq(lead_status::NEW))
            .order_by(crm_lead::Column::Id, sea_orm::Order::Asc)
            .limit(limit)
            .all(&*self.db)
            .await?;
        Ok(pending_leads)
    }

    fn build_empty_result(assignees: &[user::Model]) -> AutoAssignResult {
        AutoAssignResult {
            assigned_count: 0,
            assignee_count: assignees.len(),
            distribution: assignees
                .iter()
                .map(|u| AssigneeDistribution {
                    user_id: u.id,
                    username: u.username.clone(),
                    assigned_count: 0,
                })
                .collect(),
        }
    }

    async fn assign_leads_in_txn(
        txn: &sea_orm::DatabaseTransaction,
        history_service: &AssignmentHistoryService,
        pending_leads: &[crm_lead::Model],
        assignees: &[user::Model],
        operator_id: i32,
        operator_name: &str,
    ) -> Result<(u64, std::collections::HashMap<i32, (String, u64)>), AppError> {
        let mut distribution_map: std::collections::HashMap<i32, (String, u64)> =
            std::collections::HashMap::new();
        for a in assignees {
            distribution_map.insert(a.id, (a.username.clone(), 0));
        }

        let mut assigned_count: u64 = 0;

        for (idx, lead) in pending_leads.iter().enumerate() {
            let assignee = &assignees[idx % assignees.len()];
            Self::assign_single_lead(txn, history_service, lead, assignee, operator_id, operator_name, idx)
                .await?;

            if let Some(entry) = distribution_map.get_mut(&assignee.id) {
                entry.1 += 1;
            }
            assigned_count += 1;
        }

        Ok((assigned_count, distribution_map))
    }

    async fn assign_single_lead(
        txn: &sea_orm::DatabaseTransaction,
        history_service: &AssignmentHistoryService,
        lead: &crm_lead::Model,
        assignee: &user::Model,
        operator_id: i32,
        operator_name: &str,
        idx: usize,
    ) -> Result<(), AppError> {
        let from_user_id = lead.owner_id;
        let from_user_name = lead.owner_name.clone();

        let mut active: crm_lead::ActiveModel = lead.clone().into();
        active.owner_id = Set(assignee.id);
        active.owner_name = Set(assignee.username.clone());
        active.lead_status = Set(Some("assigned".to_string()));
        active.updated_at = Set(Some(chrono::Utc::now()));
        active.updated_by = Set(Some(operator_id));
        let updated = active.update(txn).await?;

        history_service
            .create_with_txn(
                txn,
                operator_id,
                operator_name,
                CreateAssignmentHistoryRequest {
                    lead_id: updated.id,
                    lead_no: updated.lead_no.clone(),
                    company_name: updated.company_name.clone(),
                    from_user_id: Some(from_user_id),
                    from_user_name: Some(from_user_name),
                    to_user_id: Some(assignee.id),
                    to_user_name: Some(assignee.username.clone()),
                    action: "ASSIGN".to_string(),
                    reason: Some("auto_assign_round_robin".to_string()),
                    notes: Some(format!("自动轮询分配 #{}", idx + 1)),
                },
            )
            .await?;

        Ok(())
    }

    fn build_assign_result(
        assigned_count: u64,
        assignees: &[user::Model],
        distribution_map: std::collections::HashMap<i32, (String, u64)>,
    ) -> AutoAssignResult {
        AutoAssignResult {
            assigned_count,
            assignee_count: assignees.len(),
            distribution: distribution_map
                .into_iter()
                .map(|(user_id, (username, count))| AssigneeDistribution {
                    user_id,
                    username,
                    assigned_count: count,
                })
                .collect(),
        }
    }

    /// 转移线索归属人（带审计）
    ///
    /// 业务规则：
    /// 1. 线索必须存在且未转化为客户（lead_status != 'converted'）
    /// 2. 新归属人必须是活跃用户（is_active=true）
    /// 3. 不能转移给自己（to_user_id != lead.owner_id）
    /// 4. 记录转移历史（action="TRANSFER"），携带 reason 和 notes 供审计
    pub async fn transfer_lead(
        &self,
        req: TransferLeadRequest,
        operator_id: i32,
        operator_name: &str,
    ) -> Result<TransferLeadResult, AppError> {
        if req.reason.trim().is_empty() {
            return Err(AppError::validation("转移分配失败：转移原因不能为空"));
        }

        // 查询线索
        let lead = crm_lead::Entity::find_by_id(req.lead_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("线索 {} 不存在", req.lead_id)))?;

        if lead.lead_status.as_deref() == Some(lead_status::CONVERTED) {
            return Err(AppError::validation(format!(
                "线索 {} 已转化为客户，无法转移",
                req.lead_id
            )));
        }

        if lead.owner_id == req.to_user_id {
            return Err(AppError::validation(
                format!("转移失败：线索当前归属人已是用户 {}", req.to_user_id),
            ));
        }

        // 校验新归属人存在且活跃
        let new_owner = user::Entity::find_by_id(req.to_user_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::validation(format!("新归属人用户 {} 不存在", req.to_user_id))
            })?;

        if !new_owner.is_active {
            return Err(AppError::validation(format!(
                "新归属人用户 {} 已停用，无法接收线索",
                req.to_user_id
            )));
        }

        let from_user_id = lead.owner_id;
        let from_user_name = lead.owner_name.clone();
        let now = chrono::Utc::now();

        // 事务：更新归属人 + 写入转移历史
        let txn = (*self.db).begin().await?;

        let mut active: crm_lead::ActiveModel = lead.into();
        active.owner_id = Set(new_owner.id);
        active.owner_name = Set(new_owner.username.clone());
        // 转移后状态保持原值不变（不强制改为 assigned），仅更新归属人
        active.updated_at = Set(Some(now));
        active.updated_by = Set(Some(operator_id));
        let updated = active.update(&txn).await?;

        self.history_service
            .create_with_txn(
                &txn,
                operator_id,
                operator_name,
                CreateAssignmentHistoryRequest {
                    lead_id: updated.id,
                    lead_no: updated.lead_no.clone(),
                    company_name: updated.company_name.clone(),
                    from_user_id: Some(from_user_id),
                    from_user_name: Some(from_user_name.clone()),
                    to_user_id: Some(new_owner.id),
                    to_user_name: Some(new_owner.username.clone()),
                    action: "TRANSFER".to_string(),
                    reason: Some(req.reason.clone()),
                    notes: req.notes.clone(),
                },
            )
            .await?;

        txn.commit().await?;

        info!(
            "用户 {} 将线索 {} 从用户 {} 转移给用户 {}",
            operator_id, updated.id, from_user_id, new_owner.id
        );

        Ok(TransferLeadResult {
            lead_id: updated.id,
            lead_no: updated.lead_no,
            from_user_id,
            from_user_name,
            to_user_id: new_owner.id,
            to_user_name: new_owner.username,
            transferred_at: now,
        })
    }

    /// 查询销售用户的当前线索负载（用于自动分配前的预览）
    ///
    /// 返回每个用户的活跃线索数（lead_status != 'converted' 且 lead_status != 'lost'）
    pub async fn list_assignee_workload(
        &self,
        user_ids: &[i32],
    ) -> Result<Vec<AssigneeDistribution>, AppError> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }

        let users: Vec<user::Model> = user::Entity::find()
            .filter(user::Column::Id.is_in(user_ids.to_vec()))
            .filter(user::Column::IsActive.eq(true))
            .all(&*self.db)
            .await?;

        let mut result = Vec::with_capacity(users.len());
        for u in users {
            // 统计该用户当前活跃线索数
            let count = crm_lead::Entity::find()
                .filter(crm_lead::Column::OwnerId.eq(u.id))
                .filter(crm_lead::Column::LeadStatus.is_not_null())
                .filter(
                    crm_lead::Column::LeadStatus
                        .ne(lead_status::CONVERTED)
                        .and(crm_lead::Column::LeadStatus.ne(lead_status::LOST)),
                )
                .count(&*self.db)
                .await?;

            result.push(AssigneeDistribution {
                user_id: u.id,
                username: u.username,
                assigned_count: count,
            });
        }

        // 按当前负载升序排序（负载最少的优先分配）
        result.sort_by_key(|d| d.assigned_count);
        Ok(result)
    }

    /// 抢单模式：单个销售主动认领一条未分配线索
    ///
    /// 业务规则：
    /// 1. 若指定 lead_id，则认领该线索；否则自动选取最早入库的 lead_status='new' 线索
    /// 2. 线索必须处于 'new' 状态（未分配），否则返回 validation 错误
    /// 3. 认领人必须是活跃用户（is_active=true）
    /// 4. 写入分配历史 action="CLAIM"，reason="manual_claim"
    /// 5. 事务保证线索更新与历史记录的原子性
    pub async fn claim_lead(
        &self,
        req: ClaimLeadRequest,
        operator_id: i32,
        operator_name: &str,
    ) -> Result<TransferLeadResult, AppError> {
        // 校验认领人存在且活跃
        let claimer = user::Entity::find_by_id(req.user_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::validation(format!("认领人用户 {} 不存在", req.user_id))
            })?;

        if !claimer.is_active {
            return Err(AppError::validation(format!(
                "认领人用户 {} 已停用，无法认领线索",
                req.user_id
            )));
        }

        // 获取待认领线索
        let lead: crm_lead::Model = if let Some(lead_id) = req.lead_id {
            // 指定线索 ID 认领
            crm_lead::Entity::find_by_id(lead_id)
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::not_found(format!("线索 {} 不存在", lead_id)))?
        } else {
            // 自动选取最早入库的未分配线索（FIFO）
            crm_lead::Entity::find()
                .filter(crm_lead::Column::LeadStatus.eq(lead_status::NEW))
                .order_by(crm_lead::Column::Id, sea_orm::Order::Asc)
                .limit(1)
                .all(&*self.db)
                .await?
                .into_iter()
                .next()
                .ok_or_else(|| {
                    AppError::not_found("无可认领线索：当前没有 lead_status='new' 的未分配线索")
                })?
        };

        // 校验线索状态
        if lead.lead_status.as_deref() != Some(lead_status::NEW) {
            return Err(AppError::validation(format!(
                "线索 {} 当前状态为 {:?}，非 'new' 状态无法认领",
                lead.id, lead.lead_status
            )));
        }

        if lead.owner_id == req.user_id {
            return Err(AppError::validation(
                "认领失败：线索当前归属人已是当前用户",
            ));
        }

        let from_user_id = lead.owner_id;
        let from_user_name = lead.owner_name.clone();
        let now = chrono::Utc::now();

        // 事务：更新线索归属人 + 写入认领历史
        let txn = (*self.db).begin().await?;

        let mut active: crm_lead::ActiveModel = lead.into();
        active.owner_id = Set(claimer.id);
        active.owner_name = Set(claimer.username.clone());
        active.lead_status = Set(Some("assigned".to_string()));
        active.updated_at = Set(Some(now));
        active.updated_by = Set(Some(operator_id));
        let updated = active.update(&txn).await?;

        self.history_service
            .create_with_txn(
                &txn,
                operator_id,
                operator_name,
                CreateAssignmentHistoryRequest {
                    lead_id: updated.id,
                    lead_no: updated.lead_no.clone(),
                    company_name: updated.company_name.clone(),
                    from_user_id: Some(from_user_id),
                    from_user_name: Some(from_user_name.clone()),
                    to_user_id: Some(claimer.id),
                    to_user_name: Some(claimer.username.clone()),
                    action: "CLAIM".to_string(),
                    reason: Some("manual_claim".to_string()),
                    notes: Some(format!("用户 {} 主动认领", operator_name)),
                },
            )
            .await?;

        txn.commit().await?;

        info!(
            "用户 {} 通过抢单模式认领线索 {}，归属人从 {} 变更为 {}",
            operator_id, updated.id, from_user_id, claimer.id
        );

        Ok(TransferLeadResult {
            lead_id: updated.id,
            lead_no: updated.lead_no,
            from_user_id,
            from_user_name,
            to_user_id: claimer.id,
            to_user_name: claimer.username,
            transferred_at: now,
        })
    }
}
