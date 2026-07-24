//! 服务初始化（数据库迁移 / 服务创建 / 后台任务 / AppState 组装）
//!
//! 职责：数据库连接成功后，执行防御式迁移、创建审计/清理/故障转移等服务、
//! 启动后台定时任务、组装 AppState，并返回 graceful shutdown 所需的服务句柄。

use std::io::Write;
use std::sync::Arc;

use sea_orm::{ConnectionTrait, DatabaseConnection};
use tracing::{info, warn};

use crate::config::settings::AppSettings;
use crate::utils::app_state::{AppState, AppStateParams};

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
    /// 关闭所有持有的服务（幂等安全）。
    pub fn shutdown(self) {
        // L-30 修复（批次 372 v13 复审）：关闭 OmniAuditEngine（mpsc channel + handle abort）
        if let Some(omni_audit) = self.omni_audit {
            omni_audit.shutdown();
        }
        // L-32 修复（批次 380 v13 复审）：关闭 AuditLogService（mpsc channel + handle abort）
        if let Some(audit_log) = self.audit_log {
            audit_log.shutdown();
        }
    }
}

/// L-26 修复（批次 374 v13 复审）：main.rs 后台定时任务 spawn 句柄
/// 保存 admin 缓存清理 + JTI 黑名单清理 + 慢查询采集句柄，供 shutdown abort
static MAIN_BACKGROUND_TASKS: std::sync::Mutex<Vec<tokio::task::JoinHandle<()>>> =
    std::sync::Mutex::new(Vec::new());

/// L-26 修复（批次 374）：关闭 main.rs 后台定时任务，幂等安全
pub fn shutdown_main_background_tasks() {
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
    info!("main 后台定时任务已关闭（{} 个）", count);
}

