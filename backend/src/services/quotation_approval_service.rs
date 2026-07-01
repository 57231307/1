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
    ActiveModelTrait, DatabaseConnection, EntityTrait, QuerySelect, Set, TransactionTrait,
};
use std::sync::Arc;

use crate::models::dto::bpm_dto::StartProcessRequest;
use crate::models::sales_quotation::{self, ActiveModel as QuotationActive, Entity as QuotationEntity};
use crate::services::bpm_service::BpmService;
use crate::utils::app_state::AppState;
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
    ///
    /// 流程：
    /// 1. 检查当前状态（仅 draft / rejected 可提交）
    /// 2. 根据金额选择审批角色
    /// 3. 自批：直接 approved
    /// 4. 否则：创建 BPM 流程实例 + 更新状态为 pending_approval
    pub async fn submit(&self, quotation_id: i64, user_id: i32) -> Result<sales_quotation::Model, AppError> {
        let quotation = QuotationEntity::find_by_id(quotation_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("报价单不存在"))?;

        if !["draft", "rejected"].contains(&quotation.status.as_str()) {
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

        let mut active: QuotationActive = quotation.into();
        active.status = Set("approved".to_string());
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

        // 1. 启动 BPM 流程（事务外，容错：找不到模板时 instance_id=None）
        let bpm_instance_id: Option<i32> = match bpm_service.start_process(req).await {
            Ok(resp) => Some(resp.instance_id),
            Err(_) => None,
        };

        // 2. 事务包裹状态更新（重新查询并加锁，防止 quotation 已过期）
        let txn = (*self.db).begin().await?;

        let latest = QuotationEntity::find_by_id(quotation.id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("报价单不存在"))?;

        if !["draft", "rejected"].contains(&latest.status.as_str()) {
            return Err(AppError::business(format!(
                "报价单当前状态不允许提交：{}",
                latest.status
            )));
        }

        let mut active: QuotationActive = latest.into();
        active.status = Set("pending_approval".to_string());
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

    /// 经理/总经理审批通过
    pub async fn approve(
        &self,
        quotation_id: i64,
        approver_id: i32,
    ) -> Result<sales_quotation::Model, AppError> {
        // 批次 12（2026-06-28）：事务包裹"查询 + 状态检查 + update_with_audit"，
        // 加 lock_exclusive 防止并发审批同一报价单导致重复审批或字段覆盖；
        // BPM 任务审批在事务外执行（容错，失败不阻断已提交状态）
        let txn = (*self.db).begin().await?;

        let quotation = QuotationEntity::find_by_id(quotation_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("报价单不存在"))?;

        if quotation.status != "pending_approval" {
            return Err(AppError::business(format!(
                "报价单不在待审批状态：{}",
                quotation.status
            )));
        }

        let mut active: QuotationActive = quotation.into();
        active.status = Set("approved".to_string());
        active.approved_by = Set(Some(approver_id as i64));
        active.approved_at = Set(Some(Utc::now()));
        active.updated_at = Set(Utc::now());

        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active,
            Some(approver_id),
        )
        .await?;

        txn.commit().await?;

        // 完成 BPM 任务（事务外，容错）
        if let Some(instance_id) = updated.approval_instance_id {
            let bpm_service = BpmService::new(self.db.clone());
            if let Ok(Some(instance)) = bpm_service
                .get_process_by_business("quotation", updated.id as i32)
                .await
            {
                if let Ok(tasks) = bpm_service
                    .query_user_tasks(crate::models::dto::bpm_dto::TaskQuery {
                        user_id: Some(approver_id),
                        status: Some("PENDING".to_string()),
                        page: Some(1),
                        page_size: Some(10),
                    })
                    .await
                {
                    for task in tasks.data {
                        if task.instance_id == instance.id {
                            // P0 修复（批次 4，2026-06-27）：原 `let _ = ...` 静默吞掉
                            // BPM 任务审批错误，改为 warn 日志记录，确保运维可观测。
                            if let Err(e) = bpm_service
                                .approve_task(
                                    crate::models::dto::bpm_dto::ApproveTaskRequest {
                                        task_id: task.id,
                                        handler_id: approver_id,
                                        handler_name: format!("user_{}", approver_id),
                                        action: "approve".to_string(),
                                        approval_opinion: None,
                                        attachment_urls: None,
                                    },
                                    // P0 8-4 修复：传入真实操作用户 approver_id 用于 BPM 审计追溯
                                    Some(approver_id),
                                )
                                .await
                            {
                                tracing::warn!(
                                    error = %e,
                                    task_id = task.id,
                                    quotation_id = updated.id,
                                    "BPM 报价单审批通过任务失败（不阻断主流程）"
                                );
                            }
                        }
                    }
                }
            }
            // 注：instance_id 字段记录 BPM 流程实例 id，仅作为引用保留
            let _ = instance_id;
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
        // 批次 12（2026-06-28）：事务包裹"查询 + 状态检查 + update_with_audit"，
        // 加 lock_exclusive 防止并发拒绝同一报价单导致重复拒绝或字段覆盖；
        // BPM 任务审批在事务外执行（容错，失败不阻断已提交状态）
        let txn = (*self.db).begin().await?;

        let quotation = QuotationEntity::find_by_id(quotation_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("报价单不存在"))?;

        if quotation.status != "pending_approval" {
            return Err(AppError::business(format!(
                "报价单不在待审批状态：{}",
                quotation.status
            )));
        }

        let mut active: QuotationActive = quotation.into();
        active.status = Set("rejected".to_string());
        active.approved_by = Set(Some(approver_id as i64));
        active.rejection_reason = Set(Some(reason.clone()));
        active.updated_at = Set(Utc::now());

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
            let bpm_service = BpmService::new(self.db.clone());
            if let Ok(Some(instance)) = bpm_service
                .get_process_by_business("quotation", updated.id as i32)
                .await
            {
                if let Ok(tasks) = bpm_service
                    .query_user_tasks(crate::models::dto::bpm_dto::TaskQuery {
                        user_id: Some(approver_id),
                        status: Some("PENDING".to_string()),
                        page: Some(1),
                        page_size: Some(10),
                    })
                    .await
                {
                    for task in tasks.data {
                        if task.instance_id == instance.id {
                            // P0 修复（批次 4，2026-06-27）：原 `let _ = ...` 静默吞掉
                            // BPM 任务审批错误，改为 warn 日志记录，确保运维可观测。
                            if let Err(e) = bpm_service
                                .approve_task(
                                    crate::models::dto::bpm_dto::ApproveTaskRequest {
                                        task_id: task.id,
                                        handler_id: approver_id,
                                        handler_name: format!("user_{}", approver_id),
                                        action: "reject".to_string(),
                                        approval_opinion: Some(reason.clone()),
                                        attachment_urls: None,
                                    },
                                    // P0 8-4 修复：传入真实操作用户 approver_id 用于 BPM 审计追溯
                                    Some(approver_id),
                                )
                                .await
                            {
                                tracing::warn!(
                                    error = %e,
                                    task_id = task.id,
                                    quotation_id = updated.id,
                                    "BPM 报价单审批拒绝任务失败（不阻断主流程）"
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_approver_role_small_amount_is_salesperson() {
        assert_eq!(ApproverRole::from_amount(dec!(50000)), ApproverRole::Salesperson);
        assert_eq!(ApproverRole::from_amount(dec!(99999)), ApproverRole::Salesperson);
    }

    #[test]
    fn test_approver_role_medium_amount_is_sales_manager() {
        assert_eq!(
            ApproverRole::from_amount(dec!(100000)),
            ApproverRole::SalesManager
        );
        assert_eq!(
            ApproverRole::from_amount(dec!(300000)),
            ApproverRole::SalesManager
        );
        assert_eq!(
            ApproverRole::from_amount(dec!(499999)),
            ApproverRole::SalesManager
        );
    }

    #[test]
    fn test_approver_role_large_amount_is_general_manager() {
        assert_eq!(
            ApproverRole::from_amount(dec!(500000)),
            ApproverRole::GeneralManager
        );
        assert_eq!(
            ApproverRole::from_amount(dec!(1000000)),
            ApproverRole::GeneralManager
        );
    }

    #[test]
    fn test_approver_role_code() {
        assert_eq!(ApproverRole::Salesperson.code(), "self");
        assert_eq!(ApproverRole::SalesManager.code(), "sales_manager");
        assert_eq!(ApproverRole::GeneralManager.code(), "general_manager");
    }
}
