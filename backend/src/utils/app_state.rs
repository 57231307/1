use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::middleware::api_gateway::RateLimitStore;
use crate::services::audit_cleanup_service::AuditCleanupService;
use crate::services::data_permission_service::DataPermissionService;
use crate::services::email_service::EmailService;
use crate::services::event_notification_service::EventNotificationService;
use crate::services::metrics_service::MetricsService;
use crate::services::notification_service::NotificationService;
use crate::services::omni_audit_service::OmniAuditEngine;
use crate::services::quotation_service::QuotationService;
use crate::services::quotation_pricing_service::QuotationPricingService;
use crate::services::quotation_approval_service::QuotationApprovalService;
use crate::services::quotation_convert_service::QuotationConvertService;
use crate::utils::cache::AppCache;
use crate::utils::di_container::DIContainer;

use axum::extract::FromRef;
use axum_extra::extract::cookie::Key;

/// 应用全局状态
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DatabaseConnection>,
    pub omni_audit: Arc<OmniAuditEngine>,
    pub audit_cleanup: Arc<AuditCleanupService>,
    pub jwt_secret: String,
    pub previous_jwt_secret: Option<String>,
    pub cookie_secret: String,
    pub cache: Arc<AppCache>,
    pub metrics: Arc<MetricsService>,
    pub cookie_key: Key,
    pub rate_limiter: Arc<RateLimitStore>,
    pub di_container: Arc<DIContainer>,
    pub email_service: Option<Arc<EmailService>>,
    pub event_notification_service: Option<Arc<EventNotificationService>>,
    pub data_permission_service: Arc<DataPermissionService>,
    pub notification_service: Arc<NotificationService>,
    pub allowed_origins: Vec<String>,
    // 销售报价单服务（Week 1）
    pub quotation_service: Arc<QuotationService>,
    // 销售报价单定价服务（Week 2 Task 6）
    pub quotation_pricing_service: Arc<QuotationPricingService>,
    // 销售报价单审批服务（Week 2 Task 7）
    pub quotation_approval_service: Arc<QuotationApprovalService>,
    // 销售报价单转订单服务（Week 2 Task 8）
    pub quotation_convert_service: Arc<QuotationConvertService>,
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.cookie_key.clone()
    }
}

impl FromRef<AppState> for Arc<MetricsService> {
    fn from_ref(state: &AppState) -> Self {
        state.metrics.clone()
    }
}

impl AppState {
    /// 创建应用全局状态，构造失败时返回错误（例如指标注册冲突）
    pub fn with_secrets_and_cors(
        db: Arc<DatabaseConnection>,
        omni_audit: Arc<OmniAuditEngine>,
        audit_cleanup: Arc<AuditCleanupService>,
        jwt_secret: String,
        previous_jwt_secret: Option<String>,
        cookie_secret: String,
        allowed_origins: Vec<String>,
    ) -> Result<Self, String> {
        // 启动审计日志清理任务（后台任务，失败不阻塞启动）
        let cleanup_clone = audit_cleanup.clone();
        tokio::spawn(async move {
            cleanup_clone.start_cleanup_task();
        });

        let mut final_cookie_secret = cookie_secret;
        if final_cookie_secret.len() < 32 {
            tracing::warn!(
                "配置警告: cookie_secret 长度不足 32 字节 (当前长度: {})。这会降低系统的加密安全性。已自动为您填充为 32 字节以保证服务启动，请尽快在生产环境更换！",
                final_cookie_secret.len()
            );
            final_cookie_secret.push_str(&"0".repeat(32 - final_cookie_secret.len()));
        }

        // 指标服务构造失败时显式返回错误（之前是 .expect() panic，违背 Result 语义）
        let metrics = MetricsService::new().map_err(|e| {
            format!(
                "创建 Prometheus 指标服务失败（指标名称冲突或注册表初始化错误）: {}",
                e
            )
        })?;
        let cookie_key = Key::derive_from(final_cookie_secret.as_bytes());
        let di_container = Arc::new(DIContainer::new());
        let email_service = EmailService::from_env().map(Arc::new);
        let event_notification_service = email_service.as_ref().map(|email_svc| {
            Arc::new(EventNotificationService::with_email(
                db.clone(),
                email_svc.clone(),
            ))
        });
        let data_permission_service = Arc::new(DataPermissionService::new(db.clone()));
        let notification_service = Arc::new(NotificationService::new(db.clone()));
        let quotation_service = Arc::new(QuotationService::new(db.clone()));
        let quotation_pricing_service = Arc::new(QuotationPricingService::new(db.clone()));
        let quotation_approval_service = Arc::new(QuotationApprovalService::new(db.clone()));
        let quotation_convert_service = Arc::new(QuotationConvertService::new(db.clone()));

        Ok(Self {
            db,
            omni_audit,
            audit_cleanup,
            jwt_secret,
            previous_jwt_secret,
            cookie_secret: final_cookie_secret,
            cache: AppCache::arc(),
            metrics: Arc::new(metrics),
            cookie_key,
            rate_limiter: Arc::new(RateLimitStore::new()),
            di_container,
            email_service,
            event_notification_service,
            data_permission_service,
            notification_service,
            allowed_origins,
            quotation_service,
            quotation_pricing_service,
            quotation_approval_service,
            quotation_convert_service,
        })
    }
}

