//! 业务事件通知服务
//!
//! 将业务事件与通知系统集成，在关键业务节点自动触发通知
//! 支持订单状态变更、审批提醒、库存预警等业务场景

use crate::models::notification::{NotificationPriority, NotificationType};
use crate::services::email_service::{EmailService, EmailTemplate};
use crate::services::notification_service::{CreateNotificationRequest, NotificationService};
use crate::utils::error::AppError;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// 业务事件通知服务
pub struct EventNotificationService {
    notification_service: NotificationService,
    email_service: Option<Arc<EmailService>>,
}

impl EventNotificationService {
    /// 创建服务实例（仅站内通知）
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            notification_service: NotificationService::new(db),
            email_service: None,
        }
    }

    /// 创建服务实例（含邮件通知）
    pub fn with_email(db: Arc<DatabaseConnection>, email_service: Arc<EmailService>) -> Self {
        Self {
            notification_service: NotificationService::new(db),
            email_service: Some(email_service),
        }
    }

    // ========== 订单相关通知 ==========

    /// 发送邮件通知（辅助方法）
    async fn send_email_notification(
        &self,
        to_email: &str,
        subject: &str,
        html_content: String,
    ) {
        if let Some(email_service) = &self.email_service {
            let _ = email_service
                .send_html_email(
                    vec![to_email.to_string()],
                    subject.to_string(),
                    html_content,
                )
                .await;
        }
    }

    /// 订单提交通知
    pub async fn notify_order_submitted(
        &self,
        user_id: i32,
        order_no: &str,
        order_id: i32,
    ) -> Result<(), AppError> {
        self.notification_service
            .create_notification(CreateNotificationRequest {
                user_id,
                notification_type: NotificationType::Internal,
                title: "订单已提交".to_string(),
                content: format!("订单 {} 已提交，等待审批", order_no),
                priority: NotificationPriority::Normal,
                business_type: Some("ORDER".to_string()),
                business_id: Some(order_id),
                action_url: Some(format!("/sales/orders/{}", order_id)),
                sender_id: Some(0),
                sender_name: Some("系统".to_string()),
            })
            .await?;

        // 发送邮件通知
        let html = EmailTemplate::order_notification(order_no, "已提交", "/sales/orders");
        self.send_email_notification("user@example.com", "订单状态更新", html).await;

        Ok(())
    }

    /// 订单审批通过通知
    pub async fn notify_order_approved(
        &self,
        user_id: i32,
        order_no: &str,
        order_id: i32,
        approver_name: &str,
    ) -> Result<(), AppError> {
        self.notification_service
            .create_notification(CreateNotificationRequest {
                user_id,
                notification_type: NotificationType::Internal,
                title: "订单审批通过".to_string(),
                content: format!("订单 {} 已通过 {} 的审批", order_no, approver_name),
                priority: NotificationPriority::Normal,
                business_type: Some("ORDER".to_string()),
                business_id: Some(order_id),
                action_url: Some(format!("/sales/orders/{}", order_id)),
                sender_id: Some(0),
                sender_name: Some(approver_name.to_string()),
            })
            .await?;
        Ok(())
    }

    /// 订单发货通知
    pub async fn notify_order_shipped(
        &self,
        user_id: i32,
        order_no: &str,
        order_id: i32,
    ) -> Result<(), AppError> {
        self.notification_service
            .create_notification(CreateNotificationRequest {
                user_id,
                notification_type: NotificationType::Internal,
                title: "订单已发货".to_string(),
                content: format!("订单 {} 已发货，请注意查收", order_no),
                priority: NotificationPriority::High,
                business_type: Some("ORDER".to_string()),
                business_id: Some(order_id),
                action_url: Some(format!("/sales/orders/{}", order_id)),
                sender_id: Some(0),
                sender_name: Some("系统".to_string()),
            })
            .await?;
        Ok(())
    }

    /// 订单完成通知
    pub async fn notify_order_completed(
        &self,
        user_id: i32,
        order_no: &str,
        order_id: i32,
    ) -> Result<(), AppError> {
        self.notification_service
            .create_notification(CreateNotificationRequest {
                user_id,
                notification_type: NotificationType::Internal,
                title: "订单已完成".to_string(),
                content: format!("订单 {} 已完成，感谢您的合作", order_no),
                priority: NotificationPriority::Normal,
                business_type: Some("ORDER".to_string()),
                business_id: Some(order_id),
                action_url: Some(format!("/sales/orders/{}", order_id)),
                sender_id: Some(0),
                sender_name: Some("系统".to_string()),
            })
            .await?;
        Ok(())
    }

    // ========== 审批相关通知 ==========

    /// 待审批任务通知
    pub async fn notify_pending_approval(
        &self,
        approver_user_id: i32,
        task_title: &str,
        applicant_name: &str,
        business_type: &str,
        business_id: i32,
    ) -> Result<(), AppError> {
        self.notification_service
            .create_notification(CreateNotificationRequest {
                user_id: approver_user_id,
                notification_type: NotificationType::Internal,
                title: "待审批任务".to_string(),
                content: format!("{} 提交的 '{}' 需要您审批", applicant_name, task_title),
                priority: NotificationPriority::High,
                business_type: Some(business_type.to_string()),
                business_id: Some(business_id),
                action_url: Some(format!("/approvals/{}", business_id)),
                sender_id: Some(0),
                sender_name: Some(applicant_name.to_string()),
            })
            .await?;
        Ok(())
    }

    /// 审批结果通知
    pub async fn notify_approval_result(
        &self,
        user_id: i32,
        task_title: &str,
        approved: bool,
        approver_name: &str,
        comment: Option<&str>,
    ) -> Result<(), AppError> {
        let status = if approved { "通过" } else { "拒绝" };
        let mut content = format!("您的 '{}' 申请已被 {} {}", task_title, approver_name, status);
        if let Some(c) = comment {
            content.push_str(&format!("，审批意见：{}", c));
        }

        self.notification_service
            .create_notification(CreateNotificationRequest {
                user_id,
                notification_type: NotificationType::Internal,
                title: format!("审批{}", status),
                content,
                priority: NotificationPriority::Normal,
                business_type: Some("APPROVAL".to_string()),
                business_id: None,
                action_url: None,
                sender_id: Some(0),
                sender_name: Some(approver_name.to_string()),
            })
            .await?;
        Ok(())
    }

    // ========== 库存相关通知 ==========

    /// 库存预警通知
    pub async fn notify_inventory_alert(
        &self,
        user_id: i32,
        product_name: &str,
        product_id: i32,
        current_stock: &str,
        threshold: &str,
    ) -> Result<(), AppError> {
        self.notification_service
            .create_notification(CreateNotificationRequest {
                user_id,
                notification_type: NotificationType::Internal,
                title: "库存预警".to_string(),
                content: format!(
                    "产品 '{}' 当前库存 {}，已低于预警阈值 {}",
                    product_name, current_stock, threshold
                ),
                priority: NotificationPriority::Urgent,
                business_type: Some("INVENTORY".to_string()),
                business_id: Some(product_id),
                action_url: Some(format!("/inventory/stock/{}", product_id)),
                sender_id: Some(0),
                sender_name: Some("系统".to_string()),
            })
            .await?;
        Ok(())
    }

    /// 库存盘点提醒
    pub async fn notify_inventory_count(
        &self,
        user_id: i32,
        warehouse_name: &str,
        count_id: i32,
    ) -> Result<(), AppError> {
        self.notification_service
            .create_notification(CreateNotificationRequest {
                user_id,
                notification_type: NotificationType::Internal,
                title: "库存盘点提醒".to_string(),
                content: format!("{} 的库存盘点任务已创建，请及时完成", warehouse_name),
                priority: NotificationPriority::Normal,
                business_type: Some("INVENTORY".to_string()),
                business_id: Some(count_id),
                action_url: Some(format!("/inventory/counts/{}", count_id)),
                sender_id: Some(0),
                sender_name: Some("系统".to_string()),
            })
            .await?;
        Ok(())
    }

    // ========== 采购相关通知 ==========

    /// 采购订单到货通知
    pub async fn notify_purchase_arrived(
        &self,
        user_id: i32,
        order_no: &str,
        order_id: i32,
        warehouse_name: &str,
    ) -> Result<(), AppError> {
        self.notification_service
            .create_notification(CreateNotificationRequest {
                user_id,
                notification_type: NotificationType::Internal,
                title: "采购订单到货".to_string(),
                content: format!(
                    "采购订单 {} 的货物已到达 {}，请安排入库",
                    order_no, warehouse_name
                ),
                priority: NotificationPriority::High,
                business_type: Some("PURCHASE".to_string()),
                business_id: Some(order_id),
                action_url: Some(format!("/purchases/orders/{}", order_id)),
                sender_id: Some(0),
                sender_name: Some("系统".to_string()),
            })
            .await?;
        Ok(())
    }

    // ========== 财务相关通知 ==========

    /// 应收账款到期提醒
    pub async fn notify_ar_due(
        &self,
        user_id: i32,
        customer_name: &str,
        amount: &str,
        due_date: &str,
        invoice_id: i32,
    ) -> Result<(), AppError> {
        self.notification_service
            .create_notification(CreateNotificationRequest {
                user_id,
                notification_type: NotificationType::Internal,
                title: "应收账款到期提醒".to_string(),
                content: format!(
                    "客户 {} 的应收账款 {} 将于 {} 到期，请及时跟进",
                    customer_name, amount, due_date
                ),
                priority: NotificationPriority::High,
                business_type: Some("FINANCE".to_string()),
                business_id: Some(invoice_id),
                action_url: Some(format!("/finance/invoices/{}", invoice_id)),
                sender_id: Some(0),
                sender_name: Some("系统".to_string()),
            })
            .await?;
        Ok(())
    }

    /// 付款申请通知
    pub async fn notify_payment_request(
        &self,
        approver_user_id: i32,
        request_no: &str,
        amount: &str,
        supplier_name: &str,
        request_id: i32,
    ) -> Result<(), AppError> {
        self.notification_service
            .create_notification(CreateNotificationRequest {
                user_id: approver_user_id,
                notification_type: NotificationType::Internal,
                title: "付款申请待审批".to_string(),
                content: format!(
                    "供应商 {} 的付款申请 {}，金额 {} 需要您审批",
                    supplier_name, request_no, amount
                ),
                priority: NotificationPriority::High,
                business_type: Some("FINANCE".to_string()),
                business_id: Some(request_id),
                action_url: Some(format!("/finance/payment-requests/{}", request_id)),
                sender_id: Some(0),
                sender_name: Some("系统".to_string()),
            })
            .await?;
        Ok(())
    }

    // ========== 系统通知 ==========

    /// 系统公告
    pub async fn send_system_announcement(
        &self,
        user_ids: Vec<i32>,
        title: &str,
        content: &str,
    ) -> Result<(), AppError> {
        for user_id in user_ids {
            self.notification_service
                .create_notification(CreateNotificationRequest {
                    user_id,
                    notification_type: NotificationType::System,
                    title: title.to_string(),
                    content: content.to_string(),
                    priority: NotificationPriority::Normal,
                    business_type: Some("SYSTEM".to_string()),
                    business_id: None,
                    action_url: None,
                    sender_id: Some(0),
                    sender_name: Some("系统管理员".to_string()),
                })
                .await?;
        }
        Ok(())
    }

    /// 发送通知给多个用户
    pub async fn notify_multiple_users(
        &self,
        user_ids: Vec<i32>,
        title: String,
        content: String,
        priority: NotificationPriority,
        business_type: Option<String>,
        business_id: Option<i32>,
        action_url: Option<String>,
    ) -> Result<(), AppError> {
        for user_id in user_ids {
            self.notification_service
                .create_notification(CreateNotificationRequest {
                    user_id,
                    notification_type: NotificationType::Internal,
                    title: title.clone(),
                    content: content.clone(),
                    priority: priority.clone(),
                    business_type: business_type.clone(),
                    business_id,
                    action_url: action_url.clone(),
                    sender_id: Some(0),
                    sender_name: Some("系统".to_string()),
                })
                .await?;
        }
        Ok(())
    }
}
