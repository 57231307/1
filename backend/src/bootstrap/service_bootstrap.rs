//! 服务初始化（数据库迁移 / 服务创建 / 后台任务 / AppState 组装）
//!
//! 职责：数据库连接成功后，执行防御式迁移、创建审计/清理/故障转移等服务、
//! 启动后台定时任务、组装 AppState，并返回 graceful shutdown 所需的服务句柄。

use std::io::Write;
use std::sync::Arc;

use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::config::settings::AppSettings;
use crate::container::{AppState, AppStateParams};

/// 启动过程中创建的需要在 graceful shutdown 时关闭的服务句柄。
///
/// L-30 修复（批次 372 v13 复审）：保留 OmniAuditEngine clone 用于 shutdown 后
/// 调用 shutdown()，避免审计引擎 detached task 泄漏。
/// L-32 修复（批次 380 v13 复审）：保留 AuditLogService clone 用于 shutdown 后
/// 调用 shutdown()，避免审计日志 detached task 泄漏。
pub struct BootstrapShutdownHandles {
    omni_audit: Option<Arc<crate::services::omni_audit_service::OmniAuditEngine>>,
    audit_log: Option<Arc<crate::services::audit_log_service::AuditLogService>>,
}

impl Default for BootstrapShutdownHandles {
    fn default() -> Self {
        Self {
            omni_audit: None,
            audit_log: None,
        }
    }
}

impl BootstrapShutdownHandles {
    /// 关闭所有持有的服务（幂等安全，可重复调用）。
    ///
    /// V15 P1 修复（E0507）：将 `self` 改为 `&mut self`，避免在 `&mut` 引用上
    /// 触发 move。使用 `Option::take()` 取出所有权，第二次调用时 `Option` 已为 `None`，
    /// 自然实现幂等。
    pub fn shutdown(&mut self) {
        if let Some(omni_audit) = self.omni_audit.take() {
            omni_audit.shutdown();
        }
        if let Some(audit_log) = self.audit_log.take() {
            audit_log.shutdown();
        }
    }
}

/// L-26 修复（批次 374 v13 复审）：main.rs 后台定时任务 spawn 句柄
/// 保存 admin 缓存清理 + JTI 黑名单清理 + 慢查询采集句柄，供 shutdown abort
static MAIN_BACKGROUND_TASKS: std::sync::Mutex<Vec<tokio::task::JoinHandle<()>>> =
    std::sync::Mutex::new(Vec::new());

/// V15 P2 B05-P2-5：后台定时任务统一取消 Token。
/// shutdown 时先 cancel() 通知循环优雅退出，再保留 abort() 兜底强杀未退出的任务。
static MAIN_CANCELLATION_TOKEN: once_cell::sync::Lazy<CancellationToken> =
    once_cell::sync::Lazy::new(CancellationToken::new);

/// V15 P2 B05-P2-5：获取后台任务取消 Token 的引用（供 5 个 spawn 任务 clone 传入循环）。
#[allow(dead_code)]
pub fn main_cancellation_token() -> &'static CancellationToken {
    &MAIN_CANCELLATION_TOKEN
}

/// L-26 修复（批次 374）：关闭 main.rs 后台定时任务，幂等安全。
/// V15 P2 B05-P2-5：先调用 token.cancel() 通知所有循环优雅退出，再 abort() 兜底。
pub fn shutdown_main_background_tasks() {
    MAIN_CANCELLATION_TOKEN.cancel();
    let tasks = match MAIN_BACKGROUND_TASKS.lock() {
        Ok(mut guard) => std::mem::take(&mut *guard),
        Err(e) => {
            warn!("MAIN_BACKGROUND_TASKS 锁中毒: {}", e);
            return;
        }
    };
    let count = tasks.len();
    for handle in tasks {
        handle.abort();
    }
    info!(
        "main 后台定时任务已关闭（{} 个，已发送 cancel 信号 + abort 兜底）",
        count
    );
}

