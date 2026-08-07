use futures::FutureExt;
use sea_orm::DatabaseConnection;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use crate::services::audit_cleanup_service::AuditCleanupService;
use crate::services::audit_log_service::AuditLogService;
use crate::services::failover_service::FailoverExecutor;

/// L-26 修复（批次 374 v13 复审）：app_state 后台任务 spawn 句柄
/// 保存审计清理 + 用户吊销清理句柄，供 shutdown 时 abort
static APP_STATE_BACKGROUND_TASKS: std::sync::Mutex<Vec<tokio::task::JoinHandle<()>>> =
    std::sync::Mutex::new(Vec::new());
use crate::search::SearchClient;
use crate::services::cache_service::CacheService;
use crate::services::custom_order_aftersales_service::CustomOrderAfterSalesService;
use crate::services::custom_order_crud_service::CustomOrderCrudService;
use crate::services::custom_order_process_service::CustomOrderProcessService;
use crate::services::custom_order_quality_service::CustomOrderQualityService;
use crate::services::custom_order_state_service::CustomOrderStateService;
use crate::services::data_permission_service::DataPermissionService;
use crate::services::email_service::EmailService;
use crate::services::event_notification_service::EventNotificationService;
use crate::services::metrics_service::MetricsService;
use crate::services::notification_service::NotificationService;
use crate::services::omni_audit_service::OmniAuditEngine;
use crate::services::quotation_approval_service::QuotationApprovalService;
use crate::services::quotation_convert_service::QuotationConvertService;
use crate::services::quotation_pricing_service::QuotationPricingService;
use crate::services::quotation_service::QuotationService;
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
    /// L-32 修复（批次 380 v13 复审）：审计日志服务（mpsc channel + handle 保存）
    pub audit_log: Arc<AuditLogService>,
    pub audit_cleanup: Arc<AuditCleanupService>,
    pub jwt_secret: String,
    pub previous_jwt_secret: Option<String>,
    pub cookie_secret: String,
    /// M-2 修复：独立 Webhook HMAC 密钥
    pub webhook_secret: String,
    pub cache: Arc<AppCache>,
    pub metrics: Arc<MetricsService>,
    pub cookie_key: Key,
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
    /// 批次 104 P0-1 修复：搜索客户端（Elasticsearch 集成，当前为 mock 实现，配置 ELASTICSEARCH_URL 后切换为真实 ES）
    pub search_client: Arc<dyn SearchClient>,
    /// 批次 107 P1-1 修复：进程内 L1 缓存（moka LRU+TTL）；L1 进程内超低延迟热点数据，L2 为 AppCache/Redis 分布式跨实例共享；场景：Dashboard/配置/报表热点；CACHE_ENABLED=false 关闭
    pub cache_service: Arc<CacheService>,
    /// V15 P0-B17（Batch 484）：主备切换执行器（维护 primary+backup 两个 DB 连接，ArcSwap 原子切换；备库未配置时 switch_to_backup 返回 Err 降级为仅更新 status 表）
    pub failover_executor: Arc<FailoverExecutor>,
}

