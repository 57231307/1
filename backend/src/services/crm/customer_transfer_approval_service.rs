//! 客户转移审批服务（crm/customer_transfer_approval）
//!
//! V15 P0-S08 修复：实现客户转移的多级审批流
//!
//! 流程：
//! 1. 销售员发起转移申请 → 创建审批单（pending）
//! 2. 销售经理审批：
//!    - 普通客户：经理通过即完成审批，触发实际转移
//!    - 大客户（信用额度 > 阈值）：经理通过后进入总监审批层
//! 3. 总监审批（仅大客户）：
//!    - 通过 → 触发实际转移
//!    - 拒绝 → 审批失败
//! 4. 任意层级拒绝 → 审批失败，不执行转移
//!
//! 关联：审批通过后调用 CrmAssignService::transfer_lead 执行实际转移

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::models::{
    customer_transfer_approval::{self, Entity as TransferApprovalEntity},
    crm_lead::{self, Entity as CrmLeadEntity},
    customer::Entity as CustomerEntity,
};
use crate::models::status::crm_lead as lead_status;
use crate::services::crm::assign::{CrmAssignService, TransferLeadRequest};
use crate::utils::error::AppError;

/// 大客户信用额度阈值（默认 50 万，超过则转移需总监二次审批）
const DEFAULT_LARGE_CUSTOMER_CREDIT_THRESHOLD: i64 = 500_000;

/// 创建转移审批请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTransferApprovalRequest {
    /// 客户/线索 ID
    pub lead_id: i32,
    /// 新归属人用户 ID
    pub to_user_id: i32,
    /// 申请原因（必填）
    pub reason: String,
}

/// 审批操作请求
#[derive(Debug, Clone, Deserialize)]
pub struct ApproveRequest {
    /// 审批单 ID
    pub approval_id: i32,
    /// 审批意见
    pub comment: String,
    /// 是否通过
    pub approved: bool,
}

/// 审批查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct ApprovalQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    /// 按状态过滤
    pub status: Option<String>,
    /// 按申请人过滤
    pub applicant_id: Option<i32>,
    /// 按当前审批人过滤（销售经理 / 总监）
    pub approver_id: Option<i32>,
}

/// 审批结果 DTO
#[derive(Debug, Clone, Serialize)]
pub struct TransferApprovalDto {
    pub id: i32,
    pub approval_no: String,
    pub lead_id: i32,
    pub company_name: Option<String>,
    pub from_user_id: i32,
    pub from_user_name: Option<String>,
    pub to_user_id: i32,
    pub to_user_name: Option<String>,
    pub applicant_id: i32,
    pub reason: String,
    pub is_large_customer: bool,
    pub approval_status: String,
    pub current_level: i32,
    pub max_level: i32,
    pub manager_approver_id: Option<i32>,
    pub manager_comment: Option<String>,
    pub manager_approved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub director_approver_id: Option<i32>,
    pub director_comment: Option<String>,
    pub director_approved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<customer_transfer_approval::Model> for TransferApprovalDto {
    fn from(m: customer_transfer_approval::Model) -> Self {
        Self {
            id: m.id,
            approval_no: m.approval_no,
            lead_id: m.lead_id,
            company_name: m.company_name,
            from_user_id: m.from_user_id,
            from_user_name: m.from_user_name,
            to_user_id: m.to_user_id,
            to_user_name: m.to_user_name,
            applicant_id: m.applicant_id,
            reason: m.reason,
            is_large_customer: m.is_large_customer,
            approval_status: m.approval_status,
            current_level: m.current_level,
            max_level: m.max_level,
            manager_approver_id: m.manager_approver_id,
            manager_comment: m.manager_comment,
            manager_approved_at: m.manager_approved_at,
            director_approver_id: m.director_approver_id,
            director_comment: m.director_comment,
            director_approved_at: m.director_approved_at,
            completed_at: m.completed_at,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

/// 客户转移审批服务
pub struct CustomerTransferApprovalService {
    db: Arc<DatabaseConnection>,
    assign_service: CrmAssignService,
}

impl CustomerTransferApprovalService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        let assign_service = CrmAssignService::new(db.clone());
        Self {
            db,
            assign_service,
        }
    }

    /// 创建转移审批申请
    ///
    /// 业务规则：
    /// 1. 线索必须存在且未转化为客户
    /// 2. 新归属人必须不等于当前归属人
    /// 3. 大客户（关联 customer 信用额度 > 阈值，或客户类型 vip）需总监二次审批
    /// 4. 同一线索不能存在 pending 状态的审批单
    pub async fn create_approval(
        &self,
        req: CreateTransferApprovalRequest,
        applicant_id: i32,
        applicant_name: &str,
    ) -> Result<TransferApprovalDto, AppError> {
        if req.reason.trim().is_empty() {
            return Err(AppError::validation("转移审批申请失败：申请原因不能为空"));
        }

        // 查询线索
        let lead = CrmLeadEntity::find_by_id(req.lead_id)
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
                "转移审批申请失败：新归属人已是当前归属人",
            ));
        }

