//! 销售报价单审批服务
//!
//! 业务功能：
//! - 金额阶梯审批：
//!   - < 10万：销售员自行审批
//!   - 10万 ~ 50万：销售经理审批
//!   - > 50万：总经理审批
//! - BPM 集成（提交 / 完成审批实例）
//!
//! Week 2 任务 7 - 销售报价单模块
//! 创建时间: 2026-06-16
//! 关联计划: 2026-06-16-sales-quotation-plan.md Task 7

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    // 批次 357 v13 复审 baseline 清零：移除 unused import ActiveModelTrait
    ColumnTrait,
    DatabaseConnection,
    EntityTrait,
    QueryFilter,
    QuerySelect,
    Set,
    TransactionTrait,
};
use std::sync::Arc;

use crate::container::AppState;
use crate::models::bpm_task;
use crate::models::dto::bpm_dto::StartProcessRequest;
use crate::models::sales_quotation::{
    self, ActiveModel as QuotationActive, Entity as QuotationEntity,
};
use crate::models::status::bpm_task as task_status;
use crate::models::status::quotation as quotation_status;
use crate::models::status::quotation_ext as quotation_ext_status;
use crate::services::bpm_ops::task::{APPROVE_ACTION, REJECT_ACTION};
use crate::services::bpm_service::BpmService;
use crate::utils::error::AppError;

/// 金额阶梯常量
const AMOUNT_THRESHOLD_SELF: i64 = 100_000; // 10 万
const AMOUNT_THRESHOLD_MANAGER: i64 = 500_000; // 50 万

/// 审批角色枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApproverRole {
    /// 销售员自批（< 10 万）
    Salesperson,
    /// 销售经理审批（10-50 万）
    SalesManager,
    /// 总经理审批（> 50 万）
    GeneralManager,
}

impl ApproverRole {
    /// 从金额判定审批角色
    pub fn from_amount(amount: Decimal) -> Self {
        // BE-B-1/BE-F-6 修复（2026-06-25 第二次全面审计）：
        // 原实现 amount.to_string().parse::<f64>().unwrap_or(0.0) as i64 存在两个问题：
        // 1. f64 精度损失（大金额比较错误）
        // 2. unwrap_or(0.0) 解析失败时金额被视为 0 → 命中 < 10万 分支 → 销售员自批绕过审批
        // 修复：直接用 Decimal 比较，避免 f64 中转与解析失败降级。
        let threshold_self = Decimal::new(AMOUNT_THRESHOLD_SELF, 0);
        let threshold_manager = Decimal::new(AMOUNT_THRESHOLD_MANAGER, 0);
        if amount < threshold_self {
            ApproverRole::Salesperson
        } else if amount < threshold_manager {
            ApproverRole::SalesManager
        } else {
            ApproverRole::GeneralManager
        }
    }

    /// 角色代码
    pub fn code(&self) -> &'static str {
        match self {
            ApproverRole::Salesperson => "self",
            ApproverRole::SalesManager => "sales_manager",
            ApproverRole::GeneralManager => "general_manager",
        }
    }
}

/// 审批服务
pub struct QuotationApprovalService {
    db: Arc<DatabaseConnection>,
}

