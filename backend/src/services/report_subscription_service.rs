//! 报表订阅 Service
//!
//! 提供报表订阅的CRUD操作功能

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::models::report_subscription::{
    ActiveModel, Entity as ReportSubscriptionEntity, Model as ReportSubscriptionModel,
};
use crate::utils::error::AppError;

/// 创建订阅请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubscriptionRequest {
    pub name: String,
    pub template_id: i32,
    pub frequency: String,
    pub recipients: Vec<String>,
    pub export_format: Option<String>,
    pub is_enabled: Option<bool>,
}

/// 更新订阅请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSubscriptionRequest {
    pub name: Option<String>,
    pub frequency: Option<String>,
    pub recipients: Option<Vec<String>>,
    pub export_format: Option<String>,
    pub is_enabled: Option<bool>,
}

/// 订阅查询参数
#[derive(Debug, Clone, Deserialize)]
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
        tenant_id: i32,
        user_id: i32,
        req: CreateSubscriptionRequest,
    ) -> Result<ReportSubscriptionModel, AppError> {
        let now = Utc::now();

        // 计算下次执行时间
        let next_run = match req.frequency.as_str() {
            "DAILY" => Some(now + chrono::Duration::days(1)),
            "WEEKLY" => Some(now + chrono::Duration::weeks(1)),
            "MONTHLY" => Some(now + chrono::Duration::days(30)),
            _ => return Err(AppError::ValidationError("无效的订阅频率".to_string())),
        };

        let recipients_json = serde_json::to_value(&req.recipients)
            .map_err(|e| AppError::ValidationError(format!("收件人格式错误: {}", e)))?;

        let active_model = ActiveModel {
            id: Default::default(),
            tenant_id: Set(tenant_id),
            name: Set(req.name),
            template_id: Set(req.template_id),
            frequency: Set(req.frequency),
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

        let model = active_model
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(model)
    }

    /// 获取订阅详情
    pub async fn get_by_id(&self, id: i32) -> Result<Option<ReportSubscriptionModel>, AppError> {
        let model = ReportSubscriptionEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

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
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("订阅不存在".to_string()))?;

        let mut active_model: ActiveModel = model.into();

        if let Some(name) = req.name {
            active_model.name = Set(name);
        }
        if let Some(frequency) = req.frequency {
            active_model.frequency = Set(frequency);
        }
        if let Some(recipients) = req.recipients {
            let recipients_json = serde_json::to_value(&recipients)
                .map_err(|e| AppError::ValidationError(format!("收件人格式错误: {}", e)))?;
            active_model.recipients = Set(recipients_json);
        }
        if let Some(export_format) = req.export_format {
            active_model.export_format = Set(export_format);
        }
        if let Some(is_enabled) = req.is_enabled {
            active_model.is_enabled = Set(is_enabled);
        }

        active_model.updated_at = Set(Utc::now());

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(updated)
    }

    /// 删除订阅（软删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = ReportSubscriptionEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("订阅不存在".to_string()))?;

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set("INACTIVE".to_string());
        active_model.is_enabled = Set(false);
        active_model.updated_at = Set(Utc::now());

        active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// 启用/禁用订阅
    pub async fn toggle(&self, id: i32, enabled: bool) -> Result<ReportSubscriptionModel, AppError> {
        let model = ReportSubscriptionEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("订阅不存在".to_string()))?;

        let mut active_model: ActiveModel = model.into();
        active_model.is_enabled = Set(enabled);
        active_model.updated_at = Set(Utc::now());

        if enabled {
            // 重新计算下次执行时间
            let now = Utc::now();
            let frequency = if let sea_orm::ActiveValue::Set(ref v) = active_model.frequency {
                v.clone()
            } else {
                return Err(AppError::BadRequest("frequency is required".to_string()));
            };
            let next_run = match frequency.as_str() {
                "DAILY" => Some(now + chrono::Duration::days(1)),
                "WEEKLY" => Some(now + chrono::Duration::weeks(1)),
                "MONTHLY" => Some(now + chrono::Duration::days(30)),
                _ => None,
            };
            active_model.next_run_at = Set(next_run);
        }

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(updated)
    }

    /// 查询订阅列表
    pub async fn list(
        &self,
        tenant_id: i32,
        query: SubscriptionQuery,
    ) -> Result<(Vec<ReportSubscriptionModel>, u64), AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20);

        let mut select = ReportSubscriptionEntity::find()
            .filter(crate::models::report_subscription::Column::TenantId.eq(tenant_id))
            .filter(crate::models::report_subscription::Column::Status.eq("ACTIVE"));

        if let Some(template_id) = query.template_id {
            select = select.filter(
                crate::models::report_subscription::Column::TemplateId.eq(template_id),
            );
        }

        if let Some(frequency) = query.frequency {
            select = select.filter(
                crate::models::report_subscription::Column::Frequency.eq(frequency),
            );
        }

        if let Some(is_enabled) = query.is_enabled {
            select = select.filter(
                crate::models::report_subscription::Column::IsEnabled.eq(is_enabled),
            );
        }

        let total = select
            .clone()
            .count(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let items = select
            .order_by_desc(crate::models::report_subscription::Column::CreatedAt)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok((items, total))
    }

    /// 获取用户的订阅列表
    pub async fn list_by_user(
        &self,
        tenant_id: i32,
        user_id: i32,
    ) -> Result<Vec<ReportSubscriptionModel>, AppError> {
        let items = ReportSubscriptionEntity::find()
            .filter(crate::models::report_subscription::Column::TenantId.eq(tenant_id))
            .filter(crate::models::report_subscription::Column::CreatedBy.eq(user_id))
            .filter(crate::models::report_subscription::Column::Status.eq("ACTIVE"))
            .order_by_desc(crate::models::report_subscription::Column::CreatedAt)
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(items)
    }

    /// 手动触发订阅执行
    pub async fn trigger(&self, id: i32) -> Result<(), AppError> {
        let model = ReportSubscriptionEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("订阅不存在".to_string()))?;

        // 立即将下次执行时间设为现在
        let mut active_model: ActiveModel = model.into();
        active_model.next_run_at = Set(Some(Utc::now()));
        active_model.updated_at = Set(Utc::now());

        active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}