        // 校验同一线索无 pending 审批
        let existing_pending = TransferApprovalEntity::find()
            .filter(customer_transfer_approval::Column::LeadId.eq(req.lead_id))
            .filter(
                customer_transfer_approval::Column::ApprovalStatus
                    .eq(customer_transfer_approval::STATUS_PENDING),
            )
            .count(&*self.db)
            .await?;
        if existing_pending > 0 {
            return Err(AppError::validation(
                "转移审批申请失败：该线索已存在待审批的转移申请",
            ));
        }

        // 判断是否大客户转移
        let is_large_customer = self.check_large_customer(&lead).await?;

        // 设置最大审批层级：大客户 2 层（经理 + 总监），普通客户 1 层（经理）
        let max_level = if is_large_customer { 2 } else { 1 };

        // 生成审批单号：TA + 时间戳 + lead_id
        let approval_no = format!(
            "TA{}{:06}",
            chrono::Utc::now().format("%Y%m%d%H%M%S"),
            req.lead_id.rem_euclid(1_000_000)
        );

        let now = chrono::Utc::now();
        let approval = customer_transfer_approval::ActiveModel {
            id: Default::default(),
            approval_no: Set(approval_no),
            lead_id: Set(req.lead_id),
            company_name: Set(lead.company_name.clone()),
            from_user_id: Set(lead.owner_id),
            from_user_name: Set(Some(lead.owner_name.clone())),
            to_user_id: Set(req.to_user_id),
            to_user_name: Set(None), // 待审批通过时由 transfer_lead 填充
            applicant_id: Set(applicant_id),
            reason: Set(req.reason),
            is_large_customer: Set(is_large_customer),
            approval_status: Set(customer_transfer_approval::STATUS_PENDING.to_string()),
            current_level: Set(1),
            max_level: Set(max_level),
            manager_approver_id: Set(None),
            manager_comment: Set(None),
            manager_approved_at: Set(None),
            director_approver_id: Set(None),
            director_comment: Set(None),
            director_approved_at: Set(None),
            completed_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&*self.db)
        .await?;

        info!(
            "用户 {}({}) 创建客户转移审批单 {}：线索 {} 从 {} 转移给 {}（大客户={}，max_level={}）",
            applicant_id,
            applicant_name,
            approval.approval_no,
            req.lead_id,
            lead.owner_id,
            req.to_user_id,
            is_large_customer,
            max_level
        );