/// 完整模式启动：数据库已连接后执行迁移、创建服务、启动后台任务、组装 AppState。
///
/// 返回 `(AppState, BootstrapShutdownHandles)`，后者用于 graceful shutdown 时
/// 关闭审计引擎和审计日志服务。
pub async fn bootstrap_full_mode(
    db: DatabaseConnection,
    settings: &AppSettings,
) -> Result<(AppState, BootstrapShutdownHandles), Box<dyn std::error::Error>> {
    run_defensive_migrations(&db).await;
    run_seaorm_migrator(&db).await;
    std::io::stdout().flush().ok();
    std::io::stderr().flush().ok();

    let cookie_secret = require_cookie_secret(settings);
    let webhook_secret = require_webhook_secret(settings);

    let db = Arc::new(db);
    let mut shutdown_handles = BootstrapShutdownHandles::default();

    let omni_audit = create_omni_audit_service(&db)?;
    shutdown_handles.omni_audit = Some(omni_audit.clone());

    let audit_log = create_audit_log_service(&db);
    shutdown_handles.audit_log = Some(audit_log.clone());

    let retention_days = resolve_audit_retention_days();
    let audit_cleanup = create_audit_cleanup_service(&db, retention_days);

    start_background_tasks(&db, settings);

    let backup_db = connect_backup_database().await;
    let failover_executor = create_failover_executor(&db, backup_db);

    let app_state = build_app_state(
        db,
        omni_audit,
        audit_log,
        audit_cleanup,
        cookie_secret,
        webhook_secret,
        settings,
        failover_executor,
    )?;

    start_failover_monitor(&app_state);
    start_report_subscription_scheduler(&app_state);
    start_color_card_issue_scheduler(&app_state);
    // V15 P1 batch-16 缺陷 6.1/6.2/6.3：邮件队列后台 Worker（扫描 PENDING 邮件 + 指数退避重试）
    start_email_queue_worker(&app_state);
    // V15 P1 10-1/10-2：导出合规审查定时任务（每日扫描 + 6 类异常导出行为识别）
    start_export_compliance_scheduler(&app_state);
    // V15 P1 batch-16 缺陷 8.3/8.4：追踪数据 90 天保留策略（page_views/user_behaviors 归档清理）
    start_tracking_cleanup_scheduler(&app_state);
    // P1 batch-18 缺陷 7.2：库存告警通知调度器（扫描库存告警 + 推送通知）
    start_stock_alert_notification_scheduler(&app_state);
    // 15.2-1：供应商评估定时调度（每季度/每年自动触发评估）
    start_supplier_evaluation_scheduler(&app_state);
    // 16.2-D1：定时推送后台调度（扫描到期推送订阅并触发推送）
    start_notification_push_scheduler(&app_state);
    // V15 P2 B05-P2-7：PDA/工控终端心跳超时清理任务（默认每 60 秒扫描一次超时设备）
    start_device_connection_cleanup_task(&app_state);
    init_event_bus(&app_state, settings).await;
    init_assist_dimensions(&app_state).await;
    init_es_indices().await;
    // V15 P1 20.3-B：启动 WebSocket Redis Pub/Sub 多实例广播订阅器
    let ws_pubsub_handle =
        tokio::spawn(crate::websocket::notifications::start_ws_pubsub_subscriber());
    if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
        tasks.push(ws_pubsub_handle);
    }
    // V15 P1-14.9-C：启动权限缓存 Redis Pub/Sub 订阅器（多实例缓存热更新）
    let perm_pubsub_handle =
        tokio::spawn(crate::middleware::permission::start_permission_cache_pubsub_subscriber());
    if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
        tasks.push(perm_pubsub_handle);
    }
    // V15 P1-14.10-C：启动权限合规审查定时任务（异常权限分配识别 + 定期合规审查）
    start_permission_compliance_review(&app_state);
    // batch-12 P2-8：启动审计日志分级保留清理调度
    start_audit_cleanup_scheduler(&app_state);

    Ok((app_state, shutdown_handles))
}

/// 执行 SeaORM Migration 增加 TOTP 字段及性能优化索引（防御式 IF EXISTS / DO 块）。
async fn run_defensive_migrations(db: &DatabaseConnection) {
    let sql = "
                DO $$
                BEGIN
                    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'users') THEN
                        ALTER TABLE users ADD COLUMN IF NOT EXISTS totp_secret VARCHAR(255);
                        ALTER TABLE users ADD COLUMN IF NOT EXISTS is_totp_enabled BOOLEAN NOT NULL DEFAULT FALSE;
                    END IF;
                    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'sales_orders') THEN
                        CREATE INDEX IF NOT EXISTS idx_sales_order_customer ON sales_orders(customer_id);
                    END IF;
                    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'purchase_order') THEN
                        CREATE INDEX IF NOT EXISTS idx_purchase_order_supplier ON purchase_order(supplier_id);
                    END IF;
                    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'inventory_stocks') THEN
                        CREATE INDEX IF NOT EXISTS idx_inventory_product ON inventory_stocks(product_id, warehouse_id);
                    END IF;
                END $$;
            ";
    if let Err(e) = db.execute_unprepared(sql).await {
        warn!("执行 Migration 失败: {}", e);
    } else {
        info!("成功执行 Migration (TOTP 字段及性能索引)");
    }
}

/// 启动时执行全部 SeaORM 迁移（m0001-m0028，移除 Some(5) 上限避免关键 schema 修复漏掉）。
async fn run_seaorm_migrator(db: &DatabaseConnection) {
    use migration::{Migrator, MigratorTrait};
    tracing::info!("启动时执行数据库迁移（全部 m0001-m0028）...");
    if let Err(e) = Migrator::up(db, None).await {
        tracing::warn!("启动时迁移失败: {}，将在初始化时重试", e);
    } else {
        tracing::info!("数据库迁移执行完成");
    }
    // V15 P1 25.4-J：迁移完成后检查 schema 兼容性（蓝绿部署保障）
    check_migration_compatibility(db).await;
    // batch-17 P3：检查迁移连续性，检测是否有跳跃的迁移版本
    check_migration_continuity(db).await;
}

