use sea_orm::DatabaseConnection;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;

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
use crate::services::custom_order_crud_service::CustomOrderCrudService;
use crate::services::custom_order_state_service::CustomOrderStateService;
use crate::services::custom_order_process_service::CustomOrderProcessService;
use crate::services::custom_order_quality_service::CustomOrderQualityService;
use crate::services::custom_order_aftersales_service::CustomOrderAfterSalesService;
use crate::utils::cache::AppCache;
use crate::utils::di_container::DIContainer;

use axum::extract::FromRef;
use axum_extra::extract::cookie::Key;
use dashmap::DashMap;

/// 应用全局状态
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DatabaseConnection>,
    pub omni_audit: Arc<OmniAuditEngine>,
    pub audit_cleanup: Arc<AuditCleanupService>,
    pub jwt_secret: String,
    pub previous_jwt_secret: Option<String>,
    pub cookie_secret: String,
    /// M-2 修复：独立 Webhook HMAC 密钥
    pub webhook_secret: String,
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
    // P0-3 定制订单全流程跟踪服务
    pub custom_order_crud: Arc<CustomOrderCrudService>,
    pub custom_order_state: Arc<CustomOrderStateService>,
    pub custom_order_process: Arc<CustomOrderProcessService>,
    pub custom_order_quality: Arc<CustomOrderQualityService>,
    pub custom_order_aftersales: Arc<CustomOrderAfterSalesService>,
    /// M-1 修复：每用户每小时邮件发送配额计数器
    /// key = (user_id, hour_bucket_secs)，value = 已发送封数
    pub email_send_counters: Arc<DashMap<(i32, u64), Arc<AtomicU32>>>,
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
    #[allow(clippy::too_many_arguments)] // TODO(tech-debt): 构造全局状态需要注入多个依赖；后续考虑 Builder 模式收敛参数
    pub fn with_secrets_and_cors(
        db: Arc<DatabaseConnection>,
        omni_audit: Arc<OmniAuditEngine>,
        audit_cleanup: Arc<AuditCleanupService>,
        jwt_secret: String,
        previous_jwt_secret: Option<String>,
        cookie_secret: String,
        webhook_secret: String,
        allowed_origins: Vec<String>,
    ) -> Result<Self, String> {
        // 启动审计日志清理任务（后台任务，失败不阻塞启动）
        let cleanup_clone = audit_cleanup.clone();
        tokio::spawn(async move {
            cleanup_clone.start_cleanup_task();
        });

        // P2-B 修复：cookie_secret 长度不足 32 字节时 fail-fast，禁止自动补 0 弱化密钥
        // 安全原因：补 0 / 截断会让攻击者仅需爆破 1-N 字节即可还原密钥，违背 fail-secure 原则
        if cookie_secret.len() < 32 {
            return Err(format!(
                "cookie_secret 长度不足 32 字节（当前: {} 字节）。禁止补 0/截断弱化，请通过环境变量 COOKIE_SECRET 提供至少 32 字节的强随机密钥（openssl rand -hex 32）",
                cookie_secret.len()
            ));
        }
        let final_cookie_secret = cookie_secret;

        // M-2 修复：webhook_secret 强度校验 + 与 jwt_secret 互不相同校验
        if webhook_secret.len() < 32 {
            return Err(format!(
                "webhook_secret 长度不足 32 字节（当前: {} 字节）。请通过环境变量 WEBHOOK_SECRET 提供至少 32 字节的强随机密钥（openssl rand -hex 32）",
                webhook_secret.len()
            ));
        }
        if webhook_secret == jwt_secret {
            return Err(
                "FATAL: webhook_secret 与 jwt_secret 相同，违反 M-2 修复（密钥单一违反，泄漏面扩大）。请为 webhook 单独生成密钥"
                    .to_string(),
            );
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
            db: db.clone(),
            omni_audit,
            audit_cleanup,
            jwt_secret,
            previous_jwt_secret,
            cookie_secret: final_cookie_secret,
            // M-2 修复：独立 Webhook 密钥
            webhook_secret,
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
            // P0-3 定制订单服务（延迟构造以避免影响启动）
            custom_order_crud: Arc::new(CustomOrderCrudService::new(db.clone())),
            custom_order_state: Arc::new(CustomOrderStateService::new(db.clone())),
            custom_order_process: Arc::new(CustomOrderProcessService::new(db.clone())),
            custom_order_quality: Arc::new(CustomOrderQualityService::new(db.clone())),
            custom_order_aftersales: Arc::new(CustomOrderAfterSalesService::new(db.clone())),
            // M-1 修复：邮件发送配额计数器
            email_send_counters: Arc::new(DashMap::new()),
        })
    }
}

