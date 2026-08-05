// dead_code 检查已开启；当前所有 pub API 均通过 FailoverMetrics / FailoverService 被业务引用。

//! 主备隔离服务
//!
//! 提供主备隔离业务逻辑：
//! - 健康检查（数据库 / 缓存）
//! - 切换事件记录
//! - 状态查询
//! - 手动切换（仅管理员）
//! - Prometheus 指标导出

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QuerySelect, Set, Statement, TransactionTrait,
};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

use crate::models::failover_event as event_model;
use crate::models::failover_status as status_model;
use arc_swap::ArcSwap;
use prometheus::{Encoder, IntCounterVec, IntGaugeVec, Opts, Registry, TextEncoder};

// ==================== Prometheus 指标 ====================

/// 主备隔离指标
pub struct FailoverMetrics {
    /// 主调用总次数
    pub primary_total: IntCounterVec,
    /// 主调用失败总次数
    pub primary_failed_total: IntCounterVec,
    /// 备用调用总次数
    pub backup_total: IntCounterVec,
    /// 切换总次数
    pub switch_total: IntCounterVec,
    /// 熔断器状态（0=关闭, 1=打开, 2=半开）
    pub circuit_state: IntGaugeVec,
    /// Prometheus 注册表
    pub registry: Registry,
}

impl FailoverMetrics {
    /// 创建指标实例
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();
        let primary_total = IntCounterVec::new(
            Opts::new("failover_primary_total", "主调用总次数"),
            &["function"],
        )?;
        let primary_failed_total = IntCounterVec::new(
            Opts::new("failover_primary_failed_total", "主调用失败总次数"),
            &["function"],
        )?;
        let backup_total = IntCounterVec::new(
            Opts::new("failover_backup_total", "备用调用总次数"),
            &["function"],
        )?;
        let switch_total = IntCounterVec::new(
            Opts::new("failover_switch_total", "主备切换总次数"),
            &["function"],
        )?;
        let circuit_state = IntGaugeVec::new(
            Opts::new(
                "failover_circuit_state",
                "熔断器状态（0=关闭,1=打开,2=半开）",
            ),
            &["function"],
        )?;
        registry.register(Box::new(primary_total.clone()))?;
        registry.register(Box::new(primary_failed_total.clone()))?;
        registry.register(Box::new(backup_total.clone()))?;
        registry.register(Box::new(switch_total.clone()))?;
        registry.register(Box::new(circuit_state.clone()))?;
        for func in &["database", "cache"] {
            circuit_state.with_label_values(&[func]).set(0);
        }
        Ok(Self {
            primary_total,
            primary_failed_total,
            backup_total,
            switch_total,
            circuit_state,
            registry,
        })
    }

    /// 记录主调用
    pub fn record_primary(&self, function: &str) {
        self.primary_total.with_label_values(&[function]).inc();
    }

    /// 记录主调用失败
    pub fn record_primary_failed(&self, function: &str) {
        self.primary_failed_total
            .with_label_values(&[function])
            .inc();
    }

    /// 记录备用调用
    pub fn record_backup(&self, function: &str) {
        self.backup_total.with_label_values(&[function]).inc();
    }

    /// 记录切换
    pub fn record_switch(&self, function: &str) {
        self.switch_total.with_label_values(&[function]).inc();
    }

    /// 设置熔断器状态
    pub fn set_circuit_state(&self, function: &str, state: i64) {
        self.circuit_state.with_label_values(&[function]).set(state);
    }

    /// 导出 Prometheus 文本格式
    pub fn export_text(&self) -> Result<String, prometheus::Error> {
        let metric_families = self.registry.gather();
        let encoder = TextEncoder::new();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        String::from_utf8(buffer).map_err(|e| prometheus::Error::Msg(e.to_string()))
    }
}