/// V15 P1 25.4-J：检查数据库迁移兼容性，检测违反蓝绿部署规范的 schema 设计。
///
/// 检测 NOT NULL 无 DEFAULT 的非主键字段（违反规则 1），这些字段会导致
/// 蓝绿部署时旧版本 INSERT 失败。仅 warn 不阻塞启动，由开发者在下一版本修复。
///
/// 排除项：
/// - 主键所在表（主键列 is_nullable='NO' 且 column_default 为 NULL 是正常状态）
/// - 系统列（id/created_at/updated_at 通常由 ORM/trigger 维护，应用层不直接 INSERT）
async fn check_migration_compatibility(db: &DatabaseConnection) {
    let count_sql = "
        SELECT COUNT(*) as count
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND is_nullable = 'NO'
          AND column_default IS NULL
          AND column_name NOT IN ('id', 'created_at', 'updated_at')
          AND table_name NOT IN (
              SELECT DISTINCT tc.table_name
              FROM information_schema.table_constraints tc
              JOIN information_schema.key_column_usage kcu
                ON tc.constraint_name = kcu.constraint_name
              WHERE tc.constraint_type = 'PRIMARY KEY'
                AND tc.table_schema = 'public'
          )
    ";
    match db
        .query_one(Statement::from_string(DatabaseBackend::Postgres, count_sql))
        .await
    {
        Ok(Some(row)) => {
            let count: i64 = row.try_get::<i64>("", "count").unwrap_or(0);
            if count > 0 {
                warn!(
                    violation_count = count,
                    "⚠ 检测到 {} 个 NOT NULL 无 DEFAULT 的非主键字段（违反 V15 P1 25.4-J 迁移兼容性规范）",
                    count
                );
                warn!("  蓝绿部署时旧版本 INSERT 这些字段会失败，请修复迁移使其 NULLABLE 或添加 DEFAULT");
                warn!("  规范文档：backend/migration/src/lib.rs 模块注释");
            } else {
                info!("迁移兼容性检查通过（无 NOT NULL 无 DEFAULT 的非主键字段违规）");
            }
        }
        Ok(None) => {
            warn!("迁移兼容性检查查询无结果（information_schema.columns 异常）");
        }
        Err(e) => {
            warn!(error = %e, "迁移兼容性检查查询失败（不阻塞启动）");
        }
    }
}

/// batch-17 P3: 检查迁移连续性，检测是否有跳跃的迁移版本
///
/// 从 seaql_migrations 表读取已 applied 的迁移，检查编号是否连续。
/// 仅 warn 不阻塞启动，用于发现人为跳过迁移的情况。
async fn check_migration_continuity(db: &DatabaseConnection) {
    use sea_orm::{ConnectionTrait, Statement};

    let sql = "SELECT migration_name FROM seaql_migrations ORDER BY migration_name";
    let result = db
        .query_all(Statement::from_string(sea_orm::DatabaseBackend::Postgres, sql.to_string()))
        .await;

    match result {
        Ok(rows) => {
            let mut migration_numbers: Vec<u32> = Vec::new();
            for row in rows {
                if let Ok(name) = row.try_get::<String>("", "migration_name") {
                    // 提取 mXXXX 编号
                    if let Some(num_str) = name.strip_prefix('m') {
                        if let Some(num) = num_str.split('_').next() {
                            if let Ok(n) = num.parse::<u32>() {
                                migration_numbers.push(n);
                            }
                        }
                    }
                }
            }

            migration_numbers.sort();
            migration_numbers.dedup();

            // 检查连续性
            let mut gaps = Vec::new();
            for i in 1..migration_numbers.len() {
                let prev = migration_numbers[i - 1];
                let curr = migration_numbers[i];
                if curr != prev + 1 {
                    for gap in (prev + 1)..curr {
                        gaps.push(gap);
                    }
                }
            }

            if gaps.is_empty() {
                info!("迁移连续性检查通过（{} 个迁移）", migration_numbers.len());
            } else {
                warn!(
                    "迁移连续性检查发现跳跃：缺失迁移编号 {:?}（已执行 {} 个迁移）",
                    gaps,
                    migration_numbers.len()
                );
            }
        }
        Err(e) => {
            // seaql_migrations 表可能不存在（首次启动），跳过检查
            debug!("迁移连续性检查跳过（表可能不存在）: {}", e);
        }
    }
}

/// 强制要求独立 cookie_secret 配置，禁止降级复用 jwt_secret（Wave B-2 安全修复）。
fn require_cookie_secret(settings: &AppSettings) -> String {
    let cookie_secret = match settings.auth.cookie_secret.clone() {
        Some(secret) => secret,
        None => {
            eprintln!("FATAL: COOKIE_SECRET 环境变量或 auth.cookie_secret 配置必须显式设置");
            eprintln!("FATAL: 出于安全考虑，禁止降级复用 AUTH__JWT_SECRET 作为 Cookie 加密密钥");
            eprintln!("FATAL: 请使用 `openssl rand -hex 32` 生成至少 32 字节的强随机密钥");
            eprintln!(
                "FATAL: 并通过环境变量 COOKIE_SECRET 或 config.yaml 的 auth.cookie_secret 字段注入"
            );
            std::process::exit(1);
        }
    };
    if cookie_secret.len() < 32 {
        eprintln!(
            "FATAL: COOKIE_SECRET 长度不足 32 字节（当前: {} 字节）",
            cookie_secret.len()
        );
        eprintln!("FATAL: 出于安全考虑，禁止以补 0 / 截断等方式弱化 Cookie 加密密钥");
        eprintln!("FATAL: 请使用 `openssl rand -hex 32` 生成至少 32 字节（64 个十六进制字符）的强随机密钥");
        eprintln!(
            "FATAL: 并通过环境变量 COOKIE_SECRET 或 config.yaml 的 auth.cookie_secret 字段注入"
        );
        std::process::exit(1);
    }
    cookie_secret
}

