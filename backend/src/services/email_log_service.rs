//! 邮件发送记录 Service
//!
//! 提供邮件发送记录的持久化和查询功能

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::models::email_log::{ActiveModel, Entity as EmailLogEntity, Model as EmailLogModel};
// 批次 236 v13 P1-1：邮件日志状态常量接入（规则 0）
use crate::models::status::email_log;
use crate::utils::error::AppError;

/// 创建邮件发送记录请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEmailLogRequest {
    pub user_id: Option<i32>,
    pub recipients: Vec<String>,
    pub cc: Option<Vec<String>>,
    pub bcc: Option<Vec<String>>,
    pub subject: String,
    pub body: Option<String>,
    pub template_id: Option<i32>,
}

/// 邮件发送记录查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct EmailLogQuery {
    pub status: Option<String>,
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 邮件发送记录 Service
pub struct EmailLogService {
    db: Arc<DatabaseConnection>,
}

impl EmailLogService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建邮件发送记录
    pub async fn create(
        &self,
        req: CreateEmailLogRequest,
    ) -> Result<EmailLogModel, AppError> {
        let now = Utc::now();
        let active_model = ActiveModel {
            id: Default::default(),
            user_id: Set(req.user_id),
            recipients: Set(req.recipients.join(",")),
            cc: Set(req.cc.map(|v| v.join(","))),
            bcc: Set(req.bcc.map(|v| v.join(","))),
            subject: Set(req.subject),
            body: Set(req.body),
            template_id: Set(req.template_id),
            status: Set(email_log::PENDING.to_string()),
            error_message: Set(None),
            external_message_id: Set(None),
            sent_at: Set(None),
            retry_count: Set(0),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let model = active_model.insert(&*self.db).await?;

        Ok(model)
    }

    /// 更新邮件发送状态
    pub async fn update_status(
        &self,
        id: i32,
        status: &str,
        error_message: Option<String>,
        external_message_id: Option<String>,
    ) -> Result<EmailLogModel, AppError> {
        let model = EmailLogEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("邮件记录不存在"))?;

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set(status.to_string());
        active_model.error_message = Set(error_message);
        active_model.external_message_id = Set(external_message_id);
        active_model.updated_at = Set(Utc::now());

        if status == email_log::SENT {
            active_model.sent_at = Set(Some(Utc::now()));
        }

        let updated = active_model.update(&*self.db).await?;

        Ok(updated)
    }

    /// 累加邮件重试计数并将状态重置为 PENDING，供重试调度任务识别待重试邮件
    pub async fn increment_retry(&self, id: i32) -> Result<(), AppError> {
        let model = EmailLogEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("邮件记录不存在"))?;

        let retry_count = model.retry_count + 1;
        let mut active_model: ActiveModel = model.into();
        active_model.retry_count = Set(retry_count);
        active_model.status = Set(email_log::PENDING.to_string());
        active_model.updated_at = Set(Utc::now());

        active_model.update(&*self.db).await?;

        Ok(())
    }

    /// 获取邮件发送记录详情
    pub async fn get_by_id(&self, id: i32) -> Result<Option<EmailLogModel>, AppError> {
        let model = EmailLogEntity::find_by_id(id).one(&*self.db).await?;

        Ok(model)
    }

    /// 查询邮件发送记录列表
    pub async fn list(
        &self,
        query: EmailLogQuery,
    ) -> Result<(Vec<EmailLogModel>, u64), AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

        let mut select = EmailLogEntity::find();

        if let Some(status) = query.status {
            select = select.filter(crate::models::email_log::Column::Status.eq(status));
        }

        if let Some(keyword) = query.keyword {
            select = select.filter(
                crate::models::email_log::Column::Subject
                    .contains(&keyword)
                    .or(crate::models::email_log::Column::Recipients.contains(&keyword)),
            );
        }

        let total = select.clone().count(&*self.db).await?;

        let items = select
            .order_by_desc(crate::models::email_log::Column::CreatedAt)
            .paginate(&*self.db, page_size)
            .fetch_page(page.saturating_sub(1))
            .await?;

        Ok((items, total))
    }

    /// 获取发送统计
    pub async fn get_statistics(&self) -> Result<EmailStatistics, AppError> {
        let total = EmailLogEntity::find()
            .count(&*self.db)
            .await?;

        let sent = EmailLogEntity::find()
            .filter(crate::models::email_log::Column::Status.eq(email_log::SENT))
            .count(&*self.db)
            .await?;

        let failed = EmailLogEntity::find()
            .filter(crate::models::email_log::Column::Status.eq(email_log::FAILED))
            .count(&*self.db)
            .await?;

        let pending = EmailLogEntity::find()
            .filter(crate::models::email_log::Column::Status.eq(email_log::PENDING))
            .count(&*self.db)
            .await?;

        Ok(EmailStatistics {
            total: total as i64,
            sent: sent as i64,
            failed: failed as i64,
            pending: pending as i64,
        })
    }
}

/// 邮件发送统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailStatistics {
    pub total: i64,
    pub sent: i64,
    pub failed: i64,
    pub pending: i64,
}
