//! 用户隐私同意服务（V15 P1 batch-16 缺陷 7.3）
//!
//! 提供用户隐私同意记录管理能力，配合 tracking_handler 在采集前校验同意状态。
//! 支持 4 类 consent_type：behavior_tracking / page_view_tracking / cookie_usage / marketing_email
//!
//! 合规依据：《个人信息保护法》第 14 条（同意原则）+ 第 16 条（撤回权）+ GDPR 第 7 条

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::Validate;

use crate::models::user_consent::{
    ActiveModel, Entity as UserConsentEntity, Model as UserConsentModel,
};
use crate::utils::error::AppError;

/// 同意类型枚举常量（与 DB CHECK 约束一致）
pub const CONSENT_TYPE_BEHAVIOR_TRACKING: &str = "behavior_tracking";
pub const CONSENT_TYPE_PAGE_VIEW_TRACKING: &str = "page_view_tracking";
pub const CONSENT_TYPE_COOKIE_USAGE: &str = "cookie_usage";
pub const CONSENT_TYPE_MARKETING_EMAIL: &str = "marketing_email";

/// 隐私政策默认版本号
const DEFAULT_CONSENT_TEXT_VERSION: &str = "v1.0";

/// 记录用户同意请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RecordConsentRequest {
    /// 同意类型：behavior_tracking / page_view_tracking / cookie_usage / marketing_email
    #[validate(length(min = 1, max = 50, message = "consent_type 长度 1-50"))]
    pub consent_type: String,
    /// 是否同意
    pub consent_given: bool,
    /// 隐私政策文本版本（可选，默认 v1.0）
    #[validate(length(max = 20, message = "consent_text_version 长度不超过 20"))]
    pub consent_text_version: Option<String>,
}

/// 同意状态查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct ConsentStatusQuery {
    /// 同意类型（不传则返回所有类型的最新状态）
    pub consent_type: Option<String>,
}

/// 用户同意状态响应
#[derive(Debug, Serialize)]
pub struct ConsentStatus {
    pub consent_type: String,
    pub consent_given: bool,
    pub consent_text_version: Option<String>,
    pub consented_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// 用户隐私同意 Service
pub struct UserConsentService {
    db: Arc<DatabaseConnection>,
}

impl UserConsentService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 校验 consent_type 必须为预定义类型
    fn validate_consent_type(t: &str) -> Result<(), AppError> {
        match t {
            CONSENT_TYPE_BEHAVIOR_TRACKING
            | CONSENT_TYPE_PAGE_VIEW_TRACKING
            | CONSENT_TYPE_COOKIE_USAGE
            | CONSENT_TYPE_MARKETING_EMAIL => Ok(()),
            _ => Err(AppError::validation(format!(
                "无效的 consent_type: {}（应为 behavior_tracking/page_view_tracking/cookie_usage/marketing_email）",
                t
            ))),
        }
    }