/// 强制要求独立 webhook_secret 配置（M-2 安全修复）。
fn require_webhook_secret(settings: &AppSettings) -> String {
    let webhook_secret = match settings.auth.webhook_secret.clone() {
        Some(secret) => secret,
        None => {
            eprintln!("FATAL: WEBHOOK_SECRET 环境变量或 auth.webhook_secret 配置必须显式设置");
            eprintln!("FATAL: 出于安全考虑，禁止降级复用 JWT_SECRET 作为 Webhook HMAC 密钥");
            eprintln!("FATAL: 请使用 `openssl rand -hex 32` 生成至少 32 字节的强随机密钥");
            eprintln!("FATAL: 并通过环境变量 WEBHOOK_SECRET 或 config.yaml 的 auth.webhook_secret 字段注入");
            std::process::exit(1);
        }
    };
    if webhook_secret.len() < 32 {
        eprintln!(
            "FATAL: WEBHOOK_SECRET 长度不足 32 字节（当前: {} 字节）",
            webhook_secret.len()
        );
        eprintln!("FATAL: 请重新生成强随机密钥并重新启动服务");
        std::process::exit(1);
    }
    webhook_secret
}

/// 创建 OmniAuditEngine 并打印密钥指纹，保留 clone 用于 graceful shutdown。
fn create_omni_audit_service(
    db: &Arc<DatabaseConnection>,
) -> Result<Arc<crate::services::omni_audit_service::OmniAuditEngine>, Box<dyn std::error::Error>> {
    let omni_audit = Arc::new(crate::services::omni_audit_service::OmniAuditEngine::new(
        db.clone(),
    )?);
    tracing::info!(
        fingerprint = %omni_audit.secret_key_fingerprint(),
        "OmniAuditEngine 已初始化（secret_key 指纹前 16 hex 字符）"
    );
    Ok(omni_audit)
}

/// 创建 AuditLogService（mpsc channel 模式），保留 clone 用于 graceful shutdown。
fn create_audit_log_service(
    db: &Arc<DatabaseConnection>,
) -> Arc<crate::services::audit_log_service::AuditLogService> {
    let audit_log = Arc::new(crate::services::audit_log_service::AuditLogService::new(
        db.clone(),
    ));
    tracing::info!("AuditLogService 已初始化（mpsc channel 模式）");
    audit_log
}

/// 解析 AUDIT_RETENTION_DAYS（默认 365，生产环境未设置 warn，开发环境未设置 info）。
fn resolve_audit_retention_days() -> i32 {
    match std::env::var("AUDIT_RETENTION_DAYS") {
        Ok(v) => match v.parse::<i32>() {
            Ok(d) if d > 0 => {
                info!(retention_days = d, "AUDIT_RETENTION_DAYS 已设置");
                d
            }
            _ => {
                warn!(value = %v, "AUDIT_RETENTION_DAYS 值无效（应为正整数），使用默认值 365");
                365
            }
        },
        Err(_) => {
            if crate::utils::config::is_production() {
                warn!("生产环境未设置 AUDIT_RETENTION_DAYS，使用默认值 365（建议显式设置审计日志保留天数）");
            } else {
                info!("AUDIT_RETENTION_DAYS 未设置，使用默认值 365");
            }
            365
        }
    }
}

/// 创建 AuditCleanupService（按 retention_days 自动清理过期审计日志）。
fn create_audit_cleanup_service(
    db: &Arc<DatabaseConnection>,
    retention_days: i32,
) -> Arc<crate::services::audit_cleanup_service::AuditCleanupService> {
    Arc::new(
        crate::services::audit_cleanup_service::AuditCleanupService::new(
            db.clone(),
            retention_days,
        ),
    )
}

/// 启动慢查询采集后台任务（受 settings.slow_query.enabled 配置开关控制）。
fn start_slow_query_collector(db: &Arc<DatabaseConnection>, settings: &AppSettings) {
    if !settings.slow_query.enabled {
        info!("慢查询采集任务已禁用（slow_query.enabled=false）");
        return;
    }
    let slow_collector = Arc::new(
        crate::services::slow_query_collector::SlowQueryCollector::new(
            db.clone(),
            settings.slow_query.threshold_ms,
            settings.slow_query.limit_rows,
        ),
    );
    let slow_handle = slow_collector.clone().start_collect_task(
        settings.slow_query.interval_secs,
        MAIN_CANCELLATION_TOKEN.clone(),
    );
    if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
        tasks.push(slow_handle);
    }
    info!(
        "慢查询采集任务已启动（间隔 {} 秒，阈值 {}ms）",
        settings.slow_query.interval_secs, settings.slow_query.threshold_ms
    );
}

