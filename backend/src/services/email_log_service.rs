//! 邮件发送记录 Service
//!
//! 提供邮件发送记录的持久化和查询功能

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::models::email_log::{ActiveModel, Entity as EmailLogEntity, Model as EmailLogModel};
// 批次 236 v13 P1-1：邮件日志状态常量接入（规则 0）
use crate::models::status::email_log;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;

/// 缺陷 6.2 修复：最大重试次数（超过此值转入 FAILED 死信状态）
pub const MAX_RETRY_COUNT: i32 = 3;

/// 缺陷 6.2 修复：指数退避间隔（秒）— 第 1 次 60s / 第 2 次 300s / 第 3 次 1800s
const BACKOFF_INTERVAL_SECS: &[i64] = &[60, 300, 1800];

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
    /// 缺陷 6.1 修复：HTML 正文（异步队列调度时区分 HTML 与纯文本）
    pub html_content: Option<String>,
    /// 缺陷 6.1 修复：纯文本正文
    pub text_content: Option<String>,
    /// 缺陷 6.3 修复：附件 JSON 数组 [{filename, content_base64, content_type}]
    pub attachments: Option<serde_json::Value>,
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
    pub async fn create(&self, req: CreateEmailLogRequest) -> Result<EmailLogModel, AppError> {
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
            next_retry_at: Set(None),
            attachments: Set(req.attachments),
            html_content: Set(req.html_content),
            text_content: Set(req.text_content),
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

    /// 累加邮件重试计数并按指数退避设置 next_retry_at。
    /// 缺陷 6.2 修复：retry_count >= MAX_RETRY_COUNT 时转入 FAILED 死信状态，不再重试。
    pub async fn increment_retry(&self, id: i32) -> Result<(), AppError> {
        let model = EmailLogEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("邮件记录不存在"))?;

        let new_retry_count = model.retry_count + 1;
        let now = Utc::now();

        // 缺陷 6.2 修复：超过最大重试次数 → FAILED 死信状态
        if new_retry_count >= MAX_RETRY_COUNT {
            let mut active_model: ActiveModel = model.into();
            active_model.retry_count = Set(new_retry_count);
            active_model.status = Set(email_log::FAILED.to_string());
            active_model.next_retry_at = Set(None);
            active_model.updated_at = Set(now);
            active_model.update(&*self.db).await?;
            tracing::warn!(
                email_log_id = id,
                retry_count = new_retry_count,
                max_retries = MAX_RETRY_COUNT,
                "邮件重试次数已达上限，转入 FAILED 死信状态"
            );
            return Ok(());
        }

        // 缺陷 6.2 修复：指数退避 — 第 1 次 60s / 第 2 次 300s / 第 3 次 1800s
        let backoff_idx = (new_retry_count as usize)
            .saturating_sub(1)
            .min(BACKOFF_INTERVAL_SECS.len().saturating_sub(1));
        let backoff_secs = BACKOFF_INTERVAL_SECS[backoff_idx];
        let next_retry_at = now + chrono::Duration::seconds(backoff_secs);

        let mut active_model: ActiveModel = model.into();
        active_model.retry_count = Set(new_retry_count);
        active_model.status = Set(email_log::PENDING.to_string());
        active_model.next_retry_at = Set(Some(next_retry_at));
        active_model.updated_at = Set(now);
        active_model.update(&*self.db).await?;

        tracing::info!(
            email_log_id = id,
            retry_count = new_retry_count,
            backoff_secs,
            next_retry_at = %next_retry_at,
            "邮件重试已调度"
        );
        Ok(())
    }

    /// 缺陷 6.1/6.2 修复：查询待发送邮件（PENDING + next_retry_at 已到或为 NULL + retry_count < MAX）
    /// 供后台 email_queue_worker 调度使用。
    pub async fn list_pending_for_retry(&self, limit: u64) -> Result<Vec<EmailLogModel>, AppError> {
        let now = Utc::now();
        let emails = EmailLogEntity::find()
            .filter(crate::models::email_log::Column::Status.eq(email_log::PENDING))
            .filter(crate::models::email_log::Column::RetryCount.lt(MAX_RETRY_COUNT))
            .filter(
                crate::models::email_log::Column::NextRetryAt
                    .is_null()
                    .or(crate::models::email_log::Column::NextRetryAt.lte(now)),
            )
            .order_by_asc(crate::models::email_log::Column::CreatedAt)
            .limit(limit)
            .all(&*self.db)
            .await?;
        Ok(emails)
    }

    /// 缺陷 6.1 修复：标记邮件为发送中（防止 worker 并发重复发送同一封邮件）
    pub async fn mark_as_sending(&self, id: i32) -> Result<bool, AppError> {
        let now = Utc::now();
        // 乐观锁：仅当当前状态为 PENDING 时才更新为 SENDING
        let result = crate::models::email_log::Entity::update_many()
            .filter(crate::models::email_log::Column::Id.eq(id))
            .filter(crate::models::email_log::Column::Status.eq(email_log::PENDING))
            .set(crate::models::email_log::ActiveModel {
                status: Set("SENDING".to_string()),
                updated_at: Set(now),
                ..Default::default()
            })
            .exec(self.db.as_ref())
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// 获取邮件发送记录详情
    pub async fn get_by_id(&self, id: i32) -> Result<Option<EmailLogModel>, AppError> {
        let model = EmailLogEntity::find_by_id(id).one(&*self.db).await?;

        Ok(model)
    }

    /// 查询邮件发送记录列表
    pub async fn list(&self, query: EmailLogQuery) -> Result<(Vec<EmailLogModel>, u64), AppError> {
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

        // 批次 256 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 saturating_sub(1) 偏移）
        let paginator = select
            .order_by_desc(crate::models::email_log::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;

        Ok((items, total))
    }

    /// 获取发送统计
    pub async fn get_statistics(&self) -> Result<EmailStatistics, AppError> {
        let total = EmailLogEntity::find().count(&*self.db).await?;

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