/// 应用状态构造参数对象（批次 331 v10 复审 P3 修复：聚合 8 个参数消除 too_many_arguments 警告）
pub struct AppStateParams {
    /// 数据库连接
    pub db: Arc<DatabaseConnection>,
    /// 全量审计引擎
    pub omni_audit: Arc<OmniAuditEngine>,
    /// L-32 修复（批次 380 v13 复审）：审计日志服务（mpsc channel + handle 保存）
    pub audit_log: Arc<AuditLogService>,
    /// 审计清理服务
    pub audit_cleanup: Arc<AuditCleanupService>,
    /// JWT 主密钥
    pub jwt_secret: String,
    /// JWT 轮换期间的旧密钥（可选）
    pub previous_jwt_secret: Option<String>,
    /// Cookie 签名密钥
    pub cookie_secret: String,
    /// Webhook HMAC 签名密钥
    pub webhook_secret: String,
    /// CORS 允许的源列表
    pub allowed_origins: Vec<String>,
    /// V15 P0-B17（Batch 484）：主备切换执行器（main.rs 构造后注入）
    pub failover_executor: Arc<FailoverExecutor>,
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
    /// 创建应用全局状态，构造失败时返回错误（例如指标注册冲突；批次 331 v10 复审 P3 修复：使用 AppStateParams 参数对象替代 8 个独立参数）
    pub fn with_secrets_and_cors(params: AppStateParams) -> Result<Self, String> {
        // 启动用户吊销记录清理后台任务（后台任务，失败不阻塞启动）
        spawn_background_tasks();
        // P2-B/M-2 修复：cookie_secret + webhook_secret 强度校验 + 互不相同校验
        validate_app_secrets(
            &params.cookie_secret,
            &params.webhook_secret,
            &params.jwt_secret,
        )?;
        // 构建业务服务集合（指标、cookie_key、DI 容器、邮件/通知/报价/定制订单服务）
        let services = build_app_services(&params.db, &params.cookie_secret)?;
        // 构造 AppState（消费 params 与 services）
        Ok(construct_app_state(params, services))
    }
}

/// 应用服务集合（with_secrets_and_cors 内部构建的 Arc 服务打包，避免 construct_app_state 参数过多）。
struct AppServices {
    metrics: MetricsService,
    cookie_key: Key,
    di_container: Arc<DIContainer>,
    email_service: Option<Arc<EmailService>>,
    event_notification_service: Option<Arc<EventNotificationService>>,
    data_permission_service: Arc<DataPermissionService>,
    notification_service: Arc<NotificationService>,
    quotation_service: Arc<QuotationService>,
    quotation_pricing_service: Arc<QuotationPricingService>,
    quotation_approval_service: Arc<QuotationApprovalService>,
    quotation_convert_service: Arc<QuotationConvertService>,
    custom_order_crud: Arc<CustomOrderCrudService>,
    custom_order_state: Arc<CustomOrderStateService>,
    custom_order_process: Arc<CustomOrderProcessService>,
    custom_order_quality: Arc<CustomOrderQualityService>,
    custom_order_aftersales: Arc<CustomOrderAfterSalesService>,
}

/// 启动用户吊销记录清理后台任务（L-26 修复：保存句柄供 shutdown abort）。
fn spawn_background_tasks() {
    // v11 批次 145 P1-7：启动用户吊销记录定期清理任务（每 24 小时清理一次）
    let revoked_handle = crate::services::auth_service::start_revoked_user_cleanup_task();
    if let Ok(mut tasks) = APP_STATE_BACKGROUND_TASKS.lock() {
        tasks.push(revoked_handle);
    }
}

/// 校验 cookie_secret 与 webhook_secret 强度（P2-B/M-2 修复：fail-fast，禁止补 0/截断弱化密钥）。
fn validate_app_secrets(
    cookie_secret: &str,
    webhook_secret: &str,
    jwt_secret: &str,
) -> Result<(), String> {
    if cookie_secret.len() < 32 {
        return Err(format!(
            "cookie_secret 长度不足 32 字节（当前: {} 字节）。禁止补 0/截断弱化，请通过环境变量 COOKIE_SECRET 提供至少 32 字节的强随机密钥（openssl rand -hex 32）",
            cookie_secret.len()
        ));
    }
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
    Ok(())
}