/// 启动 admin 角色缓存清理后台任务（每 10 分钟清理过期条目）。
fn start_admin_cache_cleanup_task() {
    let token = MAIN_CANCELLATION_TOKEN.clone();
    let admin_handle = tokio::spawn(async move {
        let interval = std::time::Duration::from_secs(600);
        loop {
            tokio::select! {
                _ = tokio::time::sleep(interval) => {
                    crate::utils::admin_checker::cleanup_expired_admin_cache();
                    tracing::debug!("admin 角色缓存过期条目清理完成");
                }
                _ = token.cancelled() => {
                    tracing::info!("admin 角色缓存清理任务收到取消信号，优雅退出");
                    break;
                }
            }
        }
    });
    if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
        tasks.push(admin_handle);
    }
    info!("admin 角色缓存清理任务已启动（间隔 600 秒）");
}

/// 启动 JTI 黑名单内存降级路径清理任务（每小时清理过期 JTI）。
fn start_jti_cleanup_task() {
    let token = MAIN_CANCELLATION_TOKEN.clone();
    let jti_handle = tokio::spawn(async move {
        let interval = std::time::Duration::from_secs(3600);
        loop {
            tokio::select! {
                _ = tokio::time::sleep(interval) => {
                    crate::services::auth_service::cleanup_expired_jti(0).await;
                }
                _ = token.cancelled() => {
                    tracing::info!("JTI 黑名单清理任务收到取消信号，优雅退出");
                    break;
                }
            }
        }
    });
    if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
        tasks.push(jti_handle);
    }
    info!("JTI 黑名单清理任务已启动（间隔 3600 秒，Redis 模式下为 noop）");
}

/// 启动 CRM 公海回收规则自动执行任务（每 6 小时扫描一次活跃线索）。
fn start_crm_recycle_task(db: &Arc<DatabaseConnection>) {
    let recycle_executor = std::sync::Arc::new(
        crate::services::crm::recycle_executor::RecycleExecutor::new(db.clone()),
    );
    let recycle_handle = recycle_executor.start_background_task(MAIN_CANCELLATION_TOKEN.clone());
    if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
        tasks.push(recycle_handle);
    }
    info!("CRM 公海回收规则自动执行任务已启动（间隔 6 小时）");
}

/// V15 P1 20.8-B：启动日志文件保留期清理任务（每日扫描 log_dir，删除超过 retention_days 的滚动日志文件）。
fn start_log_cleanup_task(settings: &AppSettings) {
    let log_dir = settings.log.dir.clone();
    let retention_days = settings.log.retention_days;
    let cleanup = std::sync::Arc::new(
        crate::services::log_cleanup_service::LogCleanupService::new(log_dir, retention_days),
    );
    let handle = cleanup.start_cleanup_task(MAIN_CANCELLATION_TOKEN.clone());
    if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
        tasks.push(handle);
    }
}

/// 统一启动后台周期任务（慢查询/admin 缓存/JTI/CRM 回收/日志清理）
fn start_background_tasks(db: &Arc<DatabaseConnection>, settings: &AppSettings) {
    start_slow_query_collector(db, settings);
    start_admin_cache_cleanup_task();
    start_jti_cleanup_task();
    start_crm_recycle_task(db);
    start_log_cleanup_task(settings);
}

/// 连接备库（DATABASE_BACKUP_URL 未配置或失败时返回 None，降级仅主库模式）。
async fn connect_backup_database() -> Option<Arc<sea_orm::DatabaseConnection>> {
    let backup_db_url = std::env::var("DATABASE_BACKUP_URL").unwrap_or_default();
    if backup_db_url.is_empty() {
        info!("DATABASE_BACKUP_URL 未配置，FailoverExecutor 仅主库模式（自动切换将仅更新 status 表，不切换 DB 连接）");
        return None;
    }
    match sea_orm::Database::connect(&backup_db_url).await {
        Ok(conn) => {
            info!("DATABASE_BACKUP_URL 已配置，备库连接成功（FailoverExecutor 启用真实切换）");
            Some(Arc::new(conn))
        }
        Err(e) => {
            warn!(
                error = %e,
                "DATABASE_BACKUP_URL 连接失败，FailoverExecutor 降级为仅主库模式（switch_to_backup 将返回 Err）"
            );
            None
        }
    }
}

/// 创建 FailoverExecutor（主库 + 可选备库）。
fn create_failover_executor(
    db: &Arc<DatabaseConnection>,
    backup_db: Option<Arc<sea_orm::DatabaseConnection>>,
) -> Arc<crate::services::failover_service::FailoverExecutor> {
    Arc::new(crate::services::failover_service::FailoverExecutor::new(
        db.clone(),
        backup_db,
    ))
}