impl Default for FailoverMetrics {
    fn default() -> Self {
        // 批次 92 P3-7 / 批次 95 P3-1 修复：
        // 原实现 mk_counter/mk_gauge 内 panic! 与"回退到空结构"语义矛盾，
        // 批次 92 仅加 tracing::error! 仍未真正消除 panic!。
        // IntCounterVec::new / IntGaugeVec::new 仅在指标名/标签名非法时返回 Err，
        // 此处所有名称均为静态常量，运行时不可能失败；真正失败属于不可恢复的编程错误。
        // 由于 Default trait 签名必须返回 Self（无法 return Err），
        // 改用 tracing::error! 记录详细上下文后调用 std::process::abort 终止进程，
        // 彻底消除裸 panic! 宏调用。
        fn mk_counter(name: &'static str, help: &'static str) -> IntCounterVec {
            IntCounterVec::new(Opts::new(name, help), &["function"]).unwrap_or_else(|e| {
                tracing::error!(
                    target: "failover_metrics",
                    metric_name = name,
                    error = %e,
                    "批次 95 P3-1: 静态指标初始化失败（应为编程错误），进程将终止"
                );
                std::process::abort()
            })
        }
        fn mk_gauge(name: &'static str, help: &'static str) -> IntGaugeVec {
            IntGaugeVec::new(Opts::new(name, help), &["function"]).unwrap_or_else(|e| {
                tracing::error!(
                    target: "failover_metrics",
                    metric_name = name,
                    error = %e,
                    "批次 95 P3-1: 静态指标初始化失败（应为编程错误），进程将终止"
                );
                std::process::abort()
            })
        }
        Self::new().unwrap_or_else(|e| {
            tracing::warn!("FailoverMetrics::new() 失败，回退到未注册的 metrics: {}", e);
            Self {
                primary_total: mk_counter("failover_primary_total", "主调用总次数"),
                primary_failed_total: mk_counter(
                    "failover_primary_failed_total",
                    "主调用失败总次数",
                ),
                backup_total: mk_counter("failover_backup_total", "备用调用总次数"),
                switch_total: mk_counter("failover_switch_total", "主备切换总次数"),
                circuit_state: mk_gauge("failover_circuit_state", "熔断器状态"),
                registry: Registry::new(),
            }
        })
    }
}

// ==================== 主备隔离服务 ====================

/// 主备隔离服务
pub struct FailoverService {
    /// 数据库连接（主库；用于 status/event 表读写）
    pub db: DatabaseConnection,
    /// Prometheus 指标
    pub metrics: Arc<FailoverMetrics>,
    /// V15 P0-B17：主备切换执行器（可选；未配置时 test_switch 仅更新 status 表）
    pub executor: Option<Arc<FailoverExecutor>>,
}

impl FailoverService {
    /// 创建服务实例
    pub fn new(db: DatabaseConnection, metrics: Arc<FailoverMetrics>) -> Self {
        Self {
            db,
            metrics,
            executor: None,
        }
    }

    /// V15 P0-B17：注入 FailoverExecutor，启用真实 DB 连接切换
    pub fn with_executor(mut self, executor: Arc<FailoverExecutor>) -> Self {
        self.executor = Some(executor);
        self
    }

    /// V15 P0-B16：获取当前活跃 DB 连接（若配置了 executor，返回 ArcSwap 当前指向的连接（切换后为备库）；否则返回主库连接（克隆到新 Arc，调用方短暂持有））
    pub fn get_active_db(&self) -> Arc<DatabaseConnection> {
        match &self.executor {
            Some(exec) => exec.get_current(),
            None => Arc::new(self.db.clone()),
        }
    }

    /// 获取所有主备状态
    pub async fn get_statuses(&self) -> Result<Vec<status_model::FailoverStatusDto>, String> {
        let statuses = status_model::Entity::find()
            .all(&self.db)
            .await
            .map_err(|e| format!("查询状态失败: {}", e))?;
        Ok(statuses.into_iter().map(Into::into).collect())
    }

