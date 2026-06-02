use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::middleware::api_gateway::RateLimitStore;
use crate::middleware::rate_limit::RedisRateLimiter;
use crate::services::audit_cleanup_service::AuditCleanupService;
use crate::services::data_permission_service::DataPermissionService;
use crate::services::email_service::EmailService;
use crate::services::event_notification_service::EventNotificationService;
use crate::services::metrics_service::MetricsService;
use crate::services::notification_service::NotificationService;
use crate::services::omni_audit_service::OmniAuditEngine;
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
    pub redis_limiter: Option<Arc<RedisRateLimiter>>,
    pub di_container: Arc<DIContainer>,
    pub email_service: Option<Arc<EmailService>>,
    pub event_notification_service: Option<Arc<EventNotificationService>>,
    pub data_permission_service: Arc<DataPermissionService>,
    pub notification_service: Arc<NotificationService>,
    pub allowed_origins: Vec<String>,
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.cookie_key.clone()
    }
}

impl AppState {
    pub fn new(db: Arc<DatabaseConnection>, jwt_secret: String) -> Result<Self, String> {
        let omni_audit = Arc::new(OmniAuditEngine::new(db.clone())?);
        let audit_cleanup = Arc::new(AuditCleanupService::new(db.clone(), 999)); // 保留 999 天

        // 启动审计日志清理任务
        let cleanup_clone = audit_cleanup.clone();
        tokio::spawn(async move {
            cleanup_clone.start_cleanup_task();
        });

        Ok(Self::with_secrets(
            db,
            omni_audit,
            audit_cleanup,
            jwt_secret.clone(),
            None,
            jwt_secret,
        ))
    }

    pub fn with_secrets(
        db: Arc<DatabaseConnection>,
        omni_audit: Arc<OmniAuditEngine>,
        audit_cleanup: Arc<AuditCleanupService>,
        jwt_secret: String,
        previous_jwt_secret: Option<String>,
        cookie_secret: String,
    ) -> Self {
        Self::with_secrets_and_cors(
            db,
            omni_audit,
            audit_cleanup,
            jwt_secret,
            previous_jwt_secret,
            cookie_secret,
            vec![],
        )
    }

    pub fn with_secrets_and_cors(
        db: Arc<DatabaseConnection>,
        omni_audit: Arc<OmniAuditEngine>,
        audit_cleanup: Arc<AuditCleanupService>,
        jwt_secret: String,
        previous_jwt_secret: Option<String>,
        cookie_secret: String,
        allowed_origins: Vec<String>,
    ) -> Self {
        let mut final_cookie_secret = cookie_secret;
        if final_cookie_secret.len() < 32 {
            tracing::warn!(
                "配置警告: cookie_secret 长度不足 32 字节 (当前长度: {})。这会降低系统的加密安全性。已自动为您填充为 32 字节以保证服务启动，请尽快在生产环境更换！",
                final_cookie_secret.len()
            );
            final_cookie_secret.push_str(&"0".repeat(32 - final_cookie_secret.len()));
        }

        let metrics = MetricsService::new().expect("Failed to create metrics service");
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

        // 尝试创建 Redis 限流器，如失败则设为 None（将回退到内存限流）
        let redis_limiter = std::env::var("REDIS_URL").ok().and_then(|url| {
            match RedisRateLimiter::new(&url, 100, 60) {
                Ok(limiter) => {
                    tracing::info!("Redis 限流器初始化成功");
                    Some(Arc::new(limiter))
                }
                Err(e) => {
                    tracing::warn!("Redis 限流器初始化失败: {}，将使用内存限流", e);
                    None
                }
            }
        });

        Self {
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
            redis_limiter,
            di_container,
            email_service,
            event_notification_service,
            data_permission_service,
            notification_service,
            allowed_origins,
        }
    }

    /// 从DI容器获取服务实例
    pub fn get_service<T: 'static + Send + Sync>(&self) -> Option<Arc<T>> {
        self.di_container.get::<T>()
    }

    /// 向DI容器注册服务实例
    pub fn register_service<T: 'static + Send + Sync>(&self, instance: Arc<T>) {
        self.di_container.register_singleton(instance);
    }
}

impl Default for AppState {
    /// 警告：此Default实现仅用于测试环境。
    /// 生产环境必须使用环境变量配置真实的密钥。
    /// 如果检测到编译目标为release模式，将panic以防止意外使用。
    fn default() -> Self {
        #[cfg(not(debug_assertions))]
        panic!("AppState::default() 禁止在生产环境(release模式)中使用。请使用 AppState::new() 并提供真实的密钥配置。");

        let metrics = MetricsService::new().expect("Failed to create metrics service");
        // 使用随机生成的密钥，而不是硬编码的默认值
        let random_cookie_secret =
            uuid::Uuid::new_v4().to_string() + &uuid::Uuid::new_v4().to_string();
        let cookie_key = Key::derive_from(random_cookie_secret.as_bytes());
        let db = Arc::new(DatabaseConnection::Disconnected);
        let omni_audit = Arc::new(
            OmniAuditEngine::new(db.clone())
                .expect("Failed to create OmniAuditEngine: AUDIT_SECRET_KEY must be set"),
        );
        let audit_cleanup = Arc::new(AuditCleanupService::new(db.clone(), 999));
        let di_container = Arc::new(DIContainer::new());
        let email_service = EmailService::from_env().map(Arc::new);
        let event_notification_service = Some(Arc::new(EventNotificationService::new(db.clone())));
        let data_permission_service = Arc::new(DataPermissionService::new(db.clone()));
        let notification_service = Arc::new(NotificationService::new(db.clone()));
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
            redis_limiter: None,
            di_container,
            email_service,
            event_notification_service,
            data_permission_service,
            notification_service,
            allowed_origins: vec![],
        }
    }
}
