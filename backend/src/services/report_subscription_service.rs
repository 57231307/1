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
use crate::models::report_template::Entity as ReportTemplateEntity;
use crate::utils::error::AppError;

/// 缺陷 2.3 修复：最大重试次数默认值
const DEFAULT_MAX_RETRIES: i32 = 3;

/// 缺陷 2.3 修复：指数退避间隔表（1min / 5min / 30min）
fn backoff_seconds(retry_count: i32) -> i64 {
    match retry_count {
        0 => 60,
        1 => 300,
        _ => 1800,
    }
}

/// 简易邮箱格式校验（缺陷 2.2 修复：避免引入额外 crate）
fn is_valid_email(email: &str) -> bool {
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return false;
    }
    parts[1].contains('.') && !parts[1].starts_with('.') && !parts[1].ends_with('.')
}

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

        // 缺陷 2.2 修复：校验模板存在且当前用户可见（公开或自己创建）
        let template = ReportTemplateEntity::find_by_id(req.template_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("报表模板 {} 不存在", req.template_id)))?;

        if !template.is_public && template.created_by != user_id {
            return Err(AppError::permission_denied(
                "无权订阅该私有报表模板，仅可订阅公开模板或自己创建的模板",
            ));
        }

        // 缺陷 2.2 修复：校验收件人邮箱格式（防止将敏感报表推送到非法邮箱）
        if req.recipients.is_empty() {
            return Err(AppError::validation("收件人列表不能为空"));
        }
        for email in &req.recipients {
            if !is_valid_email(email) {
                return Err(AppError::validation(format!(
                    "收件人邮箱格式无效: {}",
                    email
                )));
            }
        }

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
            // 缺陷 2.3 修复：初始化重试字段
            retry_count: Set(0),
            max_retries: Set(DEFAULT_MAX_RETRIES),
            next_retry_at: Set(None),
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

    /// 缺陷 2.3 修复：标记订阅执行成功，清零重试计数
    pub async fn mark_run_success(&self, id: i32) -> Result<(), AppError> {
        let model = ReportSubscriptionEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("订阅不存在"))?;

        let now = Utc::now();
        let next_run = model.calculate_next_run();
        let mut active_model: ActiveModel = model.clone().into();
        active_model.last_run_at = Set(Some(now));
        active_model.last_run_status = Set(Some("success".to_string()));
        active_model.last_run_error = Set(None);
        active_model.run_count = Set(model.run_count + 1);
        active_model.retry_count = Set(0);
        active_model.next_retry_at = Set(None);
        active_model.next_run_at = Set(next_run);
        active_model.updated_at = Set(now);
        active_model.update(&*self.db).await?;
        Ok(())
    }

    /// 缺陷 2.3 修复：标记订阅执行失败，按指数退避调度下次重试；超过 max_retries 转入死信状态
    pub async fn mark_run_failed(&self, id: i32, error: String) -> Result<(), AppError> {
        let model = ReportSubscriptionEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("订阅不存在"))?;

        let now = Utc::now();
        let new_retry_count = model.retry_count + 1;
        let max_retries = model.max_retries.max(DEFAULT_MAX_RETRIES);
        let mut active_model: ActiveModel = model.into();
        active_model.last_run_at = Set(Some(now));
        active_model.last_run_status = Set(Some("failed".to_string()));
        active_model.last_run_error = Set(Some(error.clone()));
        active_model.retry_count = Set(new_retry_count);

        if new_retry_count > max_retries {
            // 超过最大重试次数：转入死信状态，停止重试
            active_model.status = Set("DEAD_LETTER".to_string());
            active_model.next_retry_at = Set(None);
            tracing::warn!(
                subscription_id = id,
                retry_count = new_retry_count,
                max_retries,
                error = %error,
                "订阅重试次数超限，已转入死信状态"
            );
        } else {
            // 按指数退避调度下次重试
            let next_retry = now + chrono::Duration::seconds(backoff_seconds(new_retry_count - 1));
            active_model.next_retry_at = Set(Some(next_retry));
            tracing::info!(
                subscription_id = id,
                retry_count = new_retry_count,
                next_retry_at = %next_retry,
                error = %error,
                "订阅执行失败，已调度下次重试"
            );
        }
        active_model.updated_at = Set(now);
        active_model.update(&*self.db).await?;
        Ok(())
    }

    /// 缺陷 2.3 修复：查询需要重试的订阅（next_retry_at <= now AND status = ACTIVE）
    pub async fn list_due_retries(&self) -> Result<Vec<ReportSubscriptionModel>, AppError> {
        let now = Utc::now();
        let items = ReportSubscriptionEntity::find()
            .filter(crate::models::report_subscription::Column::Status.eq("ACTIVE"))
            .filter(crate::models::report_subscription::Column::IsEnabled.eq(true))
            .filter(crate::models::report_subscription::Column::NextRetryAt.lte(now))
            .all(&*self.db)
            .await?;
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== 缺陷 2.2 补充：订阅权限校验 — 邮箱格式校验测试 ==========

    /// 合法邮箱应通过校验
    #[test]
    fn test_is_valid_email_hefa_youx() {
        assert!(is_valid_email("user@example.com"));
        assert!(is_valid_email("john.doe@company.org"));
        assert!(is_valid_email("test+tag@domain.co.uk"));
        assert!(is_valid_email("a@b.cn"));
    }

    /// 缺少 @ 符号应拒绝
    #[test]
    fn test_is_valid_email_queshao_at() {
        assert!(!is_valid_email("userexample.com"));
        assert!(!is_valid_email("user"));
    }

    /// 多个 @ 符号应拒绝
    #[test]
    fn test_is_valid_email_duoge_at() {
        assert!(!is_valid_email("user@@example.com"));
        assert!(!is_valid_email("user@ex@ample.com"));
    }

    /// 空用户名应拒绝
    #[test]
    fn test_is_valid_email_kong_yhm() {
        assert!(!is_valid_email("@example.com"));
    }

    /// 空域名应拒绝
    #[test]
    fn test_is_valid_email_kong_ym() {
        assert!(!is_valid_email("user@"));
    }

    /// 域名缺少点号应拒绝
    #[test]
    fn test_is_valid_email_ym_qd_dh() {
        assert!(!is_valid_email("user@localhost"));
        assert!(!is_valid_email("user@example"));
    }

    /// 域名以点号开头或结尾应拒绝
    #[test]
    fn test_is_valid_email_ym_dhkg() {
        assert!(!is_valid_email("user@.example.com"));
        assert!(!is_valid_email("user@example.com."));
    }

    /// 空字符串应拒绝
    #[test]
    fn test_is_valid_email_kong_zfc() {
        assert!(!is_valid_email(""));
    }

    // ========== 缺陷 2.3 补充：重试退避间隔测试 ==========

    /// 第 1 次重试（retry_count=0）应为 60 秒（1 分钟）
    #[test]
    fn test_backoff_seconds_di_yi_ci() {
        assert_eq!(backoff_seconds(0), 60);
    }

    /// 第 2 次重试（retry_count=1）应为 300 秒（5 分钟）
    #[test]
    fn test_backoff_seconds_di_er_ci() {
        assert_eq!(backoff_seconds(1), 300);
    }

    /// 第 3 次及以后（retry_count>=2）应为 1800 秒（30 分钟）
    #[test]
    fn test_backoff_seconds_di_san_ci_yys() {
        assert_eq!(backoff_seconds(2), 1800);
        assert_eq!(backoff_seconds(3), 1800);
        assert_eq!(backoff_seconds(100), 1800);
    }

    /// 退避间隔应严格递增（指数退避语义）
    #[test]
    fn test_backoff_seconds_yg_dz() {
        assert!(backoff_seconds(0) < backoff_seconds(1));
        assert!(backoff_seconds(1) < backoff_seconds(2));
    }

    /// DEFAULT_MAX_RETRIES 常量应为 3
    #[test]
    fn test_default_max_retries_val() {
        assert_eq!(DEFAULT_MAX_RETRIES, 3);
    }
}