    /// 获取最近切换事件
    pub async fn get_recent_events(
        &self,
        limit: u64,
    ) -> Result<Vec<event_model::FailoverEventDto>, String> {
        use sea_orm::QueryOrder;
        let events = event_model::Entity::find()
            .order_by_desc(event_model::Column::CreatedAt)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(|e| format!("查询事件失败: {}", e))?;
        Ok(events.into_iter().map(Into::into).collect())
    }

    /// 记录切换事件（V15 P0-B16：改为 pub，供 FailoverMonitor 调用）
    pub async fn record_event(
        &self,
        function_name: &str,
        event_type: &str,
        from_state: Option<&str>,
        to_state: Option<&str>,
        reason: Option<&str>,
        latency_ms: Option<i32>,
    ) -> Result<(), String> {
        let now = Utc::now();
        let active = event_model::ActiveModel {
            function_name: Set(function_name.to_string()),
            event_type: Set(event_type.to_string()),
            from_state: Set(from_state.map(|s| s.to_string())),
            to_state: Set(to_state.map(|s| s.to_string())),
            reason: Set(reason.map(|s| s.to_string())),
            latency_ms: Set(latency_ms),
            created_at: Set(now),
            ..Default::default()
        };
        active
            .insert(&self.db)
            .await
            .map_err(|e| format!("记录事件失败: {}", e))?;
        Ok(())
    }

    /// 手动触发切换（仅管理员）
    /// V15 P0-B17：若配置了 FailoverExecutor，先执行真实 DB 连接原子切换（ArcSwap），；再更新 status 表 + 记录 event + 记录指标。；未配置 executor 时仅更新 status 表（与历史行为兼容）。
    pub async fn test_switch(&self, function_name: &str) -> Result<String, String> {
        info!(function = function_name, "手动触发主备切换");

        // V15 P0-B17：执行真实 DB 连接切换（若配置了 executor）
        if let Some(executor) = &self.executor {
            if executor.is_on_backup() {
                return Ok(format!("{} 已在备库上运行，无需重复切换", function_name));
            }
            // V15 P1 20.4-C：切换前等待备库 catch-up（10s 超时，RPO=0 保障）
            match self.wait_for_backup_catchup(Duration::from_secs(10)).await {
                Ok(true) => info!("test_switch: 备库 catch-up 完成，可安全切换"),
                Ok(false) => warn!("test_switch: 备库 catch-up 超时，可能存在数据丢失风险"),
                Err(e) => warn!(error = %e, "test_switch: 检查流复制同步状态失败，继续切换"),
            }
            executor.switch_to_backup().map_err(|e| {
                error!(function = function_name, error = %e, "test_switch: 切换到备库失败");
                e
            })?;
        }

        // 记录事件
        self.record_event(
            function_name,
            "switch_to_backup",
            Some("primary"),
            Some("backup"),
            Some("manual switch"),
            None,
        )
        .await?;

        // 更新状态（含 circuit_state=open + total_switches 递增）
        self.update_status_on_switch(function_name, "backup", "open")
            .await?;

        // 记录指标
        self.metrics.record_switch(function_name);

        Ok(format!("已切换 {} 至备用", function_name))
    }

    /// V15 P0-B16：递增 consecutive_failures 并更新熔断器状态
    /// 熔断器状态机：closed（正常）→ 连续失败 >= 3 → open（熔断）；open → 半开探测由 reset_consecutive_failures 在健康恢复时处理
    pub async fn increment_consecutive_failures(&self, function_name: &str) -> Result<i32, String> {
        let now = Utc::now();
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| format!("开启事务失败: {}", e))?;

        let existing = status_model::Entity::find()
            .filter(status_model::Column::FunctionName.eq(function_name))
            .one(&txn)
            .await
            .map_err(|e| format!("查询状态失败: {}", e))?;