impl Default for AppState {
    /// **警告**：此 Default 实现仅用于测试环境。
    ///
    /// 生产环境必须使用 [`AppState::with_secrets_and_cors`] 并提供真实的密钥配置。
    /// 随机生成的密钥与数据库连接（`DatabaseConnection::default()`）仅能保证单测可运行，
    /// 不具备任何业务可用性。
    #[allow(dead_code, unused_variables)] // TODO(tech-debt): Default 实现仅用于测试环境；lib-test 下局部变量可能误报 unused
    fn default() -> Self {
        // 指标服务构造失败时显式返回字符串（之前是 .expect() panic，违背测试可观察性）
        let metrics = MetricsService::new()
            .expect("测试环境创建 Prometheus 指标服务不应失败（指标命名冲突？）");
        // 使用随机生成的密钥，而不是硬编码的默认值
        let random_cookie_secret = format!("{}{}", uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
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
        // P0-3 定制订单服务（测试环境）
        let custom_order_crud = Arc::new(CustomOrderCrudService::new(db.clone()));
        let custom_order_state = Arc::new(CustomOrderStateService::new(db.clone()));
        let custom_order_process = Arc::new(CustomOrderProcessService::new(db.clone()));
        let custom_order_quality = Arc::new(CustomOrderQualityService::new(db.clone()));
        let custom_order_aftersales = Arc::new(CustomOrderAfterSalesService::new(db.clone()));
        Self {
            db: db.clone(),
            omni_audit,
            audit_cleanup,
            // Wave B-2 修复（B2-2）：生产环境禁用随机生成 JWT secret（多副本部署签名不一致）
            // 修复方案：仅在 #[cfg(test)] 单元测试场景使用固定测试密钥；
            // 生产环境必须通过环境变量 JWT_SECRET 注入，且调用方应使用 with_secrets_and_cors
            // 显式传递真实密钥（main.rs 启动时已强制校验 JWT_SECRET 强度）。
            jwt_secret: {
                #[cfg(test)]
                {
                    "test_secret_for_unit_tests_only_min_32_bytes".to_string()
                }
                #[cfg(not(test))]
                {
                    eprintln!(
                        "FATAL: AppState::default() 在生产环境被调用，禁止使用随机 JWT 密钥"
                    );
                    eprintln!(
                        "FATAL: 生产环境必须通过环境变量 JWT_SECRET 配置稳定密钥，并通过 AppState::with_secrets_and_cors 显式注入"
                    );
                    std::process::exit(1);
                }
            },
            previous_jwt_secret: None,
            cookie_secret: random_cookie_secret,
            // M-2 修复：测试环境使用独立 webhook 密钥（与 jwt_secret 错开）
            webhook_secret: "test_webhook_secret_for_unit_tests_only_min_32_bytes".to_string(),
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
            custom_order_crud,
            custom_order_state,
            custom_order_process,
            custom_order_quality,
            custom_order_aftersales,
            // M-1 修复：测试环境也使用独立配额计数器
            email_send_counters: Arc::new(DashMap::new()),
        }
    }
}
