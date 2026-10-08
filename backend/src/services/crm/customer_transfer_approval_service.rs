//! 客户转移审批服务（crm/customer_transfer_approval）
//!
//! 提供线索/客户转移的分级审批流。审批级数由 `check_large_customer` 计算：命中大客户
//! 高档分层集合的走两级（销售经理 → 总监），否则单级（销售经理审批即完成）。
//!
//! 流程（按 `max_level` 分支，非独立开关）：
//! 1. 销售员发起转移申请 → 创建审批单（pending），按 `check_large_customer` 落 `max_level`
//! 2. 销售经理审批通过：
//!    - `max_level=1`（普通客户）：直接执行转移并置 approved
//!    - `max_level=2`（大客户）：进入总监审批层（`current_level=2`），此刻不执行转移
//! 3. 总监审批通过 → 执行转移并置 approved；任意层级拒绝 → rejected，不执行转移
//!
//! 关联：审批通过后调用 `CrmAssignService::transfer_lead` 执行实际转移。
//!
//! 二级分支的真实可达性（如实说明，勿当已就绪能力）：`check_large_customer` 的唯一输入是
//! 线索 `converted_customer_id` 所指向客户的分层列（`customers.tier` 是否落在
//! `constants::customer_tier::MAJOR`）。而 `converted_customer_id` 仅在线索转化时与
//! `lead_status=CONVERTED` 同时写入；创建门 `fetch_and_validate_lead` 又拒绝 `CONVERTED`
//! 线索。因此按正常生命周期，能通过创建门的线索其 `converted_customer_id` 恒为 NULL，
//! `check_large_customer` 恒判非大客户、`max_level` 恒为 1，二级（总监）分支对正常生命周期
//! 数据不可达；该分支仅在线索同时携带 `converted_customer_id` 且状态非 `CONVERTED`（正常
//! 生命周期不产出此组合）时命中。总监层审批逻辑本身完整、未被删减，只是无正常来源触发它。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::constants::customer_tier;
use crate::models::status::crm_lead as lead_status;
use crate::models::{
    crm_lead::{self, Entity as CrmLeadEntity},
    customer::Entity as CustomerEntity,
    customer_transfer_approval::{self, Entity as TransferApprovalEntity},
};
use crate::services::crm::assign::{CrmAssignService, TransferLeadRequest, TransferLeadResult};
use crate::utils::data_scope::{
    DataScopeContext, apply_data_scope, check_resource_owner_by_member_scope,
};
use crate::utils::error::AppError;

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
        Self { db, assign_service }
    }

    /// 创建转移审批申请。
    /// 业务规则：
    /// 1. 线索必须存在且未转化为客户（`lead_status=CONVERTED` 在本步被拒）；
    /// 2. 新归属人必须不等于当前归属人；
    /// 3. `max_level` 由 `check_large_customer` 计算：线索关联客户分层落在高档集合为 2，
    ///    否则为 1；由于本步已拒绝 CONVERTED 线索、而 `converted_customer_id` 只在转化时
    ///    与 CONVERTED 同时写入，正常生命周期进入本函数的线索恒得 `max_level=1`
    ///    （二级分支可达性详见模块文档「二级分支的真实可达性」）；
    /// 4. 同一线索不能存在 pending 状态的审批单。
    pub async fn create_approval(
        &self,
        req: CreateTransferApprovalRequest,
        applicant_id: i32,
        applicant_name: &str,
    ) -> Result<TransferApprovalDto, AppError> {
        Self::validate_create_request(&req)?;
        let lead = self
            .fetch_and_validate_lead(req.lead_id, req.to_user_id)
            .await?;
        Self::ensure_no_pending_approval(&self.db, req.lead_id).await?;
        let is_large_customer = self.check_large_customer(&lead).await?;
        let max_level = if is_large_customer { 2 } else { 1 };
        let approval_no = crate::utils::number_generator::DocumentNumberGenerator::generate_no(
            &*self.db,
            "TA",
            TransferApprovalEntity,
            customer_transfer_approval::Column::ApprovalNo,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "转移审批单号生成失败");
            AppError::business_displayable("转移审批单号生成失败，请稍后重试")
        })?;
        let approval = Self::build_approval_active(
            &req,
            &lead,
            applicant_id,
            is_large_customer,
            max_level,
            approval_no,
        )
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

    /// 创建审批：校验申请原因非空
    fn validate_create_request(req: &CreateTransferApprovalRequest) -> Result<(), AppError> {
        if req.reason.trim().is_empty() {
            return Err(AppError::validation_displayable(
                "转移审批申请失败：申请原因不能为空",
            ));
        }
        Ok(())
    }

    /// 创建审批：查询线索并校验状态/归属人
    async fn fetch_and_validate_lead(
        &self,
        lead_id: i32,
        to_user_id: i32,
    ) -> Result<crm_lead::Model, AppError> {
        let lead = CrmLeadEntity::find_by_id(lead_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("线索 {} 不存在", lead_id)))?;
        if lead.lead_status.as_deref() == Some(lead_status::CONVERTED) {
            // 状态门：线索已处于「转为客户」终态，前置状态未满足，归业务族；文案含内部线索 ID 保持脱敏
            return Err(AppError::business(format!(
                "线索 {} 已转化为客户，无法转移",
                lead_id
            )));
        }
        if lead.owner_id == to_user_id {
            // 状态门：新归属人已是当前归属人，属「已处于某态不可重复动作」，归业务族；文案可外显
            return Err(AppError::business_displayable(
                "转移审批申请失败：新归属人已是当前归属人",
            ));
        }
        Ok(lead)
    }

    /// 创建审批：校验同一线索无 pending 审批
    async fn ensure_no_pending_approval(
        db: &sea_orm::DatabaseConnection,
        lead_id: i32,
    ) -> Result<(), AppError> {
        let existing_pending = TransferApprovalEntity::find()
            .filter(customer_transfer_approval::Column::LeadId.eq(lead_id))
            .filter(
                customer_transfer_approval::Column::ApprovalStatus
                    .eq(customer_transfer_approval::STATUS_PENDING),
            )
            .count(db)
            .await?;
        if existing_pending > 0 {
            // 状态门：该线索已存在待审批申请，属「已处于某态不可重复动作」，归业务族；文案可外显
            return Err(AppError::business_displayable(
                "转移审批申请失败：该线索已存在待审批的转移申请",
            ));
        }
        Ok(())
    }

    /// 创建审批：构造审批单 ActiveModel（to_user_name 待审批通过时由 transfer_lead 填充）
    fn build_approval_active(
        req: &CreateTransferApprovalRequest,
        lead: &crm_lead::Model,
        applicant_id: i32,
        is_large_customer: bool,
        max_level: i32,
        approval_no: String,
    ) -> customer_transfer_approval::ActiveModel {
        let now = chrono::Utc::now();
        customer_transfer_approval::ActiveModel {
            id: Default::default(),
            approval_no: Set(approval_no),
            lead_id: Set(req.lead_id),
            company_name: Set(lead.company_name.clone()),
            from_user_id: Set(lead.owner_id),
            from_user_name: Set(Some(lead.owner_name.clone())),
            to_user_id: Set(req.to_user_id),
            to_user_name: Set(None),
            applicant_id: Set(applicant_id),
            reason: Set(req.reason.clone()),
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
    }

    /// 销售经理审批
    /// 业务规则：1. 审批单必须存在且为 pending 状态；2. current_level 必须为 1（经理审批层）；3. 通过：普通客户（max_level=1）：直接执行转移并标记 completed；大客户（max_level=2）：进入总监审批层 current_level=2；4. 拒绝：标记 rejected，不执行转移
    pub async fn manager_approve(
        &self,
        req: ApproveRequest,
        manager_id: i32,
        manager_name: &str,
        ctx: &DataScopeContext,
    ) -> Result<TransferApprovalDto, AppError> {
        let approval = self.get_pending_approval(req.approval_id, 1, ctx).await?;

        // 在转为 ActiveModel 前从原始 Model 提取字段值，避免对 ActiveModel 调用 unwrap()
        let max_level = approval.max_level;
        let lead_id = approval.lead_id;
        let to_user_id = approval.to_user_id;
        let reason = approval.reason.clone();

        let txn = (*self.db).begin().await?;
        let now = chrono::Utc::now();

        let mut active: customer_transfer_approval::ActiveModel = approval.into();
        active.manager_approver_id = Set(Some(manager_id));
        active.manager_comment = Set(Some(req.comment.clone()));
        active.manager_approved_at = Set(Some(now));
        active.updated_at = Set(now);

        if req.approved {
            // 经理通过
            if max_level == 1 {
                // 普通客户：直接执行转移
                active.approval_status =
                    Set(customer_transfer_approval::STATUS_APPROVED.to_string());
                active.completed_at = Set(Some(now));
                // to_user_name 语义是**被转移人（第三方 to_user_id）**的展示名，唯一真实来源
                // = transfer_lead 内部 fetch_and_validate_new_owner 已查得的 new_owner.username
                // （assign.rs::build_transfer_result 的 to_user_name）。审批行先落状态，
                // 转移成功后把该真实姓名透传回写（零额外查询，不造名、不落空、不顶替）。

                let updated = active.update(&txn).await?;
                // 显式 commit 审批状态变更，再执行实际转移（transfer_lead 内部会自开事务）
                txn.commit().await?;

                let transfer_result = self
                    .execute_transfer(lead_id, to_user_id, manager_id, manager_name, &reason)
                    .await?;

                let mut named: customer_transfer_approval::ActiveModel = updated.into();
                named.to_user_name = Set(Some(transfer_result.to_user_name));
                named.updated_at = Set(chrono::Utc::now());
                let updated = named.update(&*self.db).await?;

                info!(
                    "销售经理 {} 审批通过转移单 {}（普通客户，已完成转移，被转移人 {}）",
                    manager_id, updated.approval_no, to_user_id
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
    /// 业务规则：1. 审批单必须存在且为 pending 状态；2. current_level 必须为 2（总监审批层）；3. max_level 必须为 2（大客户）；4. 通过：执行转移并标记 completed；5. 拒绝：标记 rejected，不执行转移
    pub async fn director_approve(
        &self,
        req: ApproveRequest,
        director_id: i32,
        director_name: &str,
        ctx: &DataScopeContext,
    ) -> Result<TransferApprovalDto, AppError> {
        let approval = self.get_pending_approval(req.approval_id, 2, ctx).await?;

        if approval.max_level != 2 {
            return Err(AppError::validation(
                "总监审批失败：该审批单不需要总监审批（非大客户转移）",
            ));
        }

        // 在转为 ActiveModel 前从原始 Model 提取字段值，避免对 ActiveModel 调用 unwrap()
        let lead_id = approval.lead_id;
        let to_user_id = approval.to_user_id;
        let reason = approval.reason.clone();

        let txn = (*self.db).begin().await?;
        let now = chrono::Utc::now();

        let mut active: customer_transfer_approval::ActiveModel = approval.into();
        active.director_approver_id = Set(Some(director_id));
        active.director_comment = Set(Some(req.comment.clone()));
        active.director_approved_at = Set(Some(now));
        active.updated_at = Set(now);

        if req.approved {
            // 总监通过：执行转移
            active.approval_status = Set(customer_transfer_approval::STATUS_APPROVED.to_string());
            active.completed_at = Set(Some(now));
            // to_user_name 与 manager_approve 同一口径：transfer_lead 已解析的真实新归属人
            // 用户名透传回写（零额外查询），不在审批行造名。

            let updated = active.update(&txn).await?;
            // 显式 commit 审批状态变更，再执行实际转移
            txn.commit().await?;

            let transfer_result = self
                .execute_transfer(lead_id, to_user_id, director_id, director_name, &reason)
                .await?;

            let mut named: customer_transfer_approval::ActiveModel = updated.into();
            named.to_user_name = Set(Some(transfer_result.to_user_name));
            named.updated_at = Set(chrono::Utc::now());
            let updated = named.update(&*self.db).await?;

            info!(
                "总监 {} 审批通过转移单 {}（大客户，已完成转移，被转移人 {}）",
                director_id, updated.approval_no, to_user_id
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
            // 状态门：审批单已进入终态，前置状态未满足不可取消，归业务族；文案可外显
            return Err(AppError::business_displayable(
                "取消审批失败：审批单已进入终态（approved/rejected/cancelled）",
            ));
        }

        let mut active: customer_transfer_approval::ActiveModel = approval.into();
        active.approval_status = Set(customer_transfer_approval::STATUS_CANCELLED.to_string());
        active.updated_at = Set(chrono::Utc::now());
        let updated = active.update(&*self.db).await?;

        info!(
            "用户 {} 取消转移审批单 {}",
            operator_id, updated.approval_no
        );
        Ok(updated.into())
    }

    /// 查询审批列表（行级数据权限）
    ///
    /// 以「申请人」为归属列下推行级过滤（`apply_data_scope`）：All=全量、Dept=申请人∈可见部门
    /// 成员集合、Self=仅本人。本表无 `department_id` 列，故 Dept 走「归属人∈成员集合」语义、
    /// owner 列与 dept 列两参同传。过滤在构造分页器之前施加，`total` 与可见集同源，杜绝
    /// "持键用户跨归属枚举全公司审批单"的水平越权。`applicant_id` / `approver_id` 客户端筛选
    /// 只在可见集内再收窄，不放宽可见面。
    pub async fn list_approvals(
        &self,
        query: ApprovalQuery,
        ctx: &DataScopeContext,
    ) -> Result<(Vec<TransferApprovalDto>, u64), AppError> {
        let page = query.page.unwrap_or(1).clamp(1, 1000);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

        let mut q = TransferApprovalEntity::find();

        // 行级数据权限：归属列取申请人，与详情/审批写门同源，不另造第二套判定规则
        q = apply_data_scope(
            q,
            ctx,
            customer_transfer_approval::Column::ApplicantId,
            customer_transfer_approval::Column::ApplicantId,
        );

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
        let items: Vec<customer_transfer_approval::Model> =
            paginator.fetch_page(page.saturating_sub(1)).await?;

        let dtos = items.into_iter().map(Into::into).collect();
        Ok((dtos, total))
    }

    /// 获取审批详情（行级数据权限）
    ///
    /// 不存在 → 404 先行；存在但申请人不在可见范围 → 403（固定脱敏文案、不含记录 ID）。
    /// 归属列取申请人，与列表 `apply_data_scope`、审批写门同源，不另造第二套判定。
    pub async fn get_approval(
        &self,
        approval_id: i32,
        ctx: &DataScopeContext,
    ) -> Result<TransferApprovalDto, AppError> {
        let approval = TransferApprovalEntity::find_by_id(approval_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("审批单 {} 不存在", approval_id)))?;
        Self::ensure_applicant_visible(ctx, approval.applicant_id)?;
        Ok(approval.into())
    }

    /// 单行归属门（详情读 / 经理审批 / 总监审批共用）。
    ///
    /// 判据与列表侧 `apply_data_scope` 的 Dept 分支同源——以「申请人 ∈ 可见部门成员集合」判断，
    /// 而非资源自身部门列（本表无 `department_id` 列，只能用 `check_resource_owner_by_member_scope`，
    /// 若误用 `check_resource_owner` 传 None 会把部门经理连自己的单都判死）。
    /// All=任意行；Dept=本人行或申请人∈可见成员集合（集合空退化仅本人）；Self=仅本人行；
    /// 申请人为 NULL 的历史行一律拒绝（此处 applicant_id 非空，恒有值）。越权返回的
    /// `PermissionDenied` 经 `public_message` 恒出固定脱敏常量，不含记录 ID。
    fn ensure_applicant_visible(ctx: &DataScopeContext, applicant_id: i32) -> Result<(), AppError> {
        if !check_resource_owner_by_member_scope(ctx, Some(applicant_id)) {
            return Err(AppError::permission_denied(
                "无权访问该客户转移审批单（数据范围限制）".to_string(),
            ));
        }
        Ok(())
    }

    /// 检查是否大客户转移。**唯一判据**：线索关联客户（`converted_customer_id`）的
    /// 分层列 `customers.tier` 落在高档集合 `constants::customer_tier::MAJOR`。
    /// 分层列是"大客户"的业务权威载体（用户终裁），以下形态一律判非大客户：
    /// - 线索未挂客户（正常生命周期未转化线索 `converted_customer_id` 恒 NULL）；
    /// - 客户分层为 NULL（未定档≠低档，无既有评级依据不许猜）或落在高档集合外
    ///   （SILVER/NORMAL 属阶梯低两档，判不进大客户）。
    /// **禁止**再以线索预估金额、信用额度等其它维度代理判"大客户"——那是第二套口径；
    /// 高档集合只允许定义在 `constants::customer_tier::MAJOR` 一处。
    async fn check_large_customer(&self, lead: &crm_lead::Model) -> Result<bool, AppError> {
        let Some(customer_id) = lead.converted_customer_id else {
            return Ok(false);
        };
        let customer = CustomerEntity::find_by_id(customer_id)
            .one(&*self.db)
            .await?;
        Ok(customer
            .as_ref()
            .and_then(|c| c.tier.as_deref())
            .is_some_and(customer_tier::is_major))
    }

    /// 获取待审批的审批单（指定层级，带行级数据权限）
    ///
    /// 归属门在状态/层级校验之前：越权者不得通过状态码/文案探测他人审批单的当前状态或层级；
    /// 门亦在任何落库之前（本函数只读，调用方拿到返回值后事务写入），故越权请求零审批、
    /// 零状态漂移、零审计写入。404（不存在）先于 403（不可见），与详情读同口径。
    async fn get_pending_approval(
        &self,
        approval_id: i32,
        expected_level: i32,
        ctx: &DataScopeContext,
    ) -> Result<customer_transfer_approval::Model, AppError> {
        let approval = TransferApprovalEntity::find_by_id(approval_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("审批单 {} 不存在", approval_id)))?;

        // 行级归属门：以申请人归属列判定，越权即 403 且在任何状态流转之前
        Self::ensure_applicant_visible(ctx, approval.applicant_id)?;

        if approval.approval_status != customer_transfer_approval::STATUS_PENDING {
            // 状态门：审批单当前非 pending，前置状态未满足，归业务族；文案含状态 token 保持脱敏
            return Err(AppError::business(format!(
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

    /// 执行实际转移（调用 CrmAssignService::transfer_lead）。
    /// 返回其 `TransferLeadResult`：其中 `to_user_name` 是 transfer_lead 内部
    /// `fetch_and_validate_new_owner(to_user_id)` 已查得的真实新归属人用户名
    /// （assign.rs::build_transfer_result），审批侧据此回写审批行，避免第二次查库。
    async fn execute_transfer(
        &self,
        lead_id: i32,
        to_user_id: i32,
        operator_id: i32,
        operator_name: &str,
        reason: &str,
    ) -> Result<TransferLeadResult, AppError> {
        let req = TransferLeadRequest {
            lead_id,
            to_user_id,
            reason: reason.to_string(),
            notes: Some("审批通过后自动执行".to_string()),
        };

        self.assign_service
            .transfer_lead(req, operator_id, operator_name)
            .await
    }
}
