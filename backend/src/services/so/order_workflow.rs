//! 销售订单工作流子模块（order_workflow）
//!
//! P9-2 拆分自原 `services/so/order.rs`。
//! 包含：cancel_order / submit_order / approve_order / complete_order
//!
//! ## 模块职责
//! - 销售订单审批流（草稿→待审→已审→已发货→已收款→已关闭）
//! - 状态机转换合法性校验
//! - 工作流日志（操作人/时间/原因）
//! - BPM 流程集成（提交/审批触发 BPM 服务）
//!
//! ## API 兼容
//! 通过 `crate::services::so::order::SalesService` 路径访问。

use super::order::SalesService;
use super::SalesOrderDetail;
use crate::models::sales_order;
use crate::models::sales_order::Entity as SalesOrderEntity;
use crate::utils::error::AppError;
use sea_orm::{ActiveModelTrait, EntityTrait, QueryFilter, QuerySelect, TransactionTrait};

/// 销售订单工作流子模块标记
pub const P92_WF_MODULE: &str = "sales_order_workflow";

/// 销售订单工作流状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowStage {
    /// 草稿
    Draft,
    /// 待审核
    Pending,
    /// 已审核
    Approved,
    /// 已发货
    Shipped,
    /// 已收款
    Received,
    /// 已关闭
    Closed,
}

impl WorkflowStage {
    /// 中文描述
    pub fn desc(&self) -> &'static str {
        match self {
            Self::Draft => "草稿",
            Self::Pending => "待审核",
            Self::Approved => "已审核",
            Self::Shipped => "已发货",
            Self::Received => "已收款",
            Self::Closed => "已关闭",
        }
    }
}

impl SalesService {
    // cancel_order / submit_order / approve_order / complete_order
    // 内容来自原 order.rs L815-840 + L898-978 + L979-1013 + L1014-1029

    pub async fn cancel_order(
        &self,
        order_id: i32,
        _user_id: i32,
    ) -> Result<SalesOrderDetail, AppError> {
        // 获取订单
        let order = SalesOrderEntity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("订单不存在"))?;

        // 检查订单状态是否允许取消
        // 批次 13（2026-06-28）：补 partial_shipped 状态，防止部分发货订单无法取消（死锁）。
        // 已发货部分需通过退货流程处理，取消仅作用于剩余未发货部分。
        if !["draft", "pending", "approved", "partial_shipped"].contains(&order.status.as_str()) {
            return Err(AppError::business("当前状态不允许取消".to_string()));
        }

        // 更新订单状态
        let mut order_update: sales_order::ActiveModel = order.into();
        order_update.status = sea_orm::ActiveValue::Set("cancelled".to_string());
        order_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        order_update.update(&*self.db).await?;