/// 组装 AppStateParams 并创建 AppState（失败时返回错误）。
fn build_app_state(
    db: Arc<DatabaseConnection>,
    omni_audit: Arc<crate::services::omni_audit_service::OmniAuditEngine>,
    audit_log: Arc<crate::services::audit_log_service::AuditLogService>,
    audit_cleanup: Arc<crate::services::audit_cleanup_service::AuditCleanupService>,
    cookie_secret: String,
    webhook_secret: String,
    settings: &AppSettings,
    failover_executor: Arc<crate::services::failover_service::FailoverExecutor>,
) -> Result<AppState, Box<dyn std::error::Error>> {
    let app_state_params = AppStateParams {
        db,
        omni_audit,
        audit_log,
        audit_cleanup,
        jwt_secret: settings.auth.jwt_secret.clone(),
        previous_jwt_secret: settings.auth.previous_jwt_secret.clone(),
        cookie_secret,
        webhook_secret,
        allowed_origins: settings.cors.allowed_origins.clone(),
        failover_executor: failover_executor.clone(),
    };
    match AppState::with_secrets_and_cors(app_state_params) {
        Ok(state) => Ok(state),
        Err(e) => Err(format!("初始化应用全局状态失败: {}", e).into()),
    }
}

/// 启动 FailoverMonitor 后台健康监控任务（5s 探测 + 3 次失败阈值 + 可配自动切换）。
fn start_failover_monitor(app_state: &AppState) {
    let interval_secs = std::env::var("FAILOVER_MONITOR_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(5);
    let failure_threshold = std::env::var("FAILOVER_FAILURE_THRESHOLD")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(3);
    let auto_switch_enabled = std::env::var("FAILOVER_AUTO_SWITCH_ENABLED")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    let monitor_metrics = crate::handlers::failover_handler::get_global_metrics();
    let monitor_service = crate::services::failover_service::FailoverService::new(
        (*app_state.db).clone(),
        monitor_metrics,
    )
    .with_executor(app_state.failover_executor.clone());
    let monitor = crate::services::failover_service::FailoverMonitor::new(
        monitor_service,
        std::time::Duration::from_secs(interval_secs),
        failure_threshold,
        auto_switch_enabled,
    );
    let monitor_handle = tokio::spawn(monitor.run());
    if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
        tasks.push(monitor_handle);
    }
    info!(
        interval_secs,
        failure_threshold,
        auto_switch_enabled,
        "FailoverMonitor 后台健康监控任务已启动（5s 间隔 SELECT 1 探测，连续 3 次失败触发自动切换）"
    );
}

/// 启动报表订阅调度任务（默认每 60 秒扫描到期订阅并发送邮件通知）。
fn start_report_subscription_scheduler(app_state: &AppState) {
    let scheduler = std::sync::Arc::new(
        crate::services::report_subscription_scheduler::ReportSubscriptionScheduler::new(
            app_state.db.clone(),
        ),
    );
    let scheduler_handle = scheduler.start_background_task();
    if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
        tasks.push(scheduler_handle);
    }
    info!("报表订阅调度任务已启动（默认每 60 秒扫描一次到期订阅）");
}

/// 启动色卡发放过期检查调度任务（V15 P1 缺陷 10.5-1）。
/// 默认每 24 小时扫描一次过期发放记录并自动标记为 cancelled，同时恢复色卡库存。
/// 环境变量门控：COLOR_CARD_ISSUE_EXPIRY_CHECK_ENABLED（默认 true）/ COLOR_CARD_ISSUE_EXPIRY_CHECK_INTERVAL_SECS（默认 86400）。
fn start_color_card_issue_scheduler(app_state: &AppState) {
    let scheduler = std::sync::Arc::new(
        crate::services::color_card_issue_scheduler::ColorCardIssueExpiryScheduler::new(
            app_state.db.clone(),
            Some(app_state.audit_log.clone()),
        ),
    );
    let scheduler_handle = scheduler.start_background_task();
    if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
        tasks.push(scheduler_handle);
    }
    info!("色卡发放过期检查调度任务已启动（默认每 24 小时扫描一次过期发放记录）");
}

/// 启动邮件队列后台 Worker（V15 P1 batch-16 缺陷 6.1/6.2/6.3）。
///
/// 默认每 60 秒扫描一次 PENDING 邮件并通过 EmailService 实际发送：
/// - 缺陷 6.1 修复：send_email 入口仅入队，实际发送由本 Worker 异步执行
/// - 缺陷 6.2 修复：失败时按指数退避（60s/300s/1800s）重试，超过 3 次转入 FAILED 死信
/// - 缺陷 6.3 修复：附件通过 SendGrid base64 编码方式发送
///
/// 环境变量门控：
/// - `EMAIL_QUEUE_WORKER_ENABLED`（默认 "true"）— 设为 "false" / "0" 时跳过启动
/// - `EMAIL_QUEUE_WORKER_INTERVAL_SECS`（默认 60）— 扫描间隔
fn start_email_queue_worker(app_state: &AppState) {
    let worker = std::sync::Arc::new(crate::services::email_queue_worker::EmailQueueWorker::new(
        app_state.db.clone(),
    ));
    let worker_handle = worker.start_background_task();
    if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
        tasks.push(worker_handle);
    }
    info!("邮件队列后台 Worker 已启动（默认每 60 秒扫描一次 PENDING 邮件，含指数退避重试）");
}