impl QuotationApprovalService {
    /// 从数据库连接直接构造
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 从 AppState 构造便捷方法
    pub fn from_state(state: &AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// 提交报价单进入审批流
    /// 流程：1. 检查当前状态（仅 draft / rejected 可提交）；2. 根据金额选择审批角色；3. 自批：直接 approved；4. 否则：创建 BPM 流程实例 + 更新状态为 pending_approval；批次 85 v2 复审 P1-9 说明：submit 的状态门为预检查 + 金额判定（事务外查询），；真正的状态变更在 self_approve / submit_to_bpm 内各自的事务中完成：self_approve：lock_exclusive + 状态检查（P1-9 修复）；submit_to_bpm：lock_exclusive + 状态检查（已有）；因此 submit 的预检查无 lock 是可接受的，最终一致性由子方法保证
    pub async fn submit(
        &self,
        quotation_id: i64,
        user_id: i32,
    ) -> Result<sales_quotation::Model, AppError> {
        let quotation = QuotationEntity::find_by_id(quotation_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("报价单不存在"))?;

        if ![quotation_status::DRAFT, quotation_status::REJECTED]
            .contains(&quotation.status.as_str())
        {
            return Err(AppError::business(format!(
                "报价单当前状态不允许提交：{}",
                quotation.status
            )));
        }

        let role = ApproverRole::from_amount(quotation.total_amount);

        if role == ApproverRole::Salesperson {
            // 小额自批：直接 approved
            self.self_approve(quotation_id, user_id).await
        } else {
            // 中大额：提交 BPM
            self.submit_to_bpm(quotation, user_id, role).await
        }
    }

    /// 自批：直接标记为 approved
    /// 批次 85 v2 复审 P1-9 修复：在 lock 后补状态检查；原实现有 lock_exclusive 但无状态检查，submit 预检查后到 self_approve 加锁期间，；报价单可能已被并发修改（如取消），self_approve 会直接覆盖为 approved
    async fn self_approve(
        &self,
        quotation_id: i64,
        user_id: i32,
    ) -> Result<sales_quotation::Model, AppError> {
        // 批次 12（2026-06-28）：事务包裹"查询 + update_with_audit"，
        // 加 lock_exclusive 防止并发自批导致状态不一致；
        // update_with_audit 内部 2 次写入（实体 update + 审计 insert）非原子，事务包裹保证原子性。
        let txn = (*self.db).begin().await?;

        let quotation = QuotationEntity::find_by_id(quotation_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("报价单不存在"))?;

        // 状态检查（与 submit_to_bpm 一致）：仅 draft/rejected 可提交审批
        if ![quotation_status::DRAFT, quotation_status::REJECTED]
            .contains(&quotation.status.as_str())
        {
            return Err(AppError::business(format!(
                "报价单当前状态不允许审批：{}",
                quotation.status
            )));
        }

        let mut active: QuotationActive = quotation.into();
        active.status = Set(quotation_status::APPROVED.to_string());
        active.approved_by = Set(Some(user_id as i64));
        active.approved_at = Set(Some(Utc::now()));
        active.updated_at = Set(Utc::now());

        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;
        Ok(updated)
    }

    /// 提交至 BPM 流程
    async fn submit_to_bpm(
        &self,
        quotation: sales_quotation::Model,
        user_id: i32,
        _role: ApproverRole,
    ) -> Result<sales_quotation::Model, AppError> {
        // 批次 12（2026-06-28）：BPM 启动在事务外（容错），状态更新在事务内。
        // 先 BPM start_process 获取 instance_id，再事务包裹"查询 + 状态检查 + update_with_audit"，
        // 加 lock_exclusive 防止并发提交同一报价单导致状态不一致；
        // 若事务回滚，BPM 实例成为孤儿（容错设计，BPM 失败也不阻断主流程）。
        let bpm_service = BpmService::new(self.db.clone());
        let req = StartProcessRequest {
            process_key: "quotation_approval".to_string(),
            business_type: "quotation".to_string(),
            business_id: quotation.id as i32,
            title: format!("报价单审批 - {}", quotation.quotation_no),
            initiator_id: user_id,
            initiator_name: format!("user_{}", user_id),
            initiator_department_id: None,
            priority: Some("NORMAL".to_string()),
            form_data: Some(serde_json::json!({
                "quotation_no": quotation.quotation_no,
                "total_amount": quotation.total_amount.to_string(),
                "currency": quotation.currency,
            })),
            variables: None,
        };

        // 1. 启动 BPM 流程（事务外，容错：启动失败时 instance_id=None，业务提交仍生效）
        // 失败必须点名：静默降级为 None 会让后续「业务侧审批→回写 BPM 任务」整链
        // 无实例可查而空转，运维无从发现报价单已脱离 BPM 审批轨道。
        let bpm_instance_id: Option<i32> = match bpm_service.start_process(req).await {
            Ok(resp) => Some(resp.instance_id),
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    business_type = "quotation",
                    business_id = quotation.id,
                    "报价提交启动 BPM 流程失败：该报价单将没有 BPM 实例与待办任务，\
                     后续业务侧审批只落业务表（BPM 侧留痕缺失，需人工对账）"
                );
                None
            }
        };

        // 2. 事务包裹状态更新（重新查询并加锁，防止 quotation 已过期）
        let txn = (*self.db).begin().await?;

