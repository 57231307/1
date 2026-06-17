// service.rs - gRPC service 实现
// 4 个 RPC 方法：
// 1. SendNotification - 发送单条通知
// 2. BatchSend - 批量发送
// 3. ListUserNotifications - 列出用户通知
// 4. MarkAsRead - 标记已读
//
// 关键设计：
// - 多租户隔离：每个方法接受 tenant_id 并传递给 repository 层
// - 错误处理：自定义 ServiceError 转换为 tonic::Status
// - 输入校验：使用 NewNotification::validate()

use crate::model::NewNotification;
use crate::repository;
use crate::proto::{
    notification_service_server::NotificationService, BatchSendRequest, BatchSendResponse,
    ListRequest, ListResponse, MarkAsReadRequest, MarkAsReadResponse, NotificationItem,
    SendNotificationRequest, SendNotificationResponse,
};
use sqlx::PgPool;
use std::sync::Arc;
use tonic::{Request, Response, Status};

/// NotificationServiceImpl - gRPC service 实现
pub struct NotificationServiceImpl {
    pool: Arc<PgPool>,
}

impl NotificationServiceImpl {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

#[tonic::async_trait]
impl NotificationService for NotificationServiceImpl {
    /// 发送单条通知
    async fn send_notification(
        &self,
        request: Request<SendNotificationRequest>,
    ) -> Result<Response<SendNotificationResponse>, Status> {
        let req = request.into_inner();

        // 输入校验
        if req.tenant_id <= 0 {
            return Err(Status::invalid_argument("租户 ID 无效"));
        }
        if req.user_id <= 0 {
            return Err(Status::invalid_argument("用户 ID 无效"));
        }

        let notif = NewNotification {
            tenant_id: req.tenant_id,
            user_id: req.user_id,
            title: req.title,
            content: req.content,
            category: if req.category.is_empty() {
                "system".to_string()
            } else {
                req.category
            },
            priority: req.priority as i16,
        };

        if let Err(e) = notif.validate() {
            return Err(Status::invalid_argument(e));
        }

        // 持久化
        let id = repository::insert_notification(&self.pool, &notif)
            .await
            .map_err(|e| Status::internal(format!("发送通知失败: {}", e)))?;

        tracing::info!(
            "通知发送成功：id={}, tenant_id={}, user_id={}",
            id,
            notif.tenant_id,
            notif.user_id
        );

        Ok(Response::new(SendNotificationResponse {
            id,
            status: "success".to_string(),
        }))
    }

    /// 批量发送通知
    async fn batch_send(
        &self,
        request: Request<BatchSendRequest>,
    ) -> Result<Response<BatchSendResponse>, Status> {
        let req = request.into_inner();

        let mut notifs = Vec::with_capacity(req.items.len());
        for item in req.items {
            if item.tenant_id <= 0 {
                return Err(Status::invalid_argument("租户 ID 无效"));
            }
            notifs.push(NewNotification {
                tenant_id: item.tenant_id,
                user_id: item.user_id,
                title: item.title,
                content: item.content,
                category: item.category,
                priority: item.priority as i16,
            });
        }

        let count = repository::batch_insert(&self.pool, &notifs)
            .await
            .map_err(|e| Status::internal(format!("批量发送失败: {}", e)))?;

        Ok(Response::new(BatchSendResponse {
            count: count as i32,
            status: "success".to_string(),
        }))
    }

    /// 列出用户通知
    async fn list_user_notifications(
        &self,
        request: Request<ListRequest>,
    ) -> Result<Response<ListResponse>, Status> {
        let req = request.into_inner();

        if req.tenant_id <= 0 {
            return Err(Status::invalid_argument("租户 ID 无效"));
        }

        let limit = if req.limit <= 0 { 20 } else { req.limit.min(100) };
        let offset = if req.offset < 0 { 0 } else { req.offset };

        let rows = repository::list_by_user(&self.pool, req.tenant_id, req.user_id, limit, offset)
            .await
            .map_err(|e| Status::internal(format!("查询通知失败: {}", e)))?;

        let total = repository::count_by_user(&self.pool, req.tenant_id, req.user_id)
            .await
            .map_err(|e| Status::internal(format!("统计失败: {}", e)))?;

        let items: Vec<NotificationItem> = rows
            .into_iter()
            .map(|r| NotificationItem {
                id: r.id,
                tenant_id: r.tenant_id,
                user_id: r.user_id,
                title: r.title,
                content: r.content,
                category: r.category,
                priority: r.priority as i32,
                is_read: r.is_read,
                created_at: r.created_at.to_rfc3339(),
            })
            .collect();

        Ok(Response::new(ListResponse {
            items,
            total: total as i32,
        }))
    }

    /// 标记已读
    async fn mark_as_read(
        &self,
        request: Request<MarkAsReadRequest>,
    ) -> Result<Response<MarkAsReadResponse>, Status> {
        let req = request.into_inner();

        if req.tenant_id <= 0 {
            return Err(Status::invalid_argument("租户 ID 无效"));
        }
        if req.id <= 0 {
            return Err(Status::invalid_argument("通知 ID 无效"));
        }

        let success = repository::mark_as_read(&self.pool, req.id, req.tenant_id)
            .await
            .map_err(|e| Status::internal(format!("标记已读失败: {}", e)))?;

        Ok(Response::new(MarkAsReadResponse { success }))
    }
}