/// 启动导出合规审查定时任务（V15 P1 缺陷 10-1/10-2）。
///
/// 默认每 24 小时执行一次合规审查，扫描前一天的 print/export 操作并识别 6 类异常：
/// 高频导出 / 大批量导出 / 非工作时间导出 / 离职用户导出 / 跨权限导出 / 敏感数据无审批导出。
///
/// 环境变量门控：
/// - `EXPORT_COMPLIANCE_CHECK_ENABLED`（默认 true）/ `EXPORT_COMPLIANCE_CHECK_INTERVAL_SECS`（默认 86400）。
fn start_export_compliance_scheduler(app_state: &AppState) {
    let service = std::sync::Arc::new(
        crate::services::export_compliance_service::ExportComplianceService::new(
            app_state.db.clone(),
            app_state.audit_log.clone(),
        ),
    );
    let handle = service.start_background_task();
    if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
        tasks.push(handle);
    }
    info!("导出合规审查定时任务已启动（默认每 24 小时扫描前一天 print/export 操作，识别 6 类异常行为）");
}

/// 启动追踪数据 90 天保留策略定时任务（V15 P1 batch-16 缺陷 8.3/8.4）。
///
/// 默认每 24 小时执行一次清理，将超过 retention_days 的 page_views / user_behaviors
/// 明细按 (date, path|event_type) 聚合到 page_view_daily_summary /
/// user_behavior_daily_summary 后批量删除明细，避免明细表无限膨胀。
///
/// 环境变量门控：
/// - `TRACKING_CLEANUP_ENABLED`（默认 true）/ `TRACKING_CLEANUP_INTERVAL_SECS`（默认 86400）
/// - `TRACKING_RETENTION_DAYS`（默认 90，对应《个人信息保护法》数据最小化原则）
fn start_tracking_cleanup_scheduler(app_state: &AppState) {
    let retention_days = std::env::var("TRACKING_RETENTION_DAYS")
        .ok()
        .and_then(|v| v.parse::<i32>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(crate::services::tracking_cleanup_service::DEFAULT_RETENTION_DAYS);
    let service = std::sync::Arc::new(
        crate::services::tracking_cleanup_service::TrackingCleanupService::new(
            app_state.db.clone(),
            retention_days,
        ),
    );
    let handle = service.start_background_task();
    if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
        tasks.push(handle);
    }
    info!(
        retention_days,
        "追踪数据 90 天保留策略任务已启动（默认每 24 小时扫描一次过期 page_views/user_behaviors 明细并归档）"
    );
}

/// P1 batch-18 缺陷 7.2：启动库存告警通知调度器
fn start_stock_alert_notification_scheduler(app_state: &AppState) {
    let scheduler = std::sync::Arc::new(
        crate::services::stock_alert_notification_scheduler::StockAlertNotificationScheduler::new(
            app_state.db.clone(),
        ),
    );
    let handle = scheduler.start_background_task();
    if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
        tasks.push(handle);
    }
    info!("库存告警通知调度器已启动（默认每 6 小时扫描全量库存并推送告警通知）");
}

/// 15.2-1：启动供应商评估定时调度任务（每季度/每年自动触发评估）。
fn start_supplier_evaluation_scheduler(app_state: &AppState) {
    let handle = crate::services::supplier_evaluation_service::SupplierEvaluationService::start_evaluation_scheduler(
        app_state.db.clone(),
        MAIN_CANCELLATION_TOKEN.clone(),
    );
    if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
        tasks.push(handle);
    }
    info!("供应商评估定时调度任务已启动（默认每 24 小时检查评估触发条件）");
}

/// 16.2-D1：启动定时推送后台调度任务（扫描到期推送订阅并触发推送）。
fn start_notification_push_scheduler(app_state: &AppState) {
    let scheduler = std::sync::Arc::new(
        crate::services::notification_scheduler::NotificationPushScheduler::new(
            app_state.db.clone(),
        ),
    );
    let handle = scheduler.start_background_task();
    if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
        tasks.push(handle);
    }
    info!("定时推送后台调度任务已启动（默认每 60 秒扫描一次到期推送订阅）");
}

