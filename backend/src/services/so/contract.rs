//! 销售合同/审批工作流服务（so/contract）
//!
//! 包含销售订单的提交、审批、完成、拒绝等审批工作流方法。
//! 拆分自原 `sales_service.rs`。

use crate::models::{customer, sales_order};
use crate::services::so::order::SalesService;
use crate::utils::error::AppError;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, Set};
use std::sync::Arc;

impl SalesService {
    /// 提交销售订单
    pub async fn submit_order(&self, order_id: i32, user_id: i32) -> Result<(), AppError> {
        let order = sales_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("订单不存在"))?;

        if order.status != "draft" {
            return Err(AppError::business(format!(
                "订单状态为{}，不允许提交",
                order.status
            )));
        }

        // 客户信用度复检
        let credit_service =
            crate::services::customer_credit_service::CustomerCreditService::new(self.db.clone());
        let total_amount_bigdecimal = {
            use bigdecimal::BigDecimal;
            BigDecimal::parse_bytes(order.total_amount.to_string().as_bytes(), 10)
                .unwrap_or_else(|| BigDecimal::from(0))
        };
        let credit_available = credit_service
            .check_credit_available(order.customer_id, total_amount_bigdecimal)
            .await
            .map_err(|e| AppError::business(format!("信用检查失败: {}", e)))?;

        if !credit_available {
            return Err(AppError::business("信用额度不足，无法提交订单"));
        }

        // 检查客户状态
        let customer = customer::Entity::find_by_id(order.customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("客户不存在"))?;

        if customer.status != "active" {
            return Err(AppError::business(format!(
                "客户状态为{}，不允许提交订单",
                customer.status
            )));
        }

        // 更新订单状态
        let mut order_update: sales_order::ActiveModel = order.into();
        order_update.status = Set("pending".to_string());
        order_update.updated_at = Set(chrono::Utc::now());
        order_update.update(&*self.db).await?;

        // 启动审批工作流（BPM）
        let bpm_service = crate::services::bpm_service::BpmService::new(self.db.clone());
        let _ = bpm_service
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
            .await;
        Ok(())
    }

    /// 审批销售订单
    pub async fn approve_order(&self, order_id: i32, user_id: i32) -> Result<(), AppError> {
        let order = sales_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("订单不存在"))?;

        if order.status != "pending" {
            return Err(AppError::business(format!(
                "订单状态为{}，不允许审批",
                order.status
            )));
        }

        // 检查客户是否仍处于正常状态
        let customer = customer::Entity::find_by_id(order.customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("客户不存在"))?;

        if customer.status != "active" {
            return Err(AppError::business(format!(
                "客户状态为{}，不允许审批通过",
                customer.status
            )));
        }

        let mut order_update: sales_order::ActiveModel = order.into();
        order_update.status = Set("approved".to_string());
        order_update.approved_by = Set(Some(user_id));
        order_update.approved_at = Set(Some(chrono::Utc::now()));
        order_update.updated_at = Set(chrono::Utc::now());
        order_update.update(&*self.db).await?;
        Ok(())
    }

    /// 完成销售订单
    pub async fn complete_order(&self, order_id: i32, _user_id: i32) -> Result<(), AppError> {
        let order = sales_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("订单不存在"))?;

        if order.status != "shipped" && order.status != "partial_shipped" {
            return Err(AppError::business(format!(
                "订单状态为{}，不允许标记完成",
                order.status
            )));
        }

        let mut order_update: sales_order::ActiveModel = order.into();
        order_update.status = Set("completed".to_string());
        order_update.updated_at = Set(chrono::Utc::now());
        order_update.update(&*self.db).await?;
        Ok(())
    }

    /// 拒绝销售订单
    pub async fn reject_order(
        &self,
        order_id: i32,
        reason: String,
        _user_id: i32,
    ) -> Result<(), AppError> {
        let order = sales_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("订单不存在"))?;

        if order.status != "pending" {
            return Err(AppError::business(format!(
                "订单状态为{}，不允许拒绝",
                order.status
            )));
        }

        let txn = (*self.db).begin().await?;

        // 释放库存预留
        self.release_reservations(order_id, &txn).await?;

        let mut order_update: sales_order::ActiveModel = order.into();
        order_update.status = Set("rejected".to_string());
        order_update.notes = Set(Some(reason));
        order_update.updated_at = Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_update,
            Some(0),
        )
        .await?;

        txn.commit().await?;
        Ok(())
    }
}

/// 引用 Arc 别名
#[allow(dead_code)]
pub(crate) type DbArc = Arc<DatabaseConnection>;
