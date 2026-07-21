use futures::FutureExt;
use sea_orm::DatabaseConnection;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;

use crate::services::audit_cleanup_service::AuditCleanupService;
use crate::services::audit_log_service::AuditLogService;
use crate::services::failover_service::FailoverExecutor;

/// L-26 修复（批次 374 v13 复审）：app_state 后台任务 spawn 句柄
/// 保存审计清理 + 用户吊销清理句柄，供 shutdown 时 abort
static APP_STATE_BACKGROUND_TASKS: std::sync::Mutex<Vec<tokio::task::JoinHandle<()>>> =
    std::sync::Mutex::new(Vec::new());
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
use crate::search::SearchClient;
use crate::services::cache_service::CacheService;
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
    /// 批次 104 P0-1 修复：搜索客户端（Elasticsearch 集成）
    /// 当前为 mock 实现（内存 HashMap + 关键字 substring 匹配），
    /// 配置 ELASTICSEARCH_URL 后可切换为真实 ES 客户端。
    pub search_client: Arc<dyn SearchClient>,
    /// 批次 107 P1-1 修复：进程内 L1 缓存（moka LRU + TTL）
    ///
    /// 设计为 L1 本地缓存，与 Redis L2 缓存形成多级缓存架构：
    /// - L1（本字段）：进程内 moka 缓存，超低延迟，适合热点数据
    /// - L2（state.cache 为 AppCache/Redis）：分布式缓存，跨实例共享
    ///
    /// 使用场景：Dashboard 聚合查询、配置类数据、报表热点数据
    /// 关闭方式：CACHE_ENABLED=false
    pub cache_service: Arc<CacheService>,
    /// V15 P0-B17（Batch 484）：主备切换执行器
    ///
    /// 维护 primary + optional backup 两个 DatabaseConnection，
    /// 通过 ArcSwap 原子切换。业务层通过 `failover_executor.get_current()` 获取活跃连接。
    /// 备库未配置时（DATABASE_BACKUP_URL 未设）switch_to_backup 返回 Err，降级为仅更新 status 表。
    pub failover_executor: Arc<FailoverExecutor>,
}

