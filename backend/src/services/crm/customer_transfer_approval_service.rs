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
//! 二级分支（总监审批）的可达性与执行形态（如实说明，与源码行为逐字一致）：
//! 创建门 `fetch_and_validate_lead` 对 `lead_status=CONVERTED` 线索**有条件放行**——仅当
//! 其挂链客户（`converted_customer_id`）的分层列 `customers.tier` 落在
//! `constants::customer_tier::MAJOR` 高档集合；其余 CONVERTED 形态（未挂链/未定档/低档）
//! 一律仍拒，一级审批门前置不因放开二级而松动。分层列的受控写入口是
//! `handlers/customer_handler.rs::update_customer_tier`（PUT /customers/{id}/tier，
//! 全仓唯一 HTTP 分层写腿），因此二级链路对正常生命周期数据真实可达。
//!
//! 审批通过后的执行腿按来源线索形态分派（`execute_transfer_in` 单点判型）：
//! - CONVERTED 且挂链客户 → 转移**挂链客户归属**（`customers.owner_id`，客户才是审批对象，
//!   表名 customer_transfer_approvals 与此对齐）；线索行不写：`lead_status` 保持终态
//!   CONVERTED、`converted_customer_id` 不清空、`lead.rs` 转化腿的"禁止从 CONVERTED 回退"
//!   门与本链无关且原样有效；
//! - 非 CONVERTED（含"已挂链未转化"历史杂交形态）→ 走既有线索归属腿
//!   `CrmAssignService::transfer_lead_in`，其 CONVERTED 拒绝门与自转门判据逐字不变。
//!
//! 状态跃迁与转移执行**同事务原子提交**（`manager_approve` 单级分支与 `director_approve`
//! 通过分支）：执行腿失败整体回滚，审批单保持 pending，杜绝旧形态"先 commit approved
//! 再转移"留下的"已批准未转移"半态。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DatabaseTransaction, EntityTrait, Order,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::constants::customer_tier;
use crate::models::status::crm_lead as lead_status;
use crate::models::{
    crm_lead::{self, Entity as CrmLeadEntity},
    customer::{self, Entity as CustomerEntity},
    customer_transfer_approval::{self, Entity as TransferApprovalEntity},
    user,
};
use crate::services::assignment_history_service::{
    AssignmentHistoryService, CreateAssignmentHistoryRequest,
};
use crate::services::audit_log_service::AuditLogService;
use crate::services::crm::assign::{CrmAssignService, TransferLeadRequest, TransferLeadResult};
use crate::utils::data_scope::{
    DataScopeContext, apply_data_scope, check_resource_owner_by_member_scope,
};
use crate::utils::error::AppError;
use crate::utils::redis_cache::{cache_key, redis_cache_del};

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
    /// 1. 线索必须存在；`lead_status=CONVERTED` 的线索仅在其挂链客户分层命中
    ///    `constants::customer_tier::MAJOR`（大客户二级链准入）时放行，其余 CONVERTED
    ///    形态一律仍拒（详见 `fetch_and_validate_lead` 的准入门与模块文档「二级分支」段）；
    /// 2. 新归属人必须不等于当前归属人；
    /// 3. `max_level` 由 `check_large_customer` 计算：线索关联客户分层落在高档集合为 2，
    ///    否则为 1；放行进来的 CONVERTED 大客户线索恒得 `max_level=2`（二级完整可达），
    ///    正常未转化线索仍恒为 1；
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

    /// 创建审批：查询线索并校验准入（状态门 + 大客户准入门 + 自转门）
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
            // 大客户准入门（用户终裁放开形态）：CONVERTED 线索只有挂链客户分层落在
            // `customer_tier::MAJOR` 时可发起转移审批——审批对象随之为挂链客户归属
            // （执行腿分派见 `execute_transfer_in`；线索状态保持终态，不解除转化腿的
            // "禁止从 CONVERTED 回退"门）。其余形态（未挂链/未定档/低档）一律仍拒：
            // 放开二级不得顺手拆掉一级审批的门前置，该负向不变量由
            // `tests/contract_wave17_converted_non_major_create_reject_lock_test.rs` 钉死。
            // 状态门语义未满足归业务族；文案含内部线索 ID 保持脱敏。
            if !self.check_large_customer(&lead).await? {
                return Err(AppError::business(format!(
                    "线索 {} 已转化为客户且所挂客户非大高档分层，无法转移",
                    lead_id
                )));
            }
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
                // 普通客户：执行转移与状态跃迁同事务——先在事务内转移成功、再落 approved
                // 一并提交，任一失败整体回滚（旧形态先 commit approved 再转移，转移失败
                // 留下"已批准未转移"半态，属独立缺陷，本处一并消除）
                let (transfer_result, transferred_customer_id) = self
                    .execute_transfer_in(
                        &txn,
                        lead_id,
                        to_user_id,
                        manager_id,
                        manager_name,
                        &reason,
                        ctx,
                    )
                    .await?;
                active.approval_status =
                    Set(customer_transfer_approval::STATUS_APPROVED.to_string());
                active.completed_at = Set(Some(now));
                // to_user_name 语义是**被转移人（第三方 to_user_id）**的展示名，唯一真实来源
                // = 执行腿透传的 TransferLeadResult.to_user_name（新归属人真实用户名，
                // 线索腿由 transfer_lead_in 原样带回、客户腿在执行腿内查得）。审批行与
                // 转移同事务一次落齐，不造名、不落空、不顶替，也不再需要事后第二笔回写。
                active.to_user_name = Set(Some(transfer_result.to_user_name));

                let updated = active.update(&txn).await?;
                txn.commit().await?;
                Self::invalidate_customer_cache(transferred_customer_id).await;

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
    /// 业务规则：1. 审批单必须存在且为 pending 状态；2. current_level 必须为 2（总监审批层）；
    /// 3. max_level 必须为 2（大客户）；4. 通过：执行转移与状态跃迁**同事务**提交——转移
    ///    执行腿任一步失败整体回滚，审批单保持 pending，不留"已批准未转移"半态；
    /// 5. 拒绝：标记 rejected，不执行转移
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
            // 总监通过：执行转移与状态跃迁同事务（先转移成功、再落 approved，一并提交）
            let (transfer_result, transferred_customer_id) = self
                .execute_transfer_in(
                    &txn,
                    lead_id,
                    to_user_id,
                    director_id,
                    director_name,
                    &reason,
                    ctx,
                )
                .await?;
            active.approval_status = Set(customer_transfer_approval::STATUS_APPROVED.to_string());
            active.completed_at = Set(Some(now));
            // to_user_name 与 manager_approve 同一口径：执行腿透传的 TransferLeadResult
            // 真实用户名随状态跃迁一次落齐（同事务、零事后回写），不在审批行造名。
            active.to_user_name = Set(Some(transfer_result.to_user_name));

            let updated = active.update(&txn).await?;
            txn.commit().await?;
            Self::invalidate_customer_cache(transferred_customer_id).await;

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

    /// 审批通过后的执行腿分派（单点判型，`manager_approve` 单级分支与 `director_approve`
    /// 通过分支共用；在调用方事务内执行，随审批状态跃迁同事务提交）。
    /// 返回值 `(TransferLeadResult, Option<i32>)`：前者是被转移人真实姓名的透传载体
    /// （两腿共用 `TransferLeadResult`，审批行写回只消费它的 `to_user_name`，契约锁
    /// `tests/contract_wave6_crm_transfer_approval_to_user_name_test.rs` 钉的就是这一
    /// 字面来源可追溯形态）；后者是本次执行腿是否写了客户行的信号（客户腿 Some 挂链
    /// 客户号、线索腿 None），调用方据此在**事务提交后**失效客户读缓存。
    ///
    /// 判型规则（与模块文档「二级分支」段逐字同源）：
    /// - 来源线索为 CONVERTED 终态 → 转移**挂链客户归属**（`transfer_customer_owner_in`），
    ///   线索行零改动：`lead_status` 保持 CONVERTED、`converted_customer_id` 不清空、
    ///   转化腿"禁止从 CONVERTED 回退"门与本链无关且原样有效；
    /// - 非 CONVERTED（正常未转化线索，或历史杂交形态"已挂链未转化"）→ 既有线索归属腿
    ///   `CrmAssignService::transfer_lead_in`，其存在/未转化/非自转/新归属人活跃各门
    ///   判据逐字不变（`fetch_lead_for_transfer` 的 CONVERTED 拒绝门未放开）。
    /// 入参：事务 + 线索 ID + 目标归属人 + 操作人（审批人，用于审计与历史 actor）+
    /// 申请原因（写入历史 reason）+ 数据范围上下文（客户行级归属门用）。
    async fn execute_transfer_in(
        &self,
        txn: &DatabaseTransaction,
        lead_id: i32,
        to_user_id: i32,
        operator_id: i32,
        operator_name: &str,
        reason: &str,
        ctx: &DataScopeContext,
    ) -> Result<(TransferLeadResult, Option<i32>), AppError> {
        let lead = CrmLeadEntity::find_by_id(lead_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("线索 {} 不存在", lead_id)))?;
        if lead.lead_status.as_deref() == Some(lead_status::CONVERTED) {
            let result = self
                .transfer_customer_owner_in(
                    txn,
                    &lead,
                    to_user_id,
                    operator_id,
                    operator_name,
                    reason,
                    ctx,
                )
                .await?;
            // 客户腿执行成功即必写了客户行（挂链号为 NULL 时该腿已点名拒绝），
            // 回传挂链客户号供调用方提交后失效客户读缓存
            Ok((result, lead.converted_customer_id))
        } else {
            let req = TransferLeadRequest {
                lead_id,
                to_user_id,
                reason: reason.to_string(),
                notes: Some("审批通过后自动执行".to_string()),
            };
            // 线索腿的 TransferLeadResult 原样透传，不重构造；未写客户行，缓存失效信号为 None
            let result = self
                .assign_service
                .transfer_lead_in(txn, &req, operator_id, operator_name)
                .await?;
            Ok((result, None))
        }
    }

    /// 转移挂链客户归属（`customers.owner_id`）——审批执行腿的服务内受控写函数。
    /// **不开放 HTTP 面**：唯一入口是经理/总监审批通过后的 `execute_transfer_in` 分派，
    /// 授权来源是收细后的两枚独立审批动作键（managerapprove/directorapprove，由
    /// PATH_ACTION_SEGMENT_MAP 从 URL 末段显式映射派生，与建单 create 键分离）
    /// 叠加审批单行级归属门（`get_pending_approval` 已把门）。
    ///
    /// 语义件全部复用既有形态，不造第二套：
    /// - 行级归属门：与本服务审批单门同一判据源 `check_resource_owner_by_member_scope`，
    ///   施加于客户行 `owner_id` 列（与客户读腿 `get_customer` 的归属列同列同源；
    ///   "读可 All"的 `check_resource_owner` 不在此复用）；
    /// - 新归属人校验：复用 `CrmAssignService::fetch_and_validate_new_owner`（存在+活跃，
    ///   真实用户名即审批行 to_user_name 唯一来源）；
    /// - 自转门：新归属人已是该客户当前归属人 → 业务族可展示拒绝（与线索腿同语义）；
    /// - 落痕：`AssignmentHistoryService::create_with_txn` 同事务写 TRANSFER 历史——
    ///   线索侧即"只作为审批来源留痕"（lead_id 挂审批来源线索，不写线索行任何列）；
    /// - 审计：`AuditLogService::update_with_audit`（resource_type=customer，与整行更新
    ///   腿、软删腿同一范式，事务内原子）。
    ///
    /// ES 同步判据（如实说明，非旁路）：客户 ES 文档构造器
    /// `customer_ops/update.rs::build_customer_doc` 的字段面不含归属列（id/编码/名称/
    /// 联系人/渠道），纯归属变化不产生 ES 文档漂移，故本腿不重推索引；若日后 ES 面
    /// 纳入归属字段，本腿必须同步接入。
    /// 只写 `owner_id`、`owner_assigned_at`（保护期既有语义列，转移即重新起算）、`updated_at`
    /// 三列；`department_id` 由 DB 触发器随 owner 变更自动维护，应用侧不写。
    /// 返回与线索腿同形的 `TransferLeadResult`（透传载体）：`to_user_name` 即本腿内
    /// `fetch_and_validate_new_owner` 查得的真实用户名，调用方据此回写审批行。
    async fn transfer_customer_owner_in(
        &self,
        txn: &DatabaseTransaction,
        lead: &crm_lead::Model,
        to_user_id: i32,
        operator_id: i32,
        operator_name: &str,
        reason: &str,
        ctx: &DataScopeContext,
    ) -> Result<TransferLeadResult, AppError> {
        // 建单门已保证 CONVERTED 准入线索必带挂链客户；走到这里仍为 NULL 即脏数据，
        // 点名拒绝（业务族、含 ID 脱敏），不静默不造值
        let customer_id = lead.converted_customer_id.ok_or_else(|| {
            AppError::business(format!(
                "线索 {} 已转化但无挂链客户记录，客户归属转移不可执行",
                lead.id
            ))
        })?;
        // 新归属人存在+活跃门（与线索腿同一判据源，先于任何写操作，失败零漂移）
        let new_owner = self
            .assign_service
            .fetch_and_validate_new_owner(to_user_id)
            .await?;
        let customer = CustomerEntity::find_by_id(customer_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| {
                // 转移前提缺失（挂链客户行不存在）：业务族拒绝并整体回滚，
                // 审批单保持 pending，绝不"批准了一个无处可去的转移"
                AppError::business(format!(
                    "挂链客户 {} 不存在，客户归属转移不可执行",
                    customer_id
                ))
            })?;
        if customer.owner_id == to_user_id {
            return Err(AppError::business_displayable(
                "转移审批执行失败：新归属人已是该客户当前归属人",
            ));
        }
        // 行级归属门：审批人对客户现行归属人不可见即拒绝（判据源与审批单门同一，
        // 原因只进日志，出参固定脱敏）
        if !check_resource_owner_by_member_scope(ctx, Some(customer.owner_id)) {
            tracing::warn!(
                actor = ctx.user_id,
                customer_id,
                resource_owner = customer.owner_id,
                "审批执行腿客户归属转移未过行级归属门，整体回滚（原因不外显）"
            );
            return Err(AppError::permission_denied(
                "无权执行该客户归属转移（数据范围限制）".to_string(),
            ));
        }
        let now = chrono::Utc::now();
        let from_user_id = customer.owner_id;
        // 原归属人展示名取自 users 的单次查得（与历史行同一来源，不造名）：查不到
        // 用户行（公海 0 或悬空引用）即前置数据不完整，点名拒绝并整体回滚，与
        // "挂链客户不存在"同一 fail-visible 口径——不放行一笔写不出原归属人的转移。
        let from_user_name = user::Entity::find_by_id(from_user_id)
            .one(txn)
            .await?
            .map(|u| u.username)
            .ok_or_else(|| {
                AppError::business(format!(
                    "客户 {} 的原归属人 {} 不可解析，客户归属转移不可执行",
                    customer_id, from_user_id
                ))
            })?;
        let mut active: customer::ActiveModel = customer.into();
        active.owner_id = Set(to_user_id);
        active.owner_assigned_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated =
            AuditLogService::update_with_audit(txn, "customer", active, Some(operator_id)).await?;
        AssignmentHistoryService::new(self.db.clone())
            .create_with_txn(
                txn,
                operator_id,
                operator_name,
                CreateAssignmentHistoryRequest {
                    lead_id: lead.id,
                    lead_no: lead.lead_no.clone(),
                    company_name: Some(updated.customer_name.clone()),
                    from_user_id: Some(from_user_id),
                    from_user_name: Some(from_user_name.clone()),
                    to_user_id: Some(new_owner.id),
                    to_user_name: Some(new_owner.username.clone()),
                    action: "TRANSFER".to_string(),
                    reason: Some(reason.to_string()),
                    notes: Some("审批通过后自动执行客户归属转移".to_string()),
                },
            )
            .await?;
        Ok(TransferLeadResult {
            lead_id: lead.id,
            lead_no: lead.lead_no.clone(),
            from_user_id,
            from_user_name,
            to_user_id: new_owner.id,
            to_user_name: new_owner.username,
            transferred_at: now,
        })
    }

    /// 事务提交后的收尾：本次执行腿写了客户行时失效客户读缓存
    /// （键与 `customer_ops` 读腿缓存逐字同源；必须在 commit 之后调用——
    /// 提交前删除会被并发读以旧行重建回缓存）。
    async fn invalidate_customer_cache(transferred_customer_id: Option<i32>) {
        if let Some(customer_id) = transferred_customer_id {
            redis_cache_del(&cache_key("customer", customer_id)).await;
        }
    }
}