/// 构建业务服务集合（指标服务构造失败时显式返回错误，原 .expect() panic 违背 Result 语义）。
fn build_app_services(
    db: &Arc<DatabaseConnection>,
    cookie_secret: &str,
) -> Result<AppServices, String> {
    let metrics = MetricsService::new().map_err(|e| {
        format!(
            "创建 Prometheus 指标服务失败（指标名称冲突或注册表初始化错误）: {}",
            e
        )
    })?;
    let cookie_key = Key::derive_from(cookie_secret.as_bytes());
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
    // P0-3 定制订单服务（延迟构造以避免影响启动）
    let custom_order_crud = Arc::new(CustomOrderCrudService::new(db.clone()));
    let custom_order_state = Arc::new(CustomOrderStateService::new(db.clone()));
    let custom_order_process = Arc::new(CustomOrderProcessService::new(db.clone()));
    let custom_order_quality = Arc::new(CustomOrderQualityService::new(db.clone()));
    let custom_order_aftersales = Arc::new(CustomOrderAfterSalesService::new(db.clone()));
    Ok(AppServices {
        metrics,
        cookie_key,
        di_container,
        email_service,
        event_notification_service,
        data_permission_service,
        notification_service,
        quotation_service,
        quotation_pricing_service,
        quotation_approval_service,
        quotation_convert_service,
        custom_order_crud,
        custom_order_state,
        custom_order_process,
        custom_order_quality,
        custom_order_aftersales,
    })
}

/// 构造 AppState（消费 params 与 services，inline 构造 cache/计数器/搜索/缓存服务）。
fn construct_app_state(params: AppStateParams, services: AppServices) -> AppState {
    let metrics = Arc::new(services.metrics);
    // V15 批次 07 P1-8 修复：CacheService 注入 BusinessMetrics，缓存命中/未命中自动上报 Prometheus
    let cache_service =
        Arc::new(CacheService::new().with_metrics(metrics.business_metrics.clone()));
    AppState {
        db: params.db.clone(),
        omni_audit: params.omni_audit,
        audit_log: params.audit_log,
        audit_cleanup: params.audit_cleanup,
        jwt_secret: params.jwt_secret,
        previous_jwt_secret: params.previous_jwt_secret,
        cookie_secret: params.cookie_secret,
        // M-2 修复：独立 Webhook 密钥
        webhook_secret: params.webhook_secret,
        cache: AppCache::arc(),
        metrics,
        cookie_key: services.cookie_key,
        di_container: services.di_container,
        email_service: services.email_service,
        event_notification_service: services.event_notification_service,
        data_permission_service: services.data_permission_service,
        notification_service: services.notification_service,
        allowed_origins: params.allowed_origins,
        quotation_service: services.quotation_service,
        quotation_pricing_service: services.quotation_pricing_service,
        quotation_approval_service: services.quotation_approval_service,
        quotation_convert_service: services.quotation_convert_service,
        custom_order_crud: services.custom_order_crud,
        custom_order_state: services.custom_order_state,
        custom_order_process: services.custom_order_process,
        custom_order_quality: services.custom_order_quality,
        custom_order_aftersales: services.custom_order_aftersales,
        // M-1 修复：邮件发送配额计数器
        email_send_counters: Arc::new(DashMap::new()),
        // 批次 104 P0-1 修复：搜索客户端初始化（根据环境变量决定真实 ES 或 mock）
        search_client: init_search_client(),
        // 批次 107 P1-1 修复 + V15 批次 07 P1-8 修复：
        // L1 本地缓存初始化（根据 CACHE_ENABLED 环境变量决定是否启用），
        // 并注入 BusinessMetrics 实现 Prometheus 自动上报
        cache_service,
        // V15 P0-B17（Batch 484）：主备切换执行器（main.rs 注入）
        failover_executor: params.failover_executor,
    }
}

