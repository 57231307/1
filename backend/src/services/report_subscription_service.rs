//! 报表订阅 Service
//!
//! 提供报表订阅的CRUD操作功能

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::Validate;

use sea_orm::DatabaseConnection;

use crate::utils::pagination::paginate_with_total;

use crate::models::report_subscription::{
    ActiveModel, Entity as ReportSubscriptionEntity, Model as ReportSubscriptionModel,
};
use crate::utils::error::AppError;

/// 创建订阅请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateSubscriptionRequest {
    pub name: String,
    pub template_id: i32,
    pub frequency: String,
    pub recipients: Vec<String>,
    pub parameters: Option<serde_json::Value>,
    pub export_format: Option<String>,
    pub is_enabled: Option<bool>,
}

/// 更新订阅请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateSubscriptionRequest {
    pub name: Option<String>,
    pub frequency: Option<String>,
    pub recipients: Option<Vec<String>>,
    pub export_format: Option<String>,
    pub is_enabled: Option<bool>,
}

/// 订阅查询参数
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct SubscriptionQuery {
    pub template_id: Option<i32>,
    pub frequency: Option<String>,
    pub is_enabled: Option<bool>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 报表订阅 Service
pub struct ReportSubscriptionService {
    db: Arc<DatabaseConnection>,
}

impl ReportSubscriptionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建订阅
    pub async fn create(
        &self,
        user_id: i32,
        req: CreateSubscriptionRequest,
    ) -> Result<ReportSubscriptionModel, AppError> {
        let now = Utc::now();

        // 计算下次执行时间
        let next_run = match req.frequency.as_str() {
            "DAILY" => Some(now + chrono::Duration::days(1)),
            "WEEKLY" => Some(now + chrono::Duration::weeks(1)),
            "MONTHLY" => Some(now + chrono::Duration::days(30)),
            _ => return Err(AppError::validation("无效的订阅频率")),
        };

        let recipients_json = serde_json::to_value(&req.recipients)
            .map_err(|e| AppError::validation(format!("收件人格式错误: {}", e)))?;

        let active_model = ActiveModel {
            id: Default::default(),
            name: Set(req.name),
            template_id: Set(req.template_id),
            frequency: Set(req.frequency),
            parameters: Set(req.parameters),
            recipients: Set(recipients_json),
            export_format: Set(req.export_format.unwrap_or_else(|| "pdf".to_string())),
            is_enabled: Set(req.is_enabled.unwrap_or(true)),
            status: Set("ACTIVE".to_string()),
            next_run_at: Set(next_run),
            last_run_at: Set(None),
            last_run_status: Set(None),
            last_run_error: Set(None),
            run_count: Set(0),
            created_by: Set(user_id),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let model = active_model.insert(&*self.db).await?;

        Ok(model)
    }

    /// 获取订阅详情
    pub async fn get_by_id(&self, id: i32) -> Result<Option<ReportSubscriptionModel>, AppError> {
        let model = ReportSubscriptionEntity::find_by_id(id)
            .one(&*self.db)
            .await?;

        Ok(model)
    }

    /// 更新订阅
    pub async fn update(
        &self,
        id: i32,
        req: UpdateSubscriptionRequest,
    ) -> Result<ReportSubscriptionModel, AppError> {
        let model = ReportSubscriptionEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("订阅不存在"))?;

        let mut active_model: ActiveModel = model.into();

        if let Some(name) = req.name {
            active_model.name = Set(name);
        }
        if let Some(frequency) = req.frequency {
            active_model.frequency = Set(frequency);
        }
        if let Some(recipients) = req.recipients {
            let recipients_json = serde_json::to_value(&recipients)
                .map_err(|e| AppError::validation(format!("收件人格式错误: {}", e)))?;
            active_model.recipients = Set(recipients_json);
        }
        if let Some(export_format) = req.export_format {
            active_model.export_format = Set(export_format);
        }
        if let Some(is_enabled) = req.is_enabled {
            active_model.is_enabled = Set(is_enabled);
        }

        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&*self.db).await?;

        Ok(updated)
    }

    /// 删除订阅（软删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = ReportSubscriptionEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("订阅不存在"))?;

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set("INACTIVE".to_string());
        active_model.is_enabled = Set(false);
        active_model.updated_at = Set(Utc::now());

        active_model.update(&*self.db).await?;

        Ok(())
    }

    /// 启用/禁用订阅
    pub async fn toggle(
        &self,
        id: i32,
        enabled: bool,
    ) -> Result<ReportSubscriptionModel, AppError> {
        let model = ReportSubscriptionEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("订阅不存在"))?;

        let mut active_model: ActiveModel = model.into();
        active_model.is_enabled = Set(enabled);
        active_model.updated_at = Set(Utc::now());

        if enabled {
            // 重新计算下次执行时间
            let now = Utc::now();
            let frequency = if let sea_orm::ActiveValue::Set(ref v) = active_model.frequency {
                v.clone()
            } else {
                return Err(AppError::bad_request("frequency is required"));
            };
            let next_run = match frequency.as_str() {
                "DAILY" => Some(now + chrono::Duration::days(1)),
                "WEEKLY" => Some(now + chrono::Duration::weeks(1)),
                "MONTHLY" => Some(now + chrono::Duration::days(30)),
                _ => None,
            };
            active_model.next_run_at = Set(next_run);
        }

        let updated = active_model.update(&*self.db).await?;

        Ok(updated)
    }

    /// 查询订阅列表
    pub async fn list(
        &self,
        query: SubscriptionQuery,
    ) -> Result<(Vec<ReportSubscriptionModel>, u64), AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100); // v10 P1-1 修复：page_size clamp(1,100) 防 DoS

        let mut select = ReportSubscriptionEntity::find()
            .filter(crate::models::report_subscription::Column::Status.eq("ACTIVE"));

        if let Some(template_id) = query.template_id {
            select = select
                .filter(crate::models::report_subscription::Column::TemplateId.eq(template_id));
        }

        if let Some(frequency) = query.frequency {
            select =
                select.filter(crate::models::report_subscription::Column::Frequency.eq(frequency));
        }

        if let Some(is_enabled) = query.is_enabled {
            select =
                select.filter(crate::models::report_subscription::Column::IsEnabled.eq(is_enabled));
        }

        // 批次 256 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 saturating_sub(1) 偏移）
        // 删除独立 count 查询，复用 paginator 的 num_items()，补充 page.clamp(1, 1000) 防 DoS
        let paginator = select
            .order_by_desc(crate::models::report_subscription::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;

        Ok((items, total))
    }

    /// 手动触发订阅执行
    pub async fn trigger(&self, id: i32) -> Result<(), AppError> {
        let model = ReportSubscriptionEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("订阅不存在"))?;

        // 立即将下次执行时间设为现在
        let mut active_model: ActiveModel = model.into();
        active_model.next_run_at = Set(Some(Utc::now()));
        active_model.updated_at = Set(Utc::now());

        active_model.update(&*self.db).await?;

        Ok(())
    }
}