impl Default for AppState {
    /// **警告**：此 Default 实现仅用于测试环境。
    ///
    /// 生产环境必须使用 [`AppState::with_secrets_and_cors`] 并提供真实的密钥配置。
    /// 随机生成的密钥与数据库连接（`DatabaseConnection::default()`）仅能保证单测可运行，
    /// 不具备任何业务可用性。
    fn default() -> Self {
        // 指标服务构造失败时显式返回字符串（之前是 .expect() panic，违背测试可观察性）
        let metrics = MetricsService::new()
            .expect("测试环境创建 Prometheus 指标服务不应失败（指标命名冲突？）");
        // 使用随机生成的密钥，而不是硬编码的默认值
        let random_cookie_secret =
            uuid::Uuid::new_v4().to_string() + &uuid::Uuid::new_v4().to_string();
        let cookie_key = Key::derive_from(random_cookie_secret.as_bytes());
        let db = Arc::new(DatabaseConnection::default());
        let omni_audit = Arc::new(
            OmniAuditEngine::new(db.clone())
                .expect("测试环境创建 OmniAuditEngine 不应失败（检查 AUDIT_SECRET_KEY）"),
        );
        let audit_cleanup = Arc::new(AuditCleanupService::new(db.clone(), 999));
        let di_container = Arc::new(DIContainer::new());
        let email_service = EmailService::from_env().map(Arc::new);
        let event_notification_service = Some(Arc::new(EventNotificationService::new(db.clone())));
        let data_permission_service = Arc::new(DataPermissionService::new(db.clone()));
        let notification_service = Arc::new(NotificationService::new(db.clone()));
        let quotation_service = Arc::new(QuotationService::new(db.clone()));
        let quotation_pricing_service = Arc::new(QuotationPricingService::new(db.clone()));
        let quotation_approval_service = Arc::new(QuotationApprovalService::new(db.clone()));
        let quotation_convert_service = Arc::new(QuotationConvertService::new(db.clone()));
        Self {
            db: db.clone(),
            omni_audit,
            audit_cleanup,
            jwt_secret: uuid::Uuid::new_v4().to_string() + &uuid::Uuid::new_v4().to_string(),
            previous_jwt_secret: None,
            cookie_secret: random_cookie_secret,
            cache: AppCache::arc(),
            metrics: Arc::new(metrics),
            cookie_key,
            rate_limiter: Arc::new(RateLimitStore::new()),
            di_container,
            email_service,
            event_notification_service,
            data_permission_service,
            notification_service,
            allowed_origins: vec![],
            quotation_service,
            quotation_pricing_service,
            quotation_approval_service,
            quotation_convert_service,
        }
    }
}