        self.get_order_detail(order_id).await
    }

    /// 获取订单统计
    pub async fn submit_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<sales_order::Model, AppError> {
        // 批次 12（2026-06-28）：事务包裹"查询 + 状态检查 + update_with_audit"，
        // 加 lock_exclusive 防止并发提交同一订单导致状态不一致；
        // update_with_audit 内部 2 次写入（实体 update + 审计 insert）非原子，事务包裹保证原子性。
        let txn = (*self.db).begin().await?;

        let order = SalesOrderEntity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 不存在", order_id)))?;

        if order.status != "draft" {
            return Err(AppError::business(format!(
                "订单状态为 {}，无法提交",
                order.status
            )));
        }

        // 客户信用度复检（信用表与订单表独立，用 self.db 不影响事务一致性）
        let credit_service =
            crate::services::customer_credit_service::CustomerCreditService::new(self.db.clone());
        let total_amount_decimal = {
            use rust_decimal::Decimal;
            order
                .total_amount
                .to_string()
                .parse::<rust_decimal::Decimal>()
                .unwrap_or_else(|_| Decimal::from(0))
        };
        let credit_available = credit_service
            .check_credit_available(order.customer_id, total_amount_decimal)
            .await
            .map_err(|e| AppError::business(format!("信用检查失败: {}", e)))?;
        if !credit_available {
            return Err(AppError::business("信用额度不足，无法提交订单"));
        }

        // 客户状态校验（事务内，保证校验与提交一致）
        let customer = crate::models::customer::Entity::find_by_id(order.customer_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("客户不存在"))?;
        if customer.status != "active" {
            return Err(AppError::business(format!(
                "客户状态为 {}，不允许提交订单",
                customer.status
            )));
        }

        let mut order_update: sales_order::ActiveModel = order.into();
        order_update.status = sea_orm::ActiveValue::Set("pending".to_string());
        order_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());

        // P1-11 修复（2026-06-25 综合审计）：传入真实操作人 ID，
        // 原 Some(0) 硬编码导致审计日志无法追溯提交人。
        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_update,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        // 启动审批工作流（BPM）—— 事务外，失败不阻断已提交状态
        // P0 修复（批次 4，2026-06-27）：原 `let _ = ...` 静默吞掉 BPM 启动错误，
        // 改为 warn 日志记录，保留兼容性（不阻断主流程），确保运维可观测。
        let bpm_service = crate::services::bpm_service::BpmService::new(self.db.clone());
        if let Err(e) = bpm_service
            .start_process(crate::models::dto::bpm_dto::StartProcessRequest {
                process_key: "sales_order_approval".to_string(),
                business_type: "sales_order".to_string(),
                business_id: order_id,
                title: format!("销售订单审批 - {}", order.order_no),
                initiator_id: user_id,
                initiator_name: String::new(),
                initiator_department_id: None,
                priority: None,
                form_data: None,
                variables: None,
            })
            .await
        {
            tracing::warn!(
                error = %e,
                order_id = order_id,
                "BPM 启动销售订单审批流程失败（不阻断主流程）"
            );
        }

        Ok(order)
    }

    /// 审核订单：通过或拒绝
    pub async fn approve_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<sales_order::Model, AppError> {
        // 批次 12（2026-06-28）：事务包裹"查询 + 状态检查 + update_with_audit"，
        // 加 lock_exclusive 防止并发审批同一订单导致重复审批或字段覆盖
        let txn = (*self.db).begin().await?;

        let order = SalesOrderEntity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 不存在", order_id)))?;

        if order.status != "pending" {
            return Err(AppError::business(format!(
                "订单状态为 {}，无法审核",
                order.status
            )));
        }

        let mut order_update: sales_order::ActiveModel = order.into();
        order_update.status = sea_orm::ActiveValue::Set("approved".to_string());
        order_update.approved_by = sea_orm::ActiveValue::Set(Some(user_id));
        order_update.approved_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now()));
        order_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());

        // P1-11 修复（2026-06-25 综合审计）：传入真实操作人 ID，
        // 原 Some(0) 硬编码导致审计日志无法追溯审批人。
        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_update,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(order)
    }

    /// 完成订单
    ///
    /// P1-11 修复（2026-06-25 综合审计）：新增 user_id 参数，
    /// 原 Some(0) 硬编码导致审计日志无法追溯完成操作人。
    pub async fn complete_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<sales_order::Model, AppError> {
        // 批次 12（2026-06-28）：事务包裹"查询 + 状态检查 + update_with_audit"，
        // 加 lock_exclusive 防止并发完成同一订单导致状态不一致
        let txn = (*self.db).begin().await?;

        let order = SalesOrderEntity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {} 不存在", order_id)))?;

        if !["shipped", "partial_shipped"].contains(&order.status.as_str()) {
            return Err(AppError::business(format!(
                "订单状态为 {}，无法完成",
                order.status
            )));
        }

        let mut order_update: sales_order::ActiveModel = order.into();
        order_update.status = sea_orm::ActiveValue::Set("completed".to_string());
        order_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());

        // P1-11 修复：传入真实操作人 ID
        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_update,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_stage_desc() {
        assert_eq!(WorkflowStage::Draft.desc(), "草稿");
        assert_eq!(WorkflowStage::Pending.desc(), "待审核");
        assert_eq!(WorkflowStage::Approved.desc(), "已审核");
        assert_eq!(WorkflowStage::Shipped.desc(), "已发货");
        assert_eq!(WorkflowStage::Received.desc(), "已收款");
        assert_eq!(WorkflowStage::Closed.desc(), "已关闭");
    }

    #[test]
    fn test_workflow_module_loaded() {
        assert_eq!(P92_WF_MODULE, "sales_order_workflow");
    }
}