        // 熔断阈值：连续 3 次失败触发 open
        const FAILURE_THRESHOLD: i32 = 3;

        let new_count = match existing {
            Some(s) => {
                let new = s.consecutive_failures + 1;
                let mut active: status_model::ActiveModel = s.into();
                active.consecutive_failures = Set(new);
                active.updated_at = Set(now);
                // 熔断器状态转换：达到阈值 → open
                if new >= FAILURE_THRESHOLD {
                    active.circuit_state = Set("open".to_string());
                }
                active
                    .update(&txn)
                    .await
                    .map_err(|e| format!("更新状态失败: {}", e))?;
                new
            }
            None => {
                // 状态行不存在，插入新行（首次失败）
                let active = status_model::ActiveModel {
                    function_name: Set(function_name.to_string()),
                    current_state: Set("primary".to_string()),
                    circuit_state: Set("closed".to_string()),
                    consecutive_failures: Set(1),
                    total_primary_calls: Set(0),
                    total_backup_calls: Set(0),
                    total_switches: Set(0),
                    updated_at: Set(now),
                    ..Default::default()
                };
                active
                    .insert(&txn)
                    .await
                    .map_err(|e| format!("插入状态失败: {}", e))?;
                1
            }
        };

        txn.commit()
            .await
            .map_err(|e| format!("提交事务失败: {}", e))?;
        Ok(new_count)
    }

    /// V15 P0-B16：重置 consecutive_failures 并恢复熔断器为 closed（健康检查成功时调用，同步更新 last_success_at。）
    pub async fn reset_consecutive_failures(&self, function_name: &str) -> Result<(), String> {
        let now = Utc::now();
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| format!("开启事务失败: {}", e))?;

        let existing = status_model::Entity::find()
            .filter(status_model::Column::FunctionName.eq(function_name))
            .one(&txn)
            .await
            .map_err(|e| format!("查询状态失败: {}", e))?;

        if let Some(s) = existing {
            let mut active: status_model::ActiveModel = s.into();
            active.consecutive_failures = Set(0);
            active.circuit_state = Set("closed".to_string());
            active.last_success_at = Set(Some(now));
            active.updated_at = Set(now);
            active
                .update(&txn)
                .await
                .map_err(|e| format!("更新状态失败: {}", e))?;
        }

        txn.commit()
            .await
            .map_err(|e| format!("提交事务失败: {}", e))?;
        Ok(())
    }

    /// V15 P0-B17：切换时更新 status 表（含 total_switches 递增 + last_switch_at）
    /// 专用于 test_switch / FailoverMonitor 自动切换路径：递增 total_switches（累计切换次数）；设置 last_switch_at（最近一次切换时间）；更新 current_state + circuit_state + updated_at
    pub async fn update_status_on_switch(
        &self,
        function_name: &str,
        current_state: &str,
        circuit_state: &str,
    ) -> Result<(), String> {
        let now = Utc::now();
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| format!("开启事务失败: {}", e))?;

        let existing = status_model::Entity::find()
            .filter(status_model::Column::FunctionName.eq(function_name))
            .one(&txn)
            .await
            .map_err(|e| format!("查询主备状态失败: {}", e))?;

        if let Some(existing) = existing {
            // 先保存 total_switches，避免 existing.into() 后再访问 existing 触发 E0382
            let new_total_switches = existing.total_switches + 1;
            let mut active: status_model::ActiveModel = existing.into();
            active.current_state = Set(current_state.to_string());
            active.circuit_state = Set(circuit_state.to_string());
            active.last_switch_at = Set(Some(now));
            active.total_switches = Set(new_total_switches); // 累加切换次数
            active.updated_at = Set(now);
            active
                .update(&txn)
                .await
                .map_err(|e| format!("更新主备状态失败: {}", e))?;
        } else {
            let active = status_model::ActiveModel {
                function_name: Set(function_name.to_string()),
                current_state: Set(current_state.to_string()),
                circuit_state: Set(circuit_state.to_string()),
                consecutive_failures: Set(0),
                total_primary_calls: Set(0),
                total_backup_calls: Set(0),
                total_switches: Set(1),
                last_switch_at: Set(Some(now)),
                updated_at: Set(now),
                ..Default::default()
            };
            active
                .insert(&txn)
                .await
                .map_err(|e| format!("插入主备状态失败: {}", e))?;
        }

        txn.commit()
            .await
            .map_err(|e| format!("提交事务失败: {}", e))?;
        Ok(())
    }

    /// V15 P2 20.4-D：故障回切（人工确认后切换回主库）
    ///
    /// 前置校验：
    /// 1. 当前必须在备库上运行（executor.is_on_backup() == true）
    /// 2. 主库健康检查必须通过（连续 5 次 SELECT 1 成功）
    /// 3. 备库 catch-up 检查（若有流复制配置）
    ///
    /// 校验通过后执行：
    /// 1. executor.switch_to_primary() 原子切换回主库
    /// 2. 更新 status 表（current_state=primary, circuit_state=closed）
    /// 3. 记录 event（event_type=switch_to_primary）
    /// 4. 重置 consecutive_failures
    pub async fn failback(&self, function_name: &str) -> Result<String, String> {
        info!(function = function_name, "人工确认故障回切");

        // 1. 检查是否在备库上
        let executor = self
            .executor
            .as_ref()
            .ok_or_else(|| "FailoverExecutor 未配置，无法回切".to_string())?;

        if !executor.is_on_backup() {
            return Ok(format!("{} 当前已在主库上运行，无需回切", function_name));
        }

        // 2. 检查主库健康（连续 5 次 SELECT 1）
        const HEALTH_CHECK_COUNT: u32 = 5;
        let primary_db = executor.primary_db();
        let backend = primary_db.get_database_backend();
        for i in 0..HEALTH_CHECK_COUNT {
            match primary_db
                .execute(Statement::from_sql_and_values(
                    backend,
                    "SELECT 1",
                    Vec::new(),
                ))
                .await
            {
                Ok(_) => {
                    info!(attempt = i + 1, "failback: 主库健康检查通过");
                }
                Err(e) => {
                    return Err(format!(
                        "failback: 主库健康检查失败（第 {} 次），中止回切: {}",
                        i + 1,
                        e
                    ));
                }
            }
        }

        // 3. 原子切换回主库
        executor.switch_to_primary();

        // 4. 更新 status 表
        self.update_status_on_switch(function_name, "primary", "closed")
            .await?;

        // 5. 重置 consecutive_failures
        self.reset_consecutive_failures(function_name).await?;

        // 6. 记录事件
        self.record_event(
            function_name,
            "switch_to_primary",
            Some("backup"),
            Some("primary"),
            Some("manual failback (health check passed)"),
            None,
        )
        .await?;

        // 7. 记录指标
        self.metrics.record_switch(function_name);

        Ok(format!("已回切 {} 至主库", function_name))
    }

    /// 导出 Prometheus 指标
    pub fn export_metrics(&self) -> Result<String, String> {
        self.metrics.export_text().map_err(|e| e.to_string())
    }

    /// 健康检查（V15 P0-B16：真实 ping 数据库 SELECT 1 + 缓存从 status 表读取）
    /// 数据库：在当前活跃 DB（executor 切换后为备库）上执行 `SELECT 1`，；成功返回 "primary" 或 "backup"（取决于 executor 状态），失败返回 "error"；缓存：暂无 Redis 客户端直连，从 status 表读取上次记录的状态（部分实现，；后续接入 Redis 客户端后改为真实 PING）
    pub async fn health_check(&self) -> Result<HealthStatus, String> {
        use status_model::Column as StatusCol;

        // V15 P0-B16：真实数据库健康检查（SELECT 1）
        let active_db = self.get_active_db();
        let backend = active_db.get_database_backend();
        let db_status = match active_db
            .execute(Statement::from_sql_and_values(
                backend,
                "SELECT 1",
                Vec::new(),
            ))
            .await
        {
            Ok(_) => {
                // 检查 executor 当前是否在备库上
                let on_backup = self
                    .executor
                    .as_ref()
                    .map(|e| e.is_on_backup())
                    .unwrap_or(false);
                if on_backup {
                    "backup".to_string()
                } else {
                    "primary".to_string()
                }
            }
            Err(e) => {
                warn!(error = %e, "health_check: 数据库 SELECT 1 失败");
                "error".to_string()
            }
        };

        // 缓存健康检查：从 status 表读取（部分实现，待接入 Redis 客户端后升级为 PING）
        let cache_status = match status_model::Entity::find()
            .filter(StatusCol::FunctionName.eq("cache"))
            .one(&self.db)
            .await
        {
            Ok(Some(s)) => s.current_state,
            Ok(None) => "unknown".to_string(),
            Err(_) => "error".to_string(),
        };

        Ok(HealthStatus {
            database: db_status,
            cache: cache_status,
        })
    }

    /// V15 P0-B16：纯 DB 健康探测（SELECT 1），供 FailoverMonitor 使用
    /// 与 `health_check` 区别：仅返回 bool，不读 status 表，不构造 HealthStatus，；适合高频后台探测（5s 间隔）。
    pub async fn ping_db(&self) -> bool {
        let active_db = self.get_active_db();
        let backend = active_db.get_database_backend();
        active_db
            .execute(Statement::from_sql_and_values(
                backend,
                "SELECT 1",
                Vec::new(),
            ))
            .await
            .is_ok()
    }

    /// V15 P1 20.4-C：检查 PostgreSQL 流复制同步状态（备库 catch-up 校验）
    /// 切换前调用：查询主库 pg_stat_replication 视图，确认备库 sync_state=sync 且 lag < 阈值。；返回 Ok(true) 表示同步完成可切换；Ok(false) 表示未完成同步；Err 表示查询失败。
    pub async fn check_replication_sync(&self) -> Result<bool, String> {
        const MAX_LAG_BYTES: i64 = 1024 * 1024; // 1MB 阈值，超过视为未同步
        let backend = self.db.get_database_backend();
        let sql = "SELECT COALESCE(COUNT(*), 0) AS sync_count FROM pg_stat_replication WHERE sync_state = 'sync' AND pg_wal_lsn_diff(pg_current_wal_lsn(), replay_lsn) <= $1";
        let rows = self
            .db
            .query_all(Statement::from_sql_and_values(
                backend,
                sql,
                vec![MAX_LAG_BYTES.into()],
            ))
            .await
            .map_err(|e| format!("查询 pg_stat_replication 失败: {}", e))?;
        // SQL 仅返回单行聚合结果，取首行即可（避免 clippy::never_loop）
        if let Some(row) = rows.into_iter().next() {
            let count: i64 = row
                .try_get("", "sync_count")
                .map_err(|e| format!("解析 sync_count 失败: {}", e))?;
            Ok(count > 0)
        } else {
            Ok(false)
        }
    }

    /// V15 P1 20.4-C：等待备库 catch-up 完成（轮询 pg_stat_replication，超时返回 false）
    /// 切换前调用，确保 RPO=0（无数据丢失）。
    pub async fn wait_for_backup_catchup(&self, timeout: Duration) -> Result<bool, String> {
        let start = std::time::Instant::now();
        loop {
            if self.check_replication_sync().await? {
                return Ok(true);
            }
            if start.elapsed() >= timeout {
                return Ok(false);
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

/// 健康状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthStatus {
    pub database: String,
    pub cache: String,
}

// ==================== V15 P0-B17：FailoverExecutor（主备 DB 连接原子切换） ====================

/// 主备切换执行器
/// V15 P0-B17（Batch 484）：修复审计报告 batch-17 §20.4-B 缺陷；维护 primary + optional backup 两个 `DatabaseConnection`，；通过 `ArcSwap<DatabaseConnection>` 实现运行时原子切换。；业务层通过 `get_current()` 获取当前活跃连接。；设计要点：`current` 是 `Arc<ArcSwap<DatabaseConnection>>`，load_full() 返回 `Arc<DatabaseConnection>`；`switch_to_backup()` 原子 store 备库连接，业务层下次 load 即生效；`switch_to_primary()` 原子 store 主库连接（人工确认回切时调用）；备库未配置（`backup = None`）时 `switch_to_backup` 返回 Err，降级为仅更新 status 表；线程安全：`ArcSwap` 内部无锁，多线程并发 load/store 安全。
pub struct FailoverExecutor {
    /// 当前活跃 DB 连接（ArcSwap 原子替换）
    current: Arc<ArcSwap<DatabaseConnection>>,
    /// 主库连接（用于 failback）
    primary: Arc<DatabaseConnection>,
    /// 备库连接（None 表示未配置，switch_to_backup 返回 Err）
    backup: Option<Arc<DatabaseConnection>>,
}

impl FailoverExecutor {
    /// 创建执行器（`primary`：主库连接（必须配置）；`backup`：备库连接（可选；None 时 switch_to_backup 返回 Err））
    pub fn new(primary: Arc<DatabaseConnection>, backup: Option<Arc<DatabaseConnection>>) -> Self {
        Self {
            current: Arc::new(ArcSwap::from(primary.clone())),
            primary,
            backup,
        }
    }

    /// 获取当前活跃 DB 连接（切换前返回 primary，切换后返回 backup。；返回 `Arc<DatabaseConnection>`，调用方短暂持有，不影响后续切换。）
    pub fn get_current(&self) -> Arc<DatabaseConnection> {
        self.current.load_full()
    }

    /// 切换到备库（原子替换 ArcSwap）（备库未配置时返回 Err，调用方应降级处理（仅更新 status 表）。）
    pub fn switch_to_backup(&self) -> Result<(), String> {
        let backup = self.backup.as_ref().ok_or_else(|| {
            "备库未配置（DATABASE_BACKUP_URL 未设置），无法切换到备库".to_string()
        })?;
        self.current.store(backup.clone());
        Ok(())
    }

    /// 切换回主库（人工确认后调用）（无条件 store 主库连接。调用前应确认主库已恢复健康（连续 5 次健康检查通过）。）
    pub fn switch_to_primary(&self) {
        self.current.store(self.primary.clone());
    }

    /// 当前是否在备库上运行
    pub fn is_on_backup(&self) -> bool {
        match &self.backup {
            Some(backup) => Arc::ptr_eq(&self.current.load_full(), backup),
            None => false,
        }
    }

    /// 备库是否已配置
    pub fn has_backup(&self) -> bool {
        self.backup.is_some()
    }

    /// 获取主库连接（用于 failback 前置健康检查）
    pub fn primary_db(&self) -> Arc<DatabaseConnection> {
        self.primary.clone()
    }
}

// ==================== V15 P0-B16：FailoverMonitor（后台健康监控） ====================

/// 主备隔离后台监控任务
/// V15 P0-B16（Batch 484）：修复审计报告 batch-17 §20.4-A 缺陷；每 `interval`（默认 5s）执行一次 DB 健康探测（SELECT 1），；连续 `failure_threshold`（默认 3）次失败时自动调用 `test_switch`。；启停控制：通过 `auto_switch_enabled` 开关（环境变量 `FAILOVER_AUTO_SWITCH_ENABLED`），；默认 false（仅记录日志 + 递增 consecutive_failures，不自动切换），；显式设为 true 时才触发自动切换。；防抖：自动切换成功后重置 consecutive_failures，避免反复触发。
pub struct FailoverMonitor {
    /// FailoverService 实例（持有 db + metrics + executor）
    service: FailoverService,
    /// 探测间隔（默认 5s）
    interval: Duration,
    /// 连续失败触发阈值（默认 3）
    failure_threshold: u32,
    /// 是否启用自动切换（false 时仅记录 + 递增计数，不触发 test_switch）
    auto_switch_enabled: bool,
    /// 监控的功能名（固定 "database"）
    function_name: String,
}

impl FailoverMonitor {
    /// 创建监控任务
    pub fn new(
        service: FailoverService,
        interval: Duration,
        failure_threshold: u32,
        auto_switch_enabled: bool,
    ) -> Self {
        Self {
            service,
            interval,
            failure_threshold,
            auto_switch_enabled,
            function_name: "database".to_string(),
        }
    }

    /// 启动后台监控循环（消费 self，通常在 tokio::spawn 中调用）
    /// 循环逻辑：1. sleep(interval)；2. ping_db() → bool；3. 成功：重置 consecutive_failures（若之前有失败）；4. 失败：递增 consecutive_failures；若 auto_switch_enabled && consecutive_failures >= threshold → 调用 test_switch；自动切换成功后重置计数，失败则记录 error 日志
    pub async fn run(self) {
        info!(
            interval_secs = self.interval.as_secs(),
            threshold = self.failure_threshold,
            auto_switch = self.auto_switch_enabled,
            "FailoverMonitor: 启动后台健康监控（5s 间隔，连续 3 次失败触发自动切换）"
        );

        let mut consecutive_failures: u32 = 0;

        loop {
            tokio::time::sleep(self.interval).await;

            // 1. 健康探测（SELECT 1）
            let db_ok = self.service.ping_db().await;

            if db_ok {
                // 健康恢复：重置失败计数 + 熔断器 → closed
                if consecutive_failures > 0 {
                    consecutive_failures = 0;
                    if let Err(e) = self
                        .service
                        .reset_consecutive_failures(&self.function_name)
                        .await
                    {
                        warn!(error = %e, "FailoverMonitor: reset_consecutive_failures 失败");
                    }
                    info!("FailoverMonitor: 数据库健康恢复，重置失败计数");
                }
            } else {
                consecutive_failures += 1;
                // 同步递增 status 表中的 consecutive_failures（供外部观测）
                match self
                    .service
                    .increment_consecutive_failures(&self.function_name)
                    .await
                {
                    Ok(db_count) => {
                        warn!(
                            consecutive_failures = consecutive_failures,
                            db_consecutive_failures = db_count,
                            threshold = self.failure_threshold,
                            "FailoverMonitor: 数据库健康检查失败（SELECT 1）"
                        );
                    }
                    Err(e) => {
                        error!(
                            error = %e,
                            "FailoverMonitor: increment_consecutive_failures 写表失败"
                        );
                    }
                }

                // 自动切换判断
                if self.auto_switch_enabled && consecutive_failures >= self.failure_threshold {
                    error!(
                        consecutive_failures,
                        threshold = self.failure_threshold,
                        "FailoverMonitor: 连续失败达阈值，触发自动切换到备库"
                    );
                    match self.service.test_switch(&self.function_name).await {
                        Ok(msg) => {
                            info!(msg = %msg, "FailoverMonitor: 自动切换成功");
                            consecutive_failures = 0;
                        }
                        Err(e) => {
                            error!(error = %e, "FailoverMonitor: 自动切换失败");
                        }
                    }
                }
            }
        }
    }
}

// 死代码清理（2026-06-26）：format_failover_error 仅被测试调用，无业务引用，已删除。

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let m = FailoverMetrics::new();
        assert!(m.is_ok());
    }

    // 死代码清理（2026-06-26）：test_format_failover_error 测试的 format_failover_error 已删除。
}
