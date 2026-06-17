//! 采购合同/审批工作流服务（po/contract）
//!
//! 包含采购订单的提交、审批、拒绝等审批工作流方法。
//! 拆分自原 `purchase_order_service.rs`。

use crate::models::{purchase_order, purchase_order_item, status};
use crate::utils::error::AppError;
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};

use super::order::PurchaseOrderService;

impl PurchaseOrderService {
    /// 提交采购订单
    pub async fn submit_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        // 1. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态
        if order.order_status != status::purchase_order::DRAFT
            && order.order_status != status::purchase_order::REJECTED
        {
            return Err(AppError::business(format!(
                "订单状态不允许提交，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 检查权限
        if order.created_by != user_id {
            return Err(AppError::permission_denied(
                "只能提交自己创建的订单".to_string(),
            ));
        }

        // 4. 检查是否有明细
        let item_count = purchase_order_item::Entity::find()
            .filter(purchase_order_item::Column::OrderId.eq(order_id))
            .count(&*self.db)
            .await?;

        if item_count == 0 {
            return Err(AppError::business("订单至少需要一行明细"));
        }

        // 5. 更新状态为 PENDING_APPROVAL
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.order_status = Set(status::purchase_order::PENDING_APPROVAL.to_string());
        order_active.updated_at = Set(Utc::now());
        order_active.updated_by = Set(Some(user_id));

        let updated_order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            order_active,
            Some(0),
        )
        .await?;

        // 6. 挂载 BPM 引擎
        let bpm_service = crate::services::bpm_service::BpmService::new(self.db.clone());
        let req = crate::models::dto::bpm_dto::StartProcessRequest {
            process_key: "purchase_order_approval".to_string(),
            business_type: "purchase_order".to_string(),
            business_id: order_id,
            title: format!("采购订单审批 - {}", updated_order.order_no),
            initiator_id: user_id,
            initiator_name: String::new(),
            initiator_department_id: None,
            priority: None,
            form_data: None,
            variables: None,
        };
        // 忽略找不到模板的错误，为了兼容旧数据
        let _ = bpm_service.start_process(req).await;

        Ok(updated_order)
    }

    /// 审批采购订单
    pub async fn approve_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        // 1. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态 - 只有待审批状态的订单才能审批
        if order.order_status != status::purchase_order::PENDING_APPROVAL {
            return Err(AppError::business(format!(
                "订单状态不允许审批，当前状态：{}，需要状态：PENDING_APPROVAL",
                order.order_status
            )));
        }

        // 3. 更新状态
        let now = chrono::Utc::now();
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.order_status = Set(status::purchase_order::APPROVED.to_string());
        order_active.approved_by = Set(Some(user_id));
        order_active.approved_at = Set(Some(now));
        order_active.updated_by = Set(Some(user_id));
        order_active.updated_at = Set(now);

        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            order_active,
            Some(0),
        )
        .await?;

        Ok(order)
    }

    /// 拒绝采购订单
    pub async fn reject_order(
        &self,
        order_id: i32,
        reason: String,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        // 1. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态 - 只有待审批状态的订单才能拒绝
        if order.order_status != status::purchase_order::PENDING_APPROVAL {
            return Err(AppError::business(format!(
                "订单状态不允许拒绝，当前状态：{}，需要状态：PENDING_APPROVAL",
                order.order_status
            )));
        }

        // 3. 更新状态
        let now = chrono::Utc::now();
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.order_status = Set(status::purchase_order::REJECTED.to_string());
        order_active.rejected_reason = Set(Some(reason));
        order_active.updated_by = Set(Some(user_id));
        order_active.updated_at = Set(now);

        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            order_active,
            Some(0),
        )
        .await?;

        Ok(order)
    }
}