/// 完整模式启动：数据库已连接后执行迁移、创建服务、启动后台任务、组装 AppState。
///
/// 返回 `(AppState, BootstrapShutdownHandles)`，后者用于 graceful shutdown 时
/// 关闭审计引擎和审计日志服务。
pub async fn bootstrap_full_mode(
    db: DatabaseConnection,
    settings: &AppSettings,
) -> Result<(AppState, BootstrapShutdownHandles), Box<dyn std::error::Error>> {
    // 执行 SeaORM Migration 增加 TOTP 字段及性能优化索引
    // 防御式迁移：使用 IF EXISTS / DO 块确保表不存在时不会阻断服务启动
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

    // P0-A 数据库迁移根治：启动时执行全部迁移（m0001-m0028）
    // 修复策略：移除 Some(5) 上限限制，让 Migrator::up 跑完所有 migration，
    // 避免 m0019_add_missing_columns 等关键 schema 修复被漏掉。
    // 全新部署时按编号顺序完整执行；已部署时 SeaORM 按名称去重。
    use migration::{Migrator, MigratorTrait};
    tracing::info!("启动时执行数据库迁移（全部 m0001-m0028）...");
    if let Err(e) = Migrator::up(&db, None).await {
        tracing::warn!("启动时迁移失败: {}，将在初始化时重试", e);
    } else {
        tracing::info!("数据库迁移执行完成");
    }

    std::io::stdout().flush().ok();
    std::io::stderr().flush().ok();

    // Wave B-2 修复（B2-1）：强制要求独立的 cookie_secret 配置，禁止降级复用 jwt_secret
    // 安全原因：JWT 与 Cookie 使用相同密钥会同时暴露两个攻击面（签名伪造 + Cookie 加密泄露），
    // 违反最小权限原则，且多副本部署时若运维误改 JWT 会同步影响 Cookie 加密强度。
    // 强制要求通过环境变量 COOKIE_SECRET 或配置项 auth.cookie_secret 显式注入。
    let cookie_secret = match settings.auth.cookie_secret.clone() {
        Some(secret) => secret,
        None => {
            eprintln!("FATAL: COOKIE_SECRET 环境变量或 auth.cookie_secret 配置必须显式设置");
            eprintln!("FATAL: 出于安全考虑，禁止降级复用 AUTH__JWT_SECRET 作为 Cookie 加密密钥");
            eprintln!("FATAL: 请使用 `openssl rand -hex 32` 生成至少 32 字节的强随机密钥");
            eprintln!("FATAL: 并通过环境变量 COOKIE_SECRET 或 config.yaml 的 auth.cookie_secret 字段注入");
            std::process::exit(1);
        }
    };

    if cookie_secret.len() < 32 {
        eprintln!("FATAL: COOKIE_SECRET 长度不足 32 字节（当前: {} 字节）", cookie_secret.len());
        eprintln!("FATAL: 出于安全考虑，禁止以补 0 / 截断等方式弱化 Cookie 加密密钥");
        eprintln!("FATAL: 请使用 `openssl rand -hex 32` 生成至少 32 字节（64 个十六进制字符）的强随机密钥");
        eprintln!("FATAL: 并通过环境变量 COOKIE_SECRET 或 config.yaml 的 auth.cookie_secret 字段注入");
        std::process::exit(1);
    }

    // M-2 修复：要求独立的 webhook_secret 配置
    // 安全原因：JWT_SECRET 一旦泄露（环境变量备份、容器镜像层泄漏）
    // 会导致第三方 webhook 回调被任意伪造。强制要求独立密钥。
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
        eprintln!("FATAL: WEBHOOK_SECRET 长度不足 32 字节（当前: {} 字节）", webhook_secret.len());
        eprintln!("FATAL: 请重新生成强随机密钥并重新启动服务");
        std::process::exit(1);
    }
    let db = Arc::new(db);

    let omni_audit = Arc::new(crate::services::omni_audit_service::OmniAuditEngine::new(
        db.clone(),
    )?);
    // v11 批次 154c P2-A：启动时打印密钥指纹，供运维核对 AUDIT_SECRET_KEY 配置
    tracing::info!(
        fingerprint = %omni_audit.secret_key_fingerprint(),
        "OmniAuditEngine 已初始化（secret_key 指纹前 16 hex 字符）"
    );
    // L-30 修复（批次 372 v13 复审）：保留 omni_audit clone 用于 graceful shutdown 后
    // 调用 shutdown()，避免审计引擎 detached task 泄漏
    let mut shutdown_handles = BootstrapShutdownHandles::default();
    shutdown_handles.omni_audit = Some(omni_audit.clone());

    // L-32 修复（批次 380 v13 复审）：创建 AuditLogService（mpsc channel + handle 保存）
    let audit_log = Arc::new(crate::services::audit_log_service::AuditLogService::new(db.clone()));
    tracing::info!("AuditLogService 已初始化（mpsc channel 模式）");
    // L-32 修复：保留 audit_log clone 用于 graceful shutdown 后调用 shutdown()
    shutdown_handles.audit_log = Some(audit_log.clone());

    // P2 8-13 修复：原 retention_days 硬编码 999（约 2.7 年），实际无清理效果，
    // omni_audit_logs 表无限膨胀拖累查询性能。
    // 改为环境变量配置，默认 365 天（1 年热保留），符合审计日志保留最佳实践。
    // 生产环境可通过 AUDIT_RETENTION_DAYS 覆盖；归档逻辑（1-3 年冷数据迁移）
    // 作为后续技术债单独实现。
    // L-37 修复（批次 379 v13 复审）：消除 silent default，
    // 生产环境未设置时 warn，开发环境未设置时 info。
    let retention_days = match std::env::var("AUDIT_RETENTION_DAYS") {
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
    };
    let audit_cleanup = Arc::new(
        crate::services::audit_cleanup_service::AuditCleanupService::new(
            db.clone(),
            retention_days,
        ),
    );

    // P13 批 1 B-慢查询审计：启动后台采集任务（默认 5 分钟间隔）。
    // 部署-3 修复：增加 slow_query.enabled 配置开关。
    // 关闭时（CI 容器 / 未安装 pg_stat_statements 扩展的数据库）完全跳过采集任务。
    // 开启时（默认）按配置间隔采集，失败仅记录日志，不阻断 main 启动。
    if settings.slow_query.enabled {
        let slow_collector = Arc::new(
            crate::services::slow_query_collector::SlowQueryCollector::new(
                db.clone(),
                settings.slow_query.threshold_ms,
                settings.slow_query.limit_rows,
            ),
        );
        // L-26 修复（批次 374）：保存慢查询采集句柄供 shutdown abort
        let slow_handle = slow_collector
            .clone()
            .start_collect_task(settings.slow_query.interval_secs);
        if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
            tasks.push(slow_handle);
        }
        info!(
            "慢查询采集任务已启动（间隔 {} 秒，阈值 {}ms）",
            settings.slow_query.interval_secs, settings.slow_query.threshold_ms
        );
    } else {
        info!("慢查询采集任务已禁用（slow_query.enabled=false）");
    }

    // v11 批次 156 P2-D：启动 admin 角色缓存清理后台任务（每 10 分钟清理过期条目）
    {
        // L-26 修复（批次 374）：保存句柄供 shutdown abort
        let admin_handle = tokio::spawn(async move {
            let interval = std::time::Duration::from_secs(600);
            loop {
                tokio::time::sleep(interval).await;
                crate::utils::admin_checker::cleanup_expired_admin_cache();
                tracing::debug!("admin 角色缓存过期条目清理完成");
            }
        });
        if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
            tasks.push(admin_handle);
        }
        info!("admin 角色缓存清理任务已启动（间隔 600 秒）");
    }

    // 批次 349 v12 复审 P2-3：启动 JTI 黑名单内存降级路径清理任务
    // L-26 修复（批次 374）：保存句柄供 shutdown abort
    {
        let jti_handle = tokio::spawn(async move {
            let interval = std::time::Duration::from_secs(3600);
            loop {
                tokio::time::sleep(interval).await;
                crate::services::auth_service::cleanup_expired_jti(0).await;
            }
        });
        if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
            tasks.push(jti_handle);
        }
        info!("JTI 黑名单清理任务已启动（间隔 3600 秒，Redis 模式下为 noop）");
    }

    // V15 P0-B07（Batch 482）：CRM 公海回收规则自动执行任务
    // 设计依据：审计报告 §18.3-D1 — 回收规则仅有 CRUD 无自动执行
    // 每 6 小时扫描一次超过 N 天未跟进的活跃线索，自动回收到公海
    // L-26 修复（批次 374）：保存句柄供 shutdown abort
    {
        let recycle_executor = std::sync::Arc::new(
            crate::services::crm::recycle_executor::RecycleExecutor::new(
                db.clone(),
            ),
        );
        let recycle_handle =
            recycle_executor.start_background_task();
        if let Ok(mut tasks) = MAIN_BACKGROUND_TASKS.lock() {
            tasks.push(recycle_handle);
        }
        info!("CRM 公海回收规则自动执行任务已启动（间隔 6 小时）");
    }

    // V15 P0-B17（Batch 484）：构造 FailoverExecutor
    // - 主库 = 当前 db（已连接的主库连接）
    // - 备库 = DATABASE_BACKUP_URL 环境变量指定的备库（可选，best-effort 连接）
    //   未设置 / 连接失败 → backup = None（switch_to_backup 返回 Err，降级为仅更新 status 表）
    let backup_db_url = std::env::var("DATABASE_BACKUP_URL").unwrap_or_default();
    let backup_db: Option<Arc<sea_orm::DatabaseConnection>> = if !backup_db_url.is_empty() {
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
    } else {
        info!("DATABASE_BACKUP_URL 未配置，FailoverExecutor 仅主库模式（自动切换将仅更新 status 表，不切换 DB 连接）");
        None
    };
    let failover_executor = Arc::new(
        crate::services::failover_service::FailoverExecutor::new(
            db.clone(),
            backup_db,
        ),
    );

    // 批次 331 v10 复审 P3 修复：使用 AppStateParams 参数对象替代多参数
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
        // V15 P0-B17（Batch 484）：注入 FailoverExecutor
        failover_executor: failover_executor.clone(),
    };
    let app_state = match AppState::with_secrets_and_cors(app_state_params) {
        Ok(state) => state,
        Err(e) => {
            return Err(format!("初始化应用全局状态失败: {}", e).into());
        }
    };

    // V15 P0-B16（Batch 484）：启动 FailoverMonitor 后台健康监控任务
    // - 每 5s（FAILOVER_MONITOR_INTERVAL_SECS 可配）执行 SELECT 1 健康探测
    // - 连续 3 次（FAILOVER_FAILURE_THRESHOLD 可配）失败触发自动切换
    // - FAILOVER_AUTO_SWITCH_ENABLED=true 时才真正调用 test_switch（默认 false 仅记录日志）
    // L-26 修复（批次 374）：保存句柄供 shutdown abort
    {
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

        let monitor_metrics =
            crate::handlers::failover_handler::get_global_metrics();
        let monitor_service =
            crate::services::failover_service::FailoverService::new(
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

    // P0-D16（Batch 488）：报表订阅调度任务
    // 设计依据：审计报告 batch-16 P0-16-1 — report_subscription 有 next_run_at 字段但无后台调度
    // 每 60 秒扫描 next_run_at <= now 的启用订阅，发送邮件通知 + 更新执行状态
    // 环境变量门控：REPORT_SUBSCRIPTION_SCHEDULER_ENABLED（默认 true）/ REPORT_SUBSCRIPTION_SCHEDULER_INTERVAL_SECS（默认 60）
    {
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

    crate::services::event_bus::start_event_listener(app_state.db.clone(), app_state.search_client.clone()).await;
    crate::services::event_bus::init_event_bus_with_kafka_config(&settings.kafka).await;
    // 批次 120 P2-7 修复：启动时初始化 8 个辅助核算维度（幂等实现）
    // 原方法保留 `#[allow(dead_code)]` 标记，违反规则 0（真实实现强制）。
    // 此处接入 main.rs 启动流程：服务启动时调用一次 initialize_dimensions，
    // 内部先检查每个维度是否存在再插入，重启不会重复创建。
    // 错误处理：用 tracing::warn! 降级（不阻塞启动），与 init_event_bus 一致。
    let assist_svc = crate::services::assist_accounting_service::AssistAccountingService::new(
        app_state.db.clone(),
    );
    if let Err(e) = assist_svc.initialize_dimensions().await {
        tracing::warn!(
            error = %e,
            "辅助核算维度初始化失败（不阻塞启动，后续可手工插入维度记录）"
        );
    } else {
        tracing::info!("辅助核算维度初始化完成（8 个维度：批次/色号/缸号/等级/车间/仓库/客户/供应商）");
    }
    // 批次 123 v8 复审 P1 修复：启动时确保 ES 索引存在（幂等创建）
    // 仅在配置了 ELASTICSEARCH_URL 时调用，CI 环境（未配置）跳过。
    // 错误处理：用 tracing::warn! 降级（不阻塞启动），与 initialize_dimensions 一致。
    // L-39 修复（批次 379 v13 复审）：消除 silent default，
    // 生产环境未设置时 warn，开发环境未设置时 info。
    let es_url = match std::env::var("ELASTICSEARCH_URL") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            if crate::utils::config::is_production() {
                warn!("生产环境未设置 ELASTICSEARCH_URL，搜索功能将使用 mock 客户端（建议配置可达的 ES 服务地址）");
            } else {
                info!("ELASTICSEARCH_URL 未设置，搜索功能使用 mock 客户端（开发/测试环境）");
            }
            String::new()
        }
    };
    if !es_url.is_empty() {
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

    Ok((app_state, shutdown_handles))
}