/// B05-P2-7：启动设备连接心跳超时清理任务（默认 60s 扫描，超时标记 timeout）。
// 环境变量门控：DEVICE_CONNECTION_CLEANUP_ENABLED(默认true) / DEVICE_HEARTBEAT_TIMEOUT_SECS(默认300) / DEVICE_CONNECTION_CLEANUP_INTERVAL_SECS(默认60)
fn start_device_connection_cleanup_task(app_state: &AppState) {
    let db = app_state.db.clone();
    let token = MAIN_CANCELLATION_TOKEN.clone();
    let handle = tokio::spawn(async move {
        let enabled = std::env::var("DEVICE_CONNECTION_CLEANUP_ENABLED")
            .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes" | "on"))
            .unwrap_or(true);
        if !enabled {
            info!("设备连接超时清理：环境变量 DEVICE_CONNECTION_CLEANUP_ENABLED=false，跳过启动");
            return;
        }
        let timeout_secs = std::env::var("DEVICE_HEARTBEAT_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|&v| v > 0)
            .unwrap_or(crate::services::device_connection_service::DEFAULT_HEARTBEAT_TIMEOUT_SECS);
        let interval_secs = std::env::var("DEVICE_CONNECTION_CLEANUP_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|&v| v > 0)
            .unwrap_or(crate::services::device_connection_service::DEFAULT_CLEANUP_INTERVAL_SECS);

        let interval = std::time::Duration::from_secs(interval_secs);
        info!(
            interval_secs,
            timeout_secs,
            "设备连接超时清理任务已启动（每 {} 秒扫描一次，心跳超过 {} 秒未上报则置 timeout）",
            interval_secs,
            timeout_secs
        );

        let service = crate::services::device_connection_service::DeviceConnectionService::new(db);
        loop {
            tokio::select! {
                _ = tokio::time::sleep(interval) => {
                    match service.cleanup_timeout(timeout_secs).await {
                        Ok(count) if count > 0 => {
                            info!(count, "设备连接超时清理：本轮标记 {} 台设备为 timeout", count);
                        }
                        Ok(_) => {
                            // 无超时设备，静默
                        }
                        Err(e) => {
                            warn!(error = %e, "设备连接超时清理：本轮扫描失败，下次循环继续");
                        }
                    }
                }
                _ = token.cancelled() => {
                    info!("设备连接超时清理任务收到取消信号，优雅退出");
                    break;
                }
            }
        }
    });
    if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
        tasks.push(handle);
    }
}

/// 启动事件总线监听器并按 Kafka 配置初始化事件总线。
async fn init_event_bus(app_state: &AppState, settings: &AppSettings) {
    crate::services::event_bus::start_event_listener(
        app_state.db.clone(),
        app_state.search_client.clone(),
    )
    .await;
    crate::services::event_bus::init_event_bus_with_kafka_config(&settings.kafka).await;
}

/// V15 P1-14.10-C：启动权限合规审查定时任务（每 7 天扫描权限变更日志，识别 6 类异常行为）。
fn start_permission_compliance_review(app_state: &AppState) {
    let service = std::sync::Arc::new(
        crate::services::permission_compliance_service::PermissionComplianceService::new(
            app_state.db.clone(),
            app_state.audit_log.clone(),
        ),
    );
    let handle = service.start_periodic_review(MAIN_CANCELLATION_TOKEN.clone());
    if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
        tasks.push(handle);
    }
    info!("权限合规审查定时任务已启动（14.10-C，受 MAIN_CANCELLATION_TOKEN 控制）");
}

/// batch-12 P2-8：启动审计日志分级保留清理调度（标准 scheduler 模式）
fn start_audit_cleanup_scheduler(app_state: &AppState) {
    let retention_days = resolve_audit_retention_days();
    let service = std::sync::Arc::new(
        crate::services::audit_cleanup_service::AuditCleanupService::new(
            app_state.db.clone(),
            retention_days,
        ),
    );
    let handle = service.start_cleanup_task(MAIN_CANCELLATION_TOKEN.clone());
    if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
        tasks.push(handle);
    }
    info!(
        retention_days,
        "审计日志分级保留清理调度已启动（omni_audit_logs/audit_logs {}天, permission_change_audits/security_alert_logs 7年）",
        retention_days
    );
}

/// 启动时初始化 8 个辅助核算维度（幂等实现，失败仅 warn 不阻塞启动）。
async fn init_assist_dimensions(app_state: &AppState) {
    let assist_svc = crate::services::assist_accounting_service::AssistAccountingService::new(
        app_state.db.clone(),
    );
    if let Err(e) = assist_svc.initialize_dimensions().await {
        tracing::warn!(
            error = %e,
            "辅助核算维度初始化失败（不阻塞启动，后续可手工插入维度记录）"
        );
    } else {
        tracing::info!(
            "辅助核算维度初始化完成（8 个维度：批次/色号/缸号/等级/车间/仓库/客户/供应商）"
        );
    }
}

/// 启动时确保 ES 索引存在（幂等创建，仅在 ELASTICSEARCH_URL 配置时调用）。
async fn init_es_indices() {
    let es_url = match std::env::var("ELASTICSEARCH_URL") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            if crate::utils::config::is_production() {
                warn!("生产环境未设置 ELASTICSEARCH_URL，搜索功能将使用 mock 客户端（建议配置可达的 ES 服务地址）");
            } else {
                info!("ELASTICSEARCH_URL 未设置，搜索功能使用 mock 客户端（开发/测试环境）");
            }
            return;
        }
    };
    if let Err(e) = crate::search::ensure_indices(&es_url).await {
        tracing::warn!(
            error = %e,
            url = %es_url,
            "ES 索引初始化失败（不阻塞启动，后续可手动 PUT mapping）"
        );
    } else {
        tracing::info!("ES 索引初始化完成（3 个索引：sales_orders/customers/products）");
    }
}