        Ok(approval.into())
    }

    /// 销售经理审批
    ///
    /// 业务规则：
    /// 1. 审批单必须存在且为 pending 状态
    /// 2. current_level 必须为 1（经理审批层）
    /// 3. 通过：
    ///    - 普通客户（max_level=1）：直接执行转移并标记 completed
    ///    - 大客户（max_level=2）：进入总监审批层 current_level=2
    /// 4. 拒绝：标记 rejected，不执行转移
    pub async fn manager_approve(
        &self,
        req: ApproveRequest,
        manager_id: i32,
        manager_name: &str,
    ) -> Result<TransferApprovalDto, AppError> {
        let approval = self.get_pending_approval(req.approval_id, 1).await?;

        let txn = (*self.db).begin().await?;
        let now = chrono::Utc::now();

        let mut active: customer_transfer_approval::ActiveModel = approval.into();
        active.manager_approver_id = Set(Some(manager_id));
        active.manager_comment = Set(Some(req.comment.clone()));
        active.manager_approved_at = Set(Some(now));
        active.updated_at = Set(now);

        if req.approved {
            // 经理通过
            if active.max_level.clone().unwrap() == 1 {
                // 普通客户：直接执行转移
                let lead_id = active.lead_id.clone().unwrap();
                let to_user_id = active.to_user_id.clone().unwrap();
                let reason = active.reason.clone().unwrap();

                active.approval_status =
                    Set(customer_transfer_approval::STATUS_APPROVED.to_string());
                active.completed_at = Set(Some(now));
                active.to_user_name = Set(Some(format!("用户{}", to_user_id)));
                active.updated_at = Set(now);

                let updated = active.update(&txn).await?;
                // 显式 commit 审批状态变更，再执行实际转移（transfer_lead 内部会自开事务）
                txn.commit().await?;

                self.execute_transfer(lead_id, to_user_id, manager_id, manager_name, &reason)
                    .await?;

                info!(
                    "销售经理 {} 审批通过转移单 {}（普通客户，已完成转移）",
                    manager_id, updated.approval_no
                );
                Ok(updated.into())
            } else {
                // 大客户：进入总监审批层
                active.current_level = Set(2);
                active.updated_at = Set(now);
                let updated = active.update(&txn).await?;
                txn.commit().await?;

                info!(
                    "销售经理 {} 审批通过转移单 {}（大客户，进入总监审批层）",
                    manager_id, updated.approval_no
                );
                Ok(updated.into())
            }
        } else {
            // 经理拒绝
            active.approval_status = Set(customer_transfer_approval::STATUS_REJECTED.to_string());
            active.updated_at = Set(now);
            let updated = active.update(&txn).await?;
            txn.commit().await?;

            info!(
                "销售经理 {} 拒绝转移单 {}，原因：{}",
                manager_id, updated.approval_no, req.comment
            );
            Ok(updated.into())
        }
    }

    /// 总监审批（仅大客户转移需要）
    ///
    /// 业务规则：
    /// 1. 审批单必须存在且为 pending 状态
    /// 2. current_level 必须为 2（总监审批层）
    /// 3. max_level 必须为 2（大客户）
    /// 4. 通过：执行转移并标记 completed
    /// 5. 拒绝：标记 rejected，不执行转移
    pub async fn director_approve(
        &self,
        req: ApproveRequest,
        director_id: i32,
        director_name: &str,
    ) -> Result<TransferApprovalDto, AppError> {
        let approval = self.get_pending_approval(req.approval_id, 2).await?;

        if approval.max_level != 2 {
            return Err(AppError::validation(
                "总监审批失败：该审批单不需要总监审批（非大客户转移）",
            ));
        }

        let txn = (*self.db).begin().await?;
        let now = chrono::Utc::now();

        let mut active: customer_transfer_approval::ActiveModel = approval.into();
        active.director_approver_id = Set(Some(director_id));
        active.director_comment = Set(Some(req.comment.clone()));
        active.director_approved_at = Set(Some(now));
        active.updated_at = Set(now);

        if req.approved {
            // 总监通过：执行转移
            let lead_id = active.lead_id.clone().unwrap();
            let to_user_id = active.to_user_id.clone().unwrap();
            let reason = active.reason.clone().unwrap();

            active.approval_status = Set(customer_transfer_approval::STATUS_APPROVED.to_string());
            active.completed_at = Set(Some(now));
            active.to_user_name = Set(Some(format!("用户{}", to_user_id)));
            active.updated_at = Set(now);

            let updated = active.update(&txn).await?;
            // 显式 commit 审批状态变更，再执行实际转移
            txn.commit().await?;

            self.execute_transfer(lead_id, to_user_id, director_id, director_name, &reason)
                .await?;

            info!(
                "总监 {} 审批通过转移单 {}（大客户，已完成转移）",
                director_id, updated.approval_no
            );
            Ok(updated.into())
        } else {
            // 总监拒绝
            active.approval_status = Set(customer_transfer_approval::STATUS_REJECTED.to_string());
            active.updated_at = Set(now);
            let updated = active.update(&txn).await?;
            txn.commit().await?;

            info!(
                "总监 {} 拒绝转移单 {}，原因：{}",
                director_id, updated.approval_no, req.comment
            );
            Ok(updated.into())
        }
    }

    /// 申请人取消审批
    pub async fn cancel_approval(
        &self,
        approval_id: i32,
        operator_id: i32,
    ) -> Result<TransferApprovalDto, AppError> {
        let approval = TransferApprovalEntity::find_by_id(approval_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("审批单 {} 不存在", approval_id)))?;

        if approval.applicant_id != operator_id {
            return Err(AppError::business("取消审批失败：仅申请人可取消"));
        }

        if approval.approval_status != customer_transfer_approval::STATUS_PENDING {
            return Err(AppError::validation(
                "取消审批失败：审批单已进入终态（approved/rejected/cancelled）",
            ));
        }

        let mut active: customer_transfer_approval::ActiveModel = approval.into();
        active.approval_status = Set(customer_transfer_approval::STATUS_CANCELLED.to_string());
        active.updated_at = Set(chrono::Utc::now());
        let updated = active.update(&*self.db).await?;

        info!("用户 {} 取消转移审批单 {}", operator_id, updated.approval_no);
        Ok(updated.into())
    }

    /// 查询审批列表
    pub async fn list_approvals(
        &self,
        query: ApprovalQuery,
    ) -> Result<(Vec<TransferApprovalDto>, u64), AppError> {
        let page = query.page.unwrap_or(1).clamp(1, 1000);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

        let mut q = TransferApprovalEntity::find();

        if let Some(status) = query.status {
            q = q.filter(customer_transfer_approval::Column::ApprovalStatus.eq(status));
        }
        if let Some(applicant_id) = query.applicant_id {
            q = q.filter(customer_transfer_approval::Column::ApplicantId.eq(applicant_id));
        }
        if let Some(approver_id) = query.approver_id {
            // 同时匹配经理审批人或总监审批人
            q = q.filter(
                sea_orm::Condition::any()
                    .add(customer_transfer_approval::Column::ManagerApproverId.eq(approver_id))
                    .add(customer_transfer_approval::Column::DirectorApproverId.eq(approver_id)),
            );
        }

        let paginator = q
            .order_by(customer_transfer_approval::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items: Vec<customer_transfer_approval::Model> = paginator
            .fetch_page(page.saturating_sub(1))
            .await?;

        let dtos = items.into_iter().map(Into::into).collect();
        Ok((dtos, total))
    }

    /// 获取审批详情
    pub async fn get_approval(&self, approval_id: i32) -> Result<TransferApprovalDto, AppError> {
        let approval = TransferApprovalEntity::find_by_id(approval_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("审批单 {} 不存在", approval_id)))?;
        Ok(approval.into())
    }

    /// 检查是否大客户转移
    ///
    /// 判断依据（满足任一即为大客户）：
    /// 1. 线索已转化为客户，且 customer.credit_limit > 阈值
    /// 2. 线索已转化为客户，且 customer.customer_type = 'vip'
    async fn check_large_customer(&self, lead: &crm_lead::Model) -> Result<bool, AppError> {
        if let Some(customer_id) = lead.converted_customer_id {
            let customer = CustomerEntity::find_by_id(customer_id)
                .one(&*self.db)
                .await?;
            if let Some(c) = customer {
                // vip 客户或信用额度超阈值
                if c.customer_type == "vip" {
                    return Ok(true);
                }
                let threshold = rust_decimal::Decimal::from(DEFAULT_LARGE_CUSTOMER_CREDIT_THRESHOLD);
                if c.credit_limit > threshold {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// 获取待审批的审批单（指定层级）
    async fn get_pending_approval(
        &self,
        approval_id: i32,
        expected_level: i32,
    ) -> Result<customer_transfer_approval::Model, AppError> {
        let approval = TransferApprovalEntity::find_by_id(approval_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("审批单 {} 不存在", approval_id)))?;

        if approval.approval_status != customer_transfer_approval::STATUS_PENDING {
            return Err(AppError::validation(format!(
                "审批失败：审批单当前状态为 {}，非 pending",
                approval.approval_status
            )));
        }

        if approval.current_level != expected_level {
            return Err(AppError::validation(format!(
                "审批失败：审批单当前审批层级为 {}，非期望层级 {}",
                approval.current_level, expected_level
            )));
        }

        Ok(approval)
    }

    /// 执行实际转移（调用 CrmAssignService::transfer_lead）
    async fn execute_transfer(
        &self,
        lead_id: i32,
        to_user_id: i32,
        operator_id: i32,
        operator_name: &str,
        reason: &str,
    ) -> Result<(), AppError> {
        let req = TransferLeadRequest {
            lead_id,
            to_user_id,
            reason: reason.to_string(),
            notes: Some("审批通过后自动执行".to_string()),
        };

        self.assign_service
            .transfer_lead(req, operator_id, operator_name)
            .await?;

        Ok(())
    }
}