    /// 记录用户的同意/退出决定（业务逻辑：每次变更都新增一条记录，保留审计轨迹。；同时将前一条同意记录标记为已撤回（revoked_at = now）。）
    pub async fn record_consent(
        &self,
        user_id: i32,
        req: RecordConsentRequest,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<UserConsentModel, AppError> {
        req.validate()
            .map_err(|e| AppError::validation(e.to_string()))?;
        Self::validate_consent_type(&req.consent_type)?;

        let now = Utc::now();
        let text_version = req
            .consent_text_version
            .clone()
            .unwrap_or_else(|| DEFAULT_CONSENT_TEXT_VERSION.to_string());

        // 缺陷 7.3 修复：撤回前一条同类型同意记录（保留审计轨迹）
        // 不在事务中执行，因为 consent 变更不要求严格原子性（DB 约束保证最终一致）
        let prev = UserConsentEntity::find()
            .filter(crate::models::user_consent::Column::UserId.eq(user_id))
            .filter(crate::models::user_consent::Column::ConsentType.eq(req.consent_type.clone()))
            .filter(crate::models::user_consent::Column::RevokedAt.is_null())
            .order_by_desc(crate::models::user_consent::Column::ConsentedAt)
            .one(&*self.db)
            .await?;

        if let Some(prev_model) = prev {
            let mut prev_active: ActiveModel = prev_model.into();
            prev_active.revoked_at = Set(Some(now));
            prev_active.update(&*self.db).await?;
        }

        let active = ActiveModel {
            id: Default::default(),
            user_id: Set(user_id),
            consent_type: Set(req.consent_type),
            consent_given: Set(req.consent_given),
            consent_text_version: Set(Some(text_version)),
            consented_at: Set(now),
            revoked_at: Set(None),
            ip_address: Set(ip_address),
            user_agent: Set(user_agent),
            created_at: Set(now),
        };
        let model = active.insert(&*self.db).await?;
        Ok(model)
    }

    /// 获取用户当前最新同意状态（按 consent_type）
    pub async fn get_current_consent(
        &self,
        user_id: i32,
        consent_type: &str,
    ) -> Result<Option<UserConsentModel>, AppError> {
        Self::validate_consent_type(consent_type)?;
        let model = UserConsentEntity::find()
            .filter(crate::models::user_consent::Column::UserId.eq(user_id))
            .filter(crate::models::user_consent::Column::ConsentType.eq(consent_type))
            .filter(crate::models::user_consent::Column::RevokedAt.is_null())
            .order_by_desc(crate::models::user_consent::Column::ConsentedAt)
            .one(&*self.db)
            .await?;
        Ok(model)
    }

    /// 获取用户所有 consent_type 的当前状态
    pub async fn list_current_consents(
        &self,
        user_id: i32,
    ) -> Result<Vec<UserConsentModel>, AppError> {
        let items = UserConsentEntity::find()
            .filter(crate::models::user_consent::Column::UserId.eq(user_id))
            .filter(crate::models::user_consent::Column::RevokedAt.is_null())
            .order_by_desc(crate::models::user_consent::Column::ConsentedAt)
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// 缺陷 7.3 修复：判断用户是否同意指定类型的采集（默认行为：未找到同意记录时返回 false（最小权限原则 + 合规优先））
    pub async fn is_consent_given(
        &self,
        user_id: i32,
        consent_type: &str,
    ) -> Result<bool, AppError> {
        let consent = self.get_current_consent(user_id, consent_type).await?;
        Ok(consent.map(|c| c.consent_given).unwrap_or(false))
    }

    /// 一键退出所有追踪（用户行使撤回权）
    /// 对 behavior_tracking / page_view_tracking / cookie_usage / marketing_email；4 类 consent 全部记录为 false（退出采集）。
    pub async fn opt_out_all(
        &self,
        user_id: i32,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<Vec<UserConsentModel>, AppError> {
        let all_types = [
            CONSENT_TYPE_BEHAVIOR_TRACKING,
            CONSENT_TYPE_PAGE_VIEW_TRACKING,
            CONSENT_TYPE_COOKIE_USAGE,
            CONSENT_TYPE_MARKETING_EMAIL,
        ];

        let mut results = Vec::with_capacity(all_types.len());
        for ct in all_types {
            let req = RecordConsentRequest {
                consent_type: ct.to_string(),
                consent_given: false,
                consent_text_version: Some(DEFAULT_CONSENT_TEXT_VERSION.to_string()),
            };
            let m = self
                .record_consent(user_id, req, ip_address.clone(), user_agent.clone())
                .await?;
            results.push(m);
        }
        Ok(results)
    }

    /// 一键同意所有追踪（首次登录隐私政策确认后调用）
    pub async fn opt_in_all(
        &self,
        user_id: i32,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<Vec<UserConsentModel>, AppError> {
        let all_types = [
            CONSENT_TYPE_BEHAVIOR_TRACKING,
            CONSENT_TYPE_PAGE_VIEW_TRACKING,
            CONSENT_TYPE_COOKIE_USAGE,
            CONSENT_TYPE_MARKETING_EMAIL,
        ];

        let mut results = Vec::with_capacity(all_types.len());
        for ct in all_types {
            let req = RecordConsentRequest {
                consent_type: ct.to_string(),
                consent_given: true,
                consent_text_version: Some(DEFAULT_CONSENT_TEXT_VERSION.to_string()),
            };
            let m = self
                .record_consent(user_id, req, ip_address.clone(), user_agent.clone())
                .await?;
            results.push(m);
        }
        Ok(results)
    }
}

/// 便捷方法：将 UserConsentModel 转换为 ConsentStatus
pub fn to_status(m: &UserConsentModel) -> ConsentStatus {
    ConsentStatus {
        consent_type: m.consent_type.clone(),
        consent_given: m.consent_given,
        consent_text_version: m.consent_text_version.clone(),
        consented_at: Some(m.consented_at),
    }
}