/// 测试环境服务集合（default() 内部构建的 Arc 服务 + db + cookie 密钥打包）。
#[cfg(test)]
struct TestServices {
    db: Arc<DatabaseConnection>,
    metrics: MetricsService,
    cookie_key: Key,
    random_cookie_secret: String,
    omni_audit: Arc<OmniAuditEngine>,
    audit_log: Arc<AuditLogService>,
    audit_cleanup: Arc<AuditCleanupService>,
    di_container: Arc<DIContainer>,
    email_service: Option<Arc<EmailService>>,
    event_notification_service: Option<Arc<EventNotificationService>>,
    data_permission_service: Arc<DataPermissionService>,
    notification_service: Arc<NotificationService>,
    quotation_service: Arc<QuotationService>,
    quotation_pricing_service: Arc<QuotationPricingService>,
    quotation_approval_service: Arc<QuotationApprovalService>,
    quotation_convert_service: Arc<QuotationConvertService>,
    custom_order_crud: Arc<CustomOrderCrudService>,
    custom_order_state: Arc<CustomOrderStateService>,
    custom_order_process: Arc<CustomOrderProcessService>,
    custom_order_quality: Arc<CustomOrderQualityService>,
    custom_order_aftersales: Arc<CustomOrderAfterSalesService>,
    failover_executor: Arc<FailoverExecutor>,
}