/// 应用状态构造参数对象
///
/// 批次 331 v10 复审 P3 修复：引入参数对象消除 with_secrets_and_cors 的 too_many_arguments 警告
/// （8 个独立参数 >7 触发 clippy 警告，聚合为单一 struct 便于扩展和维护）
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
    /// 创建应用全局状态，构造失败时返回错误（例如指标注册冲突）
    ///
    /// 批次 331 v10 复审 P3 修复：使用 AppStateParams 参数对象替代 8 个独立参数
    pub fn with_secrets_and_cors(params: AppStateParams) -> Result<Self, String> {
        // 启动审计日志清理任务 + 用户吊销记录定期清理任务（后台任务，失败不阻塞启动）
        spawn_background_tasks(&params.audit_cleanup);
        // P2-B/M-2 修复：cookie_secret + webhook_secret 强度校验 + 互不相同校验
        validate_app_secrets(&params.cookie_secret, &params.webhook_secret, &params.jwt_secret)?;
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

/// 启动审计清理 + 用户吊销记录清理后台任务（L-26 修复：保存句柄供 shutdown abort）。
fn spawn_background_tasks(audit_cleanup: &Arc<AuditCleanupService>) {
    let cleanup_clone = audit_cleanup.clone();
    let audit_handle = tokio::spawn(async move {
        // 批次 8（2026-06-28）：启动器 spawn panic 隔离
        let result = AssertUnwindSafe(async {
            cleanup_clone.start_cleanup_task();
        })
        .catch_unwind()
        .await;
        if let Err(panic_payload) = result {
            let panic_msg = panic_payload
                .downcast_ref::<String>()
                .map(|s| s.as_str())
                .or_else(|| panic_payload.downcast_ref::<&'static str>().copied())
                .unwrap_or("<非字符串 panic payload>");
            tracing::error!(
                panic = %panic_msg,
                "⚠ 审计清理启动器 spawn panic 已被隔离"
            );
        }
    });
    if let Ok(mut tasks) = APP_STATE_BACKGROUND_TASKS.lock() {
        tasks.push(audit_handle);
    }
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
        metrics: Arc::new(services.metrics),
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
        // 批次 107 P1-1 修复：L1 本地缓存初始化（根据 CACHE_ENABLED 环境变量决定是否启用）
        cache_service: Arc::new(CacheService::new()),
        // V15 P0-B17（Batch 484）：主备切换执行器（main.rs 注入）
        failover_executor: params.failover_executor,
    }
}

impl Default for AppState {
    /// **警告**：此 Default 实现仅用于测试环境。
    ///
    /// 生产环境必须使用 [`AppState::with_secrets_and_cors`] 并提供真实的密钥配置。
    /// 随机生成的密钥与数据库连接（`DatabaseConnection::default()`）仅能保证单测可运行，
    /// 不具备任何业务可用性。
    ///
    /// 批次 345 v11 复审 P2-8 修复：重构 default() 方法消除 #[allow(dead_code, unused_variables)]。
    /// 原实现在 jwt_secret 字段初始化器中通过 #[cfg(not(test))] 调用 std::process::exit(1)，
    /// 导致后续字段初始化代码在非测试构建中被判定为不可达，触发 dead_code + unreachable_code 警告，
    /// 需要文件级抑制掩盖。重构方案：将 #[cfg(not(test))] 的 fail-fast 提前到函数体最开头，
    /// panic! 返回 `!` 类型可 coerce 到 Self，后续不构造任何变量；测试构建中所有局部变量均被
    /// 字段初始化器使用，消除 unused_variables。规则 14 合规：移除所有警告抑制。
    fn default() -> Self {
        // 非测试环境直接 panic，禁止使用 Default 构造 AppState
        // （测试环境构造见下方 #[cfg(test)] 块；panic! 返回 `!` 可 coerce 到 Self）
        #[cfg(not(test))]
        {
            panic!(
                "AppState::default() 仅允许在测试环境调用；生产环境必须使用 \
                 AppState::with_secrets_and_cors 并通过环境变量注入真实密钥 \
                 （JWT_SECRET / COOKIE_SECRET / WEBHOOK_SECRET）"
            );
        }

        // 测试环境构造：所有局部变量均被字段初始化器使用，消除 unused_variables
        #[cfg(test)]
        {
            // 指标服务构造失败时显式 panic（测试环境指标命名冲突属致命错误）
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
            // P0-3 定制订单服务（测试环境）
            let custom_order_crud = Arc::new(CustomOrderCrudService::new(db.clone()));
            let custom_order_state = Arc::new(CustomOrderStateService::new(db.clone()));
            let custom_order_process = Arc::new(CustomOrderProcessService::new(db.clone()));
            let custom_order_quality = Arc::new(CustomOrderQualityService::new(db.clone()));
            let custom_order_aftersales = Arc::new(CustomOrderAfterSalesService::new(db.clone()));
            // V15 P0-B17（Batch 484）：测试环境构造 FailoverExecutor（仅主库，无备库）
            let failover_executor = Arc::new(FailoverExecutor::new(db.clone(), None));
            Self {
                db: db.clone(),
                omni_audit,
                audit_log,
                audit_cleanup,
                // Wave B-2 修复（B2-2）：测试环境使用固定 JWT 密钥
                // 生产环境必须通过环境变量 JWT_SECRET 注入，且调用方应使用 with_secrets_and_cors
                // 显式传递真实密钥（main.rs 启动时已强制校验 JWT_SECRET 强度）。
                jwt_secret: "test_secret_for_unit_tests_only_min_32_bytes".to_string(),
                previous_jwt_secret: None,
                cookie_secret: random_cookie_secret,
                // M-2 修复：测试环境使用独立 webhook 密钥（与 jwt_secret 错开）
                webhook_secret: "test_webhook_secret_for_unit_tests_only_min_32_bytes".to_string(),
                cache: AppCache::arc(),
                metrics: Arc::new(metrics),
                cookie_key,
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
                // 批次 104 P0-1 修复：测试环境使用 mock 搜索客户端
                search_client: init_search_client(),
                // 批次 107 P1-1 修复：测试环境启用 L1 本地缓存
                cache_service: Arc::new(CacheService::new()),
                // V15 P0-B17（Batch 484）：测试环境 failover_executor（仅主库）
                failover_executor,
            }
        }
    }
}

/// 批次 104 P0-1 修复：初始化搜索客户端
///
/// 根据环境变量 `ELASTICSEARCH_URL` 决定客户端类型：
/// - 未设置或为空：使用 mock 客户端（内存 HashMap，适用于开发/测试/CI 环境）
/// - 已设置：使用真实 ES 客户端（reqwest 直连 ES REST API，生产环境）
///
/// 设计原因：避免强制依赖 Elasticsearch 服务器，CI 环境无 ES 时仍可运行。
/// 生产环境通过环境变量切换为真实客户端。
///
/// 批次 123 v8 复审 P1 修复：原 real() 为 stub（返回 mock storage），
/// 现已真实实现 reqwest 直连 ES REST API。索引初始化在 main.rs 启动时调用 ensure_indices()。
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