        let latest = QuotationEntity::find_by_id(quotation.id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("报价单不存在"))?;

        if ![quotation_status::DRAFT, quotation_status::REJECTED].contains(&latest.status.as_str())
        {
            return Err(AppError::business(format!(
                "报价单当前状态不允许提交：{}",
                latest.status
            )));
        }

        let mut active: QuotationActive = latest.into();
        active.status = Set(quotation_ext_status::PENDING_APPROVAL.to_string());
        active.approval_instance_id = Set(bpm_instance_id.map(|i| i as i64));
        active.updated_at = Set(Utc::now());

        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;
        Ok(updated)
    }

    /// 锁定报价单并校验状态为待审批
    async fn lock_and_validate_quotation_for_approval_txn(
        &self,
        quotation_id: i64,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<sales_quotation::Model, AppError> {
        let quotation = QuotationEntity::find_by_id(quotation_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found("报价单不存在"))?;
        if quotation.status != quotation_ext_status::PENDING_APPROVAL {
            return Err(AppError::business(format!(
                "报价单不在待审批状态：{}",
                quotation.status
            )));
        }
        Ok(quotation)
    }

    /// 构造审批通过的 ActiveModel
    ///
    /// 通过理由真实落 `approval_reason` 列（handler 侧已保证 trim 非空），不再只进日志。
    /// BPM 侧裁决意见的唯一载体仍是 `bpm_task.approval_opinion`，本列只承载业务侧裁量，
    /// 两端不互写（避免同一裁量双源）。
    fn build_approved_quotation_active_model(
        quotation: sales_quotation::Model,
        approver_id: i32,
        approval_reason: String,
    ) -> QuotationActive {
        let mut active: QuotationActive = quotation.into();
        active.status = Set(quotation_status::APPROVED.to_string());
        active.approved_by = Set(Some(approver_id as i64));
        active.approved_at = Set(Some(Utc::now()));
        active.approval_reason = Set(Some(approval_reason));
        active.updated_at = Set(Utc::now());
        active
    }

    /// 提交事务后完成 BPM 审批任务（容错：失败只记日志，不回滚业务侧已提交的审批结果）
    ///
    /// 定位口径：先按 business_type+business_id 定位流程实例，再按
    /// `instance_id + 写入方同源常量 bpm_task::PENDING` 定位**本单据**的待办任务，
    /// 只回写这些任务。旧形态「按办理人查用户待办分页」有两个结构性缺陷：
    /// 过滤值用大写 `"PENDING"` 字面量，与写入方小写词表（instance.rs/task.rs 写
    /// `bpm_task::PENDING="pending"`，DB CHECK 钉小写集）永不相等，回写循环恒空转；
    /// 且办理人过滤+分页上限 10 会在待办多于 10 条时漏查本单据任务，故一并弃用。
    async fn handle_bpm_quotation_approval_after_commit(
        &self,
        quotation_id: i64,
        approver_id: i32,
    ) {
        let bpm_service = BpmService::new(self.db.clone());
        let instance = match bpm_service
            .get_process_by_business("quotation", quotation_id as i32)
            .await
        {
            Ok(Some(instance)) => instance,
            Ok(None) => {
                tracing::warn!(
                    business_type = "quotation",
                    business_id = quotation_id,
                    "报价审批 BPM 回写：未找到该单据的流程实例，待办任务无法闭环，需人工对账"
                );
                return;
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    business_type = "quotation",
                    business_id = quotation_id,
                    "报价审批 BPM 回写：查询流程实例失败，待办任务未闭环"
                );
                return;
            }
        };
        let tasks = match bpm_task::Entity::find()
            .filter(bpm_task::Column::InstanceId.eq(instance.id))
            .filter(bpm_task::Column::Status.eq(task_status::PENDING))
            .all(&*self.db)
            .await
        {
            Ok(tasks) => tasks,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    business_type = "quotation",
                    business_id = quotation_id,
                    instance_id = instance.id,
                    "报价审批 BPM 回写：查询待办任务失败，任务未闭环"
                );
                return;
            }
        };
        if tasks.is_empty() {
            tracing::warn!(
                business_type = "quotation",
                business_id = quotation_id,
                instance_id = instance.id,
                expected_task_status = task_status::PENDING,
                "报价审批 BPM 回写：该实例下无状态为待处理的 bpm_task，任务未闭环，需人工对账"
            );
            return;
        }
        for task in tasks {
            // approve_task 的处理人取已认证操作人（user_id 参数，见 bpm_ops/task.rs），
            // req.handler_id/handler_name 在当前签名下不被消费，仅按 DTO 形状填充。
            if let Err(e) = bpm_service
                .approve_task(
                    crate::models::dto::bpm_dto::ApproveTaskRequest {
                        task_id: task.id,
                        handler_id: approver_id,
                        handler_name: String::new(),
                        action: APPROVE_ACTION.to_string(),
                        approval_opinion: None,
                        attachment_urls: None,
                    },
                    Some(approver_id),
                )
                .await
            {
                tracing::warn!(
                    error = %e,
                    task_id = task.id,
                    business_type = "quotation",
                    business_id = quotation_id,
                    "BPM 报价单审批通过任务失败（不阻断已提交的业务状态）"
                );
            } else {
                tracing::info!(
                    task_id = task.id,
                    business_id = quotation_id,
                    "报价审批通过 BPM 回写完成"
                );
            }
        }
    }

    /// 经理/总经理审批通过
    ///
    /// `approval_reason` 为业务侧通过理由（handler 已收口 trim 非空），真实落
    /// `approval_reason` 列；BPM 待办的裁决意见另由 `bpm_task.approval_opinion` 承载，
    /// 本方法不把业务理由回灌 BPM、也不从 BPM 反灌业务列（禁双写）。
    pub async fn approve(
        &self,
        quotation_id: i64,
        approver_id: i32,
        approval_reason: String,
    ) -> Result<sales_quotation::Model, AppError> {
        // 事务包裹查询+状态检查+审计更新，BPM 任务审批在事务外执行（容错）
        let txn = (*self.db).begin().await?;
        let quotation = self
            .lock_and_validate_quotation_for_approval_txn(quotation_id, &txn)
            .await?;
        let active =
            Self::build_approved_quotation_active_model(quotation, approver_id, approval_reason);
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active,
            Some(approver_id),
        )
        .await?;
        txn.commit().await?;
        // 完成 BPM 任务（事务外，容错）
        if updated.approval_instance_id.is_some() {
            self.handle_bpm_quotation_approval_after_commit(quotation_id, approver_id)
                .await;
        } else {
            // 状态已是 pending_approval 却无实例：提交时 BPM 启动失败或旧数据遗留，
            // 必须点名——这意味着 BPM 侧永无任务可闭环（静默跳过即二次空转）。
            tracing::warn!(
                business_type = "quotation",
                business_id = quotation_id,
                "报价审批通过 BPM 回写跳过：单据处于待审批状态但未关联 BPM 实例，BPM 侧无任务可闭环，需人工对账"
            );
        }
        Ok(updated)
    }

    /// 审批拒绝
    pub async fn reject(
        &self,
        quotation_id: i64,
        approver_id: i32,
        reason: String,
    ) -> Result<sales_quotation::Model, AppError> {
        // 事务包裹"查询 + 状态检查 + update_with_audit"，加 lock_exclusive 防并发；
        // BPM 任务审批在事务外执行（容错，失败不阻断已提交状态）
        let txn = (*self.db).begin().await?;
        let updated = self
            .reject_quotation_txn(quotation_id, approver_id, &reason, &txn)
            .await?;
        txn.commit().await?;
        // 完成 BPM 任务（事务外，容错）
        self.complete_rejection_bpm(&updated, approver_id, &reason)
            .await;
        Ok(updated)
    }

    /// 事务内执行拒绝：lock_exclusive + 状态校验 + 构建更新 + 审计更新
    async fn reject_quotation_txn(
        &self,
        quotation_id: i64,
        approver_id: i32,
        reason: &str,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<sales_quotation::Model, AppError> {
        // 复用审批锁校验：lock_exclusive + 状态为待审批
        let quotation = self
            .lock_and_validate_quotation_for_approval_txn(quotation_id, txn)
            .await?;
        let mut active: QuotationActive = quotation.into();
        active.status = Set(quotation_status::REJECTED.to_string());
        active.rejection_reason = Set(Some(reason.to_string()));
        // 结论人/结论时间列口径与 approve 路径一致：本域只有 approved_by/approved_at
        // 一列套承载"审批结论"，结论性质由 status 区分，拒绝同样必须留决策时间与决策人。
        active.approved_by = Set(Some(approver_id as i64));
        active.approved_at = Set(Some(Utc::now()));
        active.updated_at = Set(Utc::now());
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            active,
            Some(approver_id),
        )
        .await?;
        Ok(updated)
    }

    /// 事务外完成 BPM 拒绝任务（容错：失败只记日志，不回改业务侧已提交的拒绝结果）
    ///
    /// 定位口径与 `handle_bpm_quotation_approval_after_commit` 完全一致：
    /// business_type+business_id 定位实例，`instance_id + bpm_task::PENDING`（写入方
    /// 同源常量）定位本单据待办任务；拒绝理由只写 `bpm_task.approval_opinion`
    /// 这一 BPM 载体，不在业务表另建第二份理由列（业务表 `rejection_reason` 是
    /// 本域既有专列，维持原写入点不动）。
    async fn complete_rejection_bpm(
        &self,
        updated: &sales_quotation::Model,
        approver_id: i32,
        reason: &str,
    ) {
        if updated.approval_instance_id.is_none() {
            // 小额自批路径从未启动 BPM 流程，无任务可回写，属正常分支
            tracing::debug!(
                business_type = "quotation",
                business_id = updated.id,
                "报价拒绝 BPM 回写跳过：该单据未关联 BPM 实例（小额自批路径）"
            );
            return;
        }
        let bpm_service = BpmService::new(self.db.clone());
        let instance = match bpm_service
            .get_process_by_business("quotation", updated.id as i32)
            .await
        {
            Ok(Some(instance)) => instance,
            Ok(None) => {
                tracing::warn!(
                    business_type = "quotation",
                    business_id = updated.id,
                    approval_instance_id = ?updated.approval_instance_id,
                    "报价拒绝 BPM 回写：业务单记录了实例 ID 但按单据查不到流程实例，拒绝理由未进 BPM，需人工对账"
                );
                return;
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    business_type = "quotation",
                    business_id = updated.id,
                    "报价拒绝 BPM 回写：查询流程实例失败，拒绝理由未进 BPM"
                );
                return;
            }
        };
        let tasks = match bpm_task::Entity::find()
            .filter(bpm_task::Column::InstanceId.eq(instance.id))
            .filter(bpm_task::Column::Status.eq(task_status::PENDING))
            .all(&*self.db)
            .await
        {
            Ok(tasks) => tasks,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    business_type = "quotation",
                    business_id = updated.id,
                    instance_id = instance.id,
                    "报价拒绝 BPM 回写：查询待办任务失败，拒绝理由未进 BPM"
                );
                return;
            }
        };
        if tasks.is_empty() {
            tracing::warn!(
                business_type = "quotation",
                business_id = updated.id,
                instance_id = instance.id,
                expected_task_status = task_status::PENDING,
                "报价拒绝 BPM 回写：该实例下无状态为待处理的 bpm_task，拒绝理由未进 BPM，需人工对账"
            );
            return;
        }
        for task in tasks {
            // approve_task 的处理人取已认证操作人（user_id 参数），req.handler_* 不被消费
            if let Err(e) = bpm_service
                .approve_task(
                    crate::models::dto::bpm_dto::ApproveTaskRequest {
                        task_id: task.id,
                        handler_id: approver_id,
                        handler_name: String::new(),
                        action: REJECT_ACTION.to_string(),
                        approval_opinion: Some(reason.to_string()),
                        attachment_urls: None,
                    },
                    Some(approver_id),
                )
                .await
            {
                tracing::warn!(
                    error = %e,
                    task_id = task.id,
                    business_type = "quotation",
                    business_id = updated.id,
                    "BPM 报价单审批拒绝任务失败（不阻断已提交的业务状态）"
                );
            } else {
                tracing::info!(
                    task_id = task.id,
                    business_id = updated.id,
                    "报价拒绝 BPM 回写完成：任务已拒绝、理由已落 approval_opinion"
                );
            }
        }
    }
}