/// 构建测试环境服务集合（default() 调用，构造失败时显式 panic）。
#[cfg(test)]
fn build_test_services() -> TestServices {
    let metrics =
        MetricsService::new().expect("测试环境创建 Prometheus 指标服务不应失败（指标命名冲突？）");
    let random_cookie_secret = format!("{}{}", uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    let cookie_key = Key::derive_from(random_cookie_secret.as_bytes());
    let db = Arc::new(DatabaseConnection::default());
    let omni_audit = Arc::new(
        OmniAuditEngine::new(db.clone())
            .expect("测试环境创建 OmniAuditEngine 不应失败（检查 AUDIT_SECRET_KEY）"),
    );
    let audit_log = Arc::new(AuditLogService::new(db.clone()));
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
    let custom_order_crud = Arc::new(CustomOrderCrudService::new(db.clone()));
    let custom_order_state = Arc::new(CustomOrderStateService::new(db.clone()));
    let custom_order_process = Arc::new(CustomOrderProcessService::new(db.clone()));
    let custom_order_quality = Arc::new(CustomOrderQualityService::new(db.clone()));
    let custom_order_aftersales = Arc::new(CustomOrderAfterSalesService::new(db.clone()));
    let failover_executor = Arc::new(FailoverExecutor::new(db.clone(), None));
    TestServices {
        db,
        metrics,
        cookie_key,
        random_cookie_secret,
        omni_audit,
        audit_log,
        audit_cleanup,
        di_container,
        email_service,
        event_notification_service,
        data_permission_service,
        notification_service,
        quotation_service,
        quotation_pricing_service,
        quotation_approval_service,
        quotation_convert_service,
        custom_order_crud,
        custom_order_state,
        custom_order_process,
        custom_order_quality,
        custom_order_aftersales,
        failover_executor,
    }
}

impl Default for AppState {
    /// **警告**：此 Default 实现仅用于测试环境（生产必须用 with_secrets_and_cors 提供真实密钥；随机密钥+DatabaseConnection::default() 仅保证单测可运行，无业务可用性）
    fn default() -> Self {
        // 非测试环境直接 panic，禁止使用 Default 构造 AppState
        // （panic! 返回 `!` 可 coerce 到 Self；测试环境构造见下方 #[cfg(test)] 块）
        #[cfg(not(test))]
        {
            panic!(
                "AppState::default() 仅允许在测试环境调用；生产环境必须使用 \
                 AppState::with_secrets_and_cors 并通过环境变量注入真实密钥 \
                 （JWT_SECRET / COOKIE_SECRET / WEBHOOK_SECRET）"
            );
        }

        // 测试环境构造：服务集合由 build_test_services 构建，default 仅负责组装 struct literal
        #[cfg(test)]
        {
            let svc = build_test_services();
            let metrics = Arc::new(svc.metrics);
            // V15 批次 07 P1-8 修复：测试环境也注入 BusinessMetrics
            let cache_service =
                Arc::new(CacheService::new().with_metrics(metrics.business_metrics.clone()));
            Self {
                db: svc.db,
                omni_audit: svc.omni_audit,
                audit_log: svc.audit_log,
                audit_cleanup: svc.audit_cleanup,
                // Wave B-2 修复（B2-2）：测试环境使用固定 JWT 密钥
                jwt_secret: "test_secret_for_unit_tests_only_min_32_bytes".to_string(),
                previous_jwt_secret: None,
                cookie_secret: svc.random_cookie_secret,
                // M-2 修复：测试环境使用独立 webhook 密钥（与 jwt_secret 错开）
                webhook_secret: "test_webhook_secret_for_unit_tests_only_min_32_bytes".to_string(),
                cache: AppCache::arc(),
                metrics,
                cookie_key: svc.cookie_key,
                di_container: svc.di_container,
                email_service: svc.email_service,
                event_notification_service: svc.event_notification_service,
                data_permission_service: svc.data_permission_service,
                notification_service: svc.notification_service,
                allowed_origins: vec![],
                quotation_service: svc.quotation_service,
                quotation_pricing_service: svc.quotation_pricing_service,
                quotation_approval_service: svc.quotation_approval_service,
                quotation_convert_service: svc.quotation_convert_service,
                custom_order_crud: svc.custom_order_crud,
                custom_order_state: svc.custom_order_state,
                custom_order_process: svc.custom_order_process,
                custom_order_quality: svc.custom_order_quality,
                custom_order_aftersales: svc.custom_order_aftersales,
                // M-1 修复：测试环境也使用独立配额计数器
                email_send_counters: Arc::new(DashMap::new()),
                // 批次 104 P0-1 修复：测试环境使用 mock 搜索客户端
                search_client: init_search_client(),
                // 批次 107 P1-1 修复 + V15 批次 07 P1-8 修复：
                // 测试环境启用 L1 本地缓存，并注入 BusinessMetrics
                cache_service,
                // V15 P0-B17（Batch 484）：测试环境 failover_executor（仅主库）
                failover_executor: svc.failover_executor,
            }
        }
    }
}

/// 批次 104 P0-1 修复：初始化搜索客户端（按 ELASTICSEARCH_URL 决定类型：空=mock 内存 HashMap 开发/测试/CI；已设置=真实 reqwest 直连 ES REST API 生产环境）
/// 设计避免强制依赖 ES，CI 无 ES 仍可运行；批次 123 v8 复审 P1 修复：原 real() stub 已真实实现 reqwest 直连，索引初始化在 main.rs ensure_indices() 调用
fn init_search_client() -> Arc<dyn SearchClient> {
    let es_url = std::env::var("ELASTICSEARCH_URL").unwrap_or_default();
    if es_url.is_empty() {
        tracing::info!("ELASTICSEARCH_URL 未配置，使用 mock 搜索客户端（内存存储）");
        Arc::new(crate::search::ElasticClient::mock())
    } else {
        // 规则 12 合规：不记录完整 URL，防止 URL 中的 user:password@host 凭据泄露
        tracing::info!("ELASTICSEARCH_URL 已配置，使用真实 Elasticsearch 客户端");
        Arc::new(crate::search::ElasticClient::real(es_url))
    }
}

/// L-26 修复（批次 374 v13 复审）：关闭 app_state 后台定时任务
/// abort 审计清理 + 用户吊销清理 task，幂等安全
pub fn shutdown_app_state_background_tasks() {
    let tasks = match APP_STATE_BACKGROUND_TASKS.lock() {
        Ok(mut guard) => std::mem::take(&mut *guard),
        Err(e) => {
            tracing::error!(error = %e, "APP_STATE_BACKGROUND_TASKS 锁中毒");
            return;
        }
    };
    let count = tasks.len();
    for handle in tasks {
        handle.abort();
    }
    tracing::info!("app_state 后台定时任务已关闭（{} 个）", count);
}
