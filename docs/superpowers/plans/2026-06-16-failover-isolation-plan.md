# 主备隔离模块实施计划（P0 阶段）

> **编写时间**: 2026-06-16
> **实施范围**: P0 阶段（数据库 + 缓存）
> **Spec**: [`/workspace/docs/superpowers/specs/2026-06-16-failover-isolation-design.md`](../specs/2026-06-16-failover-isolation-design.md)
> **目标**: 3 周内完成 14 Task，PR 合到 test 分支，交付 TEST 测试版本
> **实施模式**: subagent-driven（避免信息孤岛）

---

## 1. 实施约束

### 1.1 硬性约束（不可违反）

1. **代码规范**
   - 文件名 ≤ 9 字符（failover-related 例外）
   - 禁止硬编码（URL/密钥/密码）
   - 代码注释使用中文
   - 文本使用中文
   - 严禁 `auth.tenant_id.unwrap_or(0)`，必须 `extract_tenant_id(&auth)?`

2. **死代码处理**
   - 禁止文件级 `#![allow(dead_code)]` 全局抑制
   - 禁止 crate 级 `#![allow(unused_imports)]` 等
   - 启用 `#[allow(dead_code)]` 项级抑制必须加 `TODO(tech-debt)` 注释
   - SeaORM 自动生成模型可保留文件级抑制

3. **Git 规范**
   - 分支命名：`trae/solo-agent-P0-2-{your-id}`
   - Commit 规范：`type(scope): 中文描述`
   - **不要**合到 main（main 保持现状）
   - **不要**修改 P0-1 销售报价单代码
   - 推送后创建 PR 合到 test 分支

4. **构建验证**
   - 沙箱 5.8GB 内存：禁止 `cargo test`，仅用 `cargo check --lib`
   - 完整验证依赖 CI/CD 流水线
   - 等待 CI 4 job 全绿

5. **数据库**
   - PostgreSQL 类型
   - 通过 SeaORM migration 创建表
   - 必须可回滚（down.sql 完整）

### 1.2 沙箱限制

| 资源 | 限制 | 应对 |
|------|------|------|
| 内存 | 5.8 GB | 不用 `cargo test`，仅 `cargo check --lib` |
| rustc | 默认 1.81 | 用 rustc 1.94（已安装 `/usr/local/rust-1.94/`）|
| 网络 | 限制 | 离线构建，避免下载大依赖 |
| 时间 | 单次 ≤ 1 小时 | 拆分为小任务并行 |

### 1.3 命名规范

| 类型 | 限制 |
|------|------|
| 函数/方法 | ≤ 9 字符（failover_xxx 例外） |
| 变量 | ≤ 9 字符（无意义缩写） |
| 文件名 | ≤ 9 字符（failover-related 例外） |
| 常量 | SCREAMING_SNAKE ≤ 30 字符 |

---

## 2. 任务总览（14 Task / 3 周）

### Week 1：后端基础（6 Task）

| Task | 描述 | 估时 | 子代理 |
|------|------|------|--------|
| T1.1 | 数据库 migration：3 张表（failover_status / failover_event / failover_config） | 2h | 串行 |
| T1.2 | FailoverCall trait + 熔断器实现 | 3h | 并行 |
| T1.3 | 数据库主备实现（FailoverDatabase） | 3h | 串行 |
| T1.4 | 缓存主备实现（FailoverCache + moka LRU） | 3h | 并行 |
| T1.5 | 配置加载（config/failover.rs） | 1h | 串行 |
| T1.6 | Prometheus 指标（failover_metrics.rs） | 2h | 串行 |

### Week 2：业务接入（4 Task）

| Task | 描述 | 估时 | 子代理 |
|------|------|------|--------|
| T2.1 | 集成测试 5 个场景（failover_*_test.rs） | 4h | 并行 |
| T2.2 | 故障注入测试（chaos test） | 3h | 串行 |
| T2.3 | HTTP API 端点（3 个 + handler） | 2h | 串行 |
| T2.4 | admin 监控页面（failover.vue + 3 组件） | 4h | 并行 |

### Week 3：交付（4 Task）

| Task | 描述 | 估时 | 子代理 |
|------|------|------|--------|
| T3.1 | 部署文档（failover-deployment-guide.md） | 1h | 串行 |
| T3.2 | Grafana dashboard + 告警规则 | 2h | 并行 |
| T3.3 | TEST 测试版本（Docker + compose + start.sh） | 3h | 串行 |
| T3.4 | CHANGELOG + MEMORY 更新 + PR 合并 | 2h | 串行 |

**总工作量**：约 35h ≈ 5 个工作日（含 14 Task）

---

## 3. 详细任务清单

### T1.1 数据库 migration

**目标**：创建 3 张表

**步骤**：
1. 创建 migration 目录 `backend/migrations/20260616000005_create_failover_tables/`
2. 编写 `up.sql`：
   - `failover_status`（主备实时状态）
   - `failover_event`（切换事件流水）
   - `failover_config`（配置持久化）
3. 编写 `down.sql`（3 个 DROP TABLE）
4. 创建 SeaORM 模型 `backend/src/models/failover_status.rs` / `failover_event.rs` / `failover_config.rs`
5. 在 `backend/src/models/mod.rs` 添加 `pub mod` 声明

**SQL 模板**：

```sql
-- 20260616000005_create_failover_tables/up.sql

-- 主备实时状态表
CREATE TABLE failover_status (
    id BIGSERIAL PRIMARY KEY,
    function_name VARCHAR(100) NOT NULL UNIQUE,
    current_state VARCHAR(20) NOT NULL DEFAULT 'primary',
    circuit_state VARCHAR(20) NOT NULL DEFAULT 'closed',
    primary_url VARCHAR(500),
    backup_type VARCHAR(50),
    last_switch_at TIMESTAMPTZ,
    last_success_at TIMESTAMPTZ,
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    total_primary_calls BIGINT NOT NULL DEFAULT 0,
    total_backup_calls BIGINT NOT NULL DEFAULT 0,
    total_switches BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_failover_status_func ON failover_status(function_name);

-- 切换事件流水表
CREATE TABLE failover_event (
    id BIGSERIAL PRIMARY KEY,
    function_name VARCHAR(100) NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    from_state VARCHAR(20),
    to_state VARCHAR(20),
    reason TEXT,
    latency_ms INTEGER,
    tenant_id BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_failover_event_func_time ON failover_event(function_name, created_at DESC);
CREATE INDEX idx_failover_event_type ON failover_event(event_type);

-- 配置持久化表
CREATE TABLE failover_config (
    id BIGSERIAL PRIMARY KEY,
    function_name VARCHAR(100) NOT NULL,
    config_key VARCHAR(200) NOT NULL,
    config_value TEXT NOT NULL,
    description TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(function_name, config_key)
);
```

**测试**：本地 `cargo check --lib` 通过
**Commit**：`feat(failover): 数据库 migration 3 张表（failover_status/event/config）`

---

### T1.2 FailoverCall trait + 熔断器

**目标**：定义统一接口和熔断器

**步骤**：
1. 创建 `backend/src/utils/failover/mod.rs`（FailoverCall trait + FailoverError）
2. 创建 `backend/src/utils/failover/circuit_breaker.rs`
3. 在 `backend/src/utils/mod.rs` 添加 `pub mod failover;`
4. 编写单元测试

**代码模板**：

```rust
// backend/src/utils/failover/circuit_breaker.rs
use std::sync::atomic::{AtomicU8, AtomicU32, AtomicI64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed = 0,
    Open = 1,
    HalfOpen = 2,
}

pub struct CircuitBreaker {
    threshold: u32,
    duration: Duration,
    state: AtomicU8,
    consecutive_failures: AtomicU32,
    opened_at_ms: AtomicI64,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, duration: Duration) -> Self {
        Self {
            threshold,
            duration,
            state: AtomicU8::new(CircuitState::Closed as u8),
            consecutive_failures: AtomicU32::new(0),
            opened_at_ms: AtomicI64::new(0),
        }
    }

    pub fn state(&self) -> CircuitState {
        match self.state.load(Ordering::Acquire) {
            0 => CircuitState::Closed,
            1 => CircuitState::Open,
            2 => CircuitState::HalfOpen,
            _ => CircuitState::Closed,
        }
    }

    pub fn is_open(&self) -> bool {
        // 如果熔断打开，检查是否已过熔断时长
        if self.state() == CircuitState::Open {
            let opened_at = self.opened_at_ms.load(Ordering::Acquire);
            let now = Self::now_ms();
            if now - opened_at >= self.duration.as_millis() as i64 {
                // 进入半开状态
                self.state.store(CircuitState::HalfOpen as u8, Ordering::Release);
                return false;
            }
            return true;
        }
        false
    }

    pub fn record_success(&self) {
        self.consecutive_failures.store(0, Ordering::Release);
        self.state.store(CircuitState::Closed as u8, Ordering::Release);
    }

    pub fn record_failure(&self) {
        let failures = self.consecutive_failures.fetch_add(1, Ordering::AcqRel) + 1;
        if failures >= self.threshold {
            self.state.store(CircuitState::Open as u8, Ordering::Release);
            self.opened_at_ms.store(Self::now_ms(), Ordering::Release);
        }
    }

    fn now_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }
}
```

**Commit**：`feat(failover): FailoverCall trait + 熔断器实现`

---

### T1.3 数据库主备实现

**目标**：`FailoverDatabase` 实现 `FailoverCall`

**步骤**：
1. 创建 `backend/src/utils/failover/database.rs`
2. 在 `mod.rs` 中添加 `pub mod database;`
3. 实现 `primary_call` / `backup_call` / ping
4. 添加连接池健康检查

**代码模板**：

```rust
// backend/src/utils/failover/database.rs
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, DbErr, ConnectionTrait, Statement, DbBackend};
use crate::config::failover::DatabaseFailoverConfig;
use crate::utils::failover::{FailoverCall, FailoverError};
use crate::utils::failover::circuit_breaker::CircuitBreaker;

pub struct FailoverDatabase {
    primary: DatabaseConnection,
    backup: DatabaseConnection,
    circuit: Arc<CircuitBreaker>,
    config: DatabaseFailoverConfig,
    function_name: String,
}

impl FailoverDatabase {
    pub fn new(
        primary: DatabaseConnection,
        backup: DatabaseConnection,
        config: DatabaseFailoverConfig,
    ) -> Self {
        let circuit = Arc::new(CircuitBreaker::new(
            config.circuit_breaker_threshold,
            Duration::from_secs(config.circuit_breaker_duration_s),
        ));
        Self {
            primary,
            backup,
            circuit,
            config,
            function_name: "database".to_string(),
        }
    }
}

#[async_trait]
impl FailoverCall<bool, DbErr> for FailoverDatabase {
    async fn primary_call(&self) -> Result<bool, DbErr> {
        ping(&self.primary).await
    }
    async fn backup_call(&self) -> Result<bool, DbErr> {
        ping(&self.backup).await
    }
    fn primary_timeout(&self) -> Duration {
        Duration::from_millis(self.config.primary_timeout_ms)
    }
    fn backup_timeout(&self) -> Duration {
        Duration::from_millis(self.config.backup_timeout_ms)
    }
    fn function_name(&self) -> &str {
        &self.function_name
    }
}

async fn ping(db: &DatabaseConnection) -> Result<bool, DbErr> {
    db.execute(Statement::from_string(DbBackend::Postgres, "SELECT 1".to_string())).await?;
    Ok(true)
}
```

**Commit**：`feat(failover): 数据库主备实现 FailoverDatabase`

---

### T1.4 缓存主备实现

**目标**：`FailoverCache` + moka LRU

**步骤**：
1. 在 `Cargo.toml` 添加 `moka = { version = "0.12", features = ["future"] }` 和 `redis = { version = "0.27", features = ["tokio-comp"] }`
2. 创建 `backend/src/utils/failover/cache.rs`
3. 实现 Redis 主 + moka LRU 备

**代码模板**：

```rust
// backend/src/utils/failover/cache.rs
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use moka::future::Cache;
use redis::AsyncCommands;
use crate::config::failover::CacheFailoverConfig;
use crate::utils::failover::{FailoverCall, FailoverError};
use crate::utils::failover::circuit_breaker::CircuitBreaker;

pub struct FailoverCache {
    primary: redis::Client,
    backup: Cache<String, Vec<u8>>,
    circuit: Arc<CircuitBreaker>,
    config: CacheFailoverConfig,
    function_name: String,
}

impl FailoverCache {
    pub fn new(
        primary_url: &str,
        config: CacheFailoverConfig,
    ) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(primary_url)?;
        let cache = Cache::builder()
            .max_capacity(config.backup_max_entries as u64)
            .build();
        let circuit = Arc::new(CircuitBreaker::new(
            5, // 默认熔断阈值
            Duration::from_secs(30),
        ));
        Ok(Self {
            primary: client,
            backup: cache,
            circuit,
            config,
            function_name: "cache".to_string(),
        })
    }
}

#[async_trait]
impl FailoverCall<Option<Vec<u8>>, redis::RedisError> for FailoverCache {
    async fn primary_call(&self) -> Result<Option<Vec<u8>>, redis::RedisError> {
        let mut conn = self.primary.get_async_connection().await?;
        let pong: Option<String> = conn.ping().await?;
        Ok(pong.map(|s| s.into_bytes()))
    }
    async fn backup_call(&self) -> Result<Option<Vec<u8>>, redis::RedisError> {
        // 进程内 LRU 永远可用
        Ok(Some(b"PONG".to_vec()))
    }
    fn primary_timeout(&self) -> Duration {
        Duration::from_millis(self.config.primary_timeout_ms)
    }
    fn backup_timeout(&self) -> Duration {
        Duration::from_millis(self.config.backup_timeout_ms)
    }
    fn function_name(&self) -> &str {
        &self.function_name
    }
}
```

**Commit**：`feat(failover): 缓存主备实现 FailoverCache + moka LRU`

---

### T1.5 配置加载

**目标**：`config/failover.rs`

**步骤**：
1. 在 `backend/src/config/` 目录下创建 `failover.rs`
2. 在 `backend/src/config/mod.rs` 中添加 `pub mod failover;`
3. 实现从 toml 文件加载
4. 支持环境变量覆盖

**代码模板**：

```rust
// backend/src/config/failover.rs
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct FailoverConfig {
    pub database: DatabaseFailoverConfig,
    pub cache: CacheFailoverConfig,
    #[serde(default)]
    pub monitoring: MonitoringFailoverConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseFailoverConfig {
    pub primary_url: String,
    pub backup_url: String,
    pub primary_timeout_ms: u64,
    pub backup_timeout_ms: u64,
    pub circuit_breaker_threshold: u32,
    pub circuit_breaker_duration_s: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CacheFailoverConfig {
    pub primary_url: String,
    pub backup_max_entries: usize,
    pub primary_timeout_ms: u64,
    pub backup_timeout_ms: u64,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct MonitoringFailoverConfig {
    #[serde(default = "default_true")]
    pub metrics_enabled: bool,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_true() -> bool { true }
fn default_log_level() -> String { "info".to_string() }

impl FailoverConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        toml::from_str(&content).map_err(|e| e.to_string())
    }

    pub fn load_from_env() -> Result<Self, String> {
        // 从环境变量加载
        Ok(Self {
            database: DatabaseFailoverConfig {
                primary_url: std::env::var("DATABASE_URL_PRIMARY").map_err(|_| "DATABASE_URL_PRIMARY not set".to_string())?,
                backup_url: std::env::var("DATABASE_URL_BACKUP").map_err(|_| "DATABASE_URL_BACKUP not set".to_string())?,
                primary_timeout_ms: 3000,
                backup_timeout_ms: 5000,
                circuit_breaker_threshold: 5,
                circuit_breaker_duration_s: 30,
            },
            cache: CacheFailoverConfig {
                primary_url: std::env::var("REDIS_URL").map_err(|_| "REDIS_URL not set".to_string())?,
                backup_max_entries: 10_000,
                primary_timeout_ms: 1000,
                backup_timeout_ms: 0,
            },
            monitoring: MonitoringFailoverConfig::default(),
        })
    }
}
```

**Commit**：`feat(failover): 配置加载 failover.rs`

---

### T1.6 Prometheus 指标

**目标**：`monitoring/failover_metrics.rs`

**步骤**：
1. 创建 `backend/src/monitoring/failover_metrics.rs`
2. 在 `backend/src/monitoring/mod.rs` 中添加 `pub mod failover_metrics;`
3. 注册 5 个指标
4. 提供 `record_*` 方法

**代码模板**：

```rust
// backend/src/monitoring/failover_metrics.rs
use prometheus::{IntCounterVec, IntGaugeVec, Opts, Registry};

pub struct FailoverMetrics {
    pub primary_total: IntCounterVec,
    pub primary_failed_total: IntCounterVec,
    pub backup_total: IntCounterVec,
    pub switch_total: IntCounterVec,
    pub circuit_state: IntGaugeVec,
    pub registry: Registry,
}

impl FailoverMetrics {
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
            Opts::new("failover_circuit_state", "熔断器状态（0=关闭,1=打开,2=半开）"),
            &["function"],
        )?;
        registry.register(Box::new(primary_total.clone()))?;
        registry.register(Box::new(primary_failed_total.clone()))?;
        registry.register(Box::new(backup_total.clone()))?;
        registry.register(Box::new(switch_total.clone()))?;
        registry.register(Box::new(circuit_state.clone()))?;
        Ok(Self {
            primary_total,
            primary_failed_total,
            backup_total,
            switch_total,
            circuit_state,
            registry,
        })
    }

    pub fn record_primary(&self, function: &str) {
        self.primary_total.with_label_values(&[function]).inc();
    }

    pub fn record_primary_failed(&self, function: &str) {
        self.primary_failed_total.with_label_values(&[function]).inc();
    }

    pub fn record_backup(&self, function: &str) {
        self.backup_total.with_label_values(&[function]).inc();
    }

    pub fn record_switch(&self, function: &str) {
        self.switch_total.with_label_values(&[function]).inc();
    }

    pub fn set_circuit_state(&self, function: &str, state: i64) {
        self.circuit_state.with_label_values(&[function]).set(state);
    }
}
```

**Commit**：`feat(failover): Prometheus 监控指标`

---

### T2.1 集成测试

**目标**：5 个集成测试文件

**步骤**：
1. 创建 `backend/tests/failover_database_test.rs`
2. 创建 `backend/tests/failover_cache_test.rs`
3. 创建 `backend/tests/failover_circuit_test.rs`
4. 创建 `backend/tests/failover_recovery_test.rs`
5. 创建 `backend/tests/failover_api_test.rs`

**注意**：沙箱 OOM 跑不了 `cargo test`，仅本地写代码 + 依赖 CI 验证

**Commit**：`test(failover): 5 个集成测试文件`

---

### T2.2 故障注入测试（chaos test）

**目标**：`docs/chaos-test-scenarios.md`

**步骤**：
1. 编写 5 个故障注入场景
2. 包含注入命令 + 预期行为 + 验证步骤

**模板**：

```markdown
# 故障注入测试用例

## 场景 1：主库连接拒绝

**注入命令**：
```bash
iptables -A OUTPUT -p tcp --dport 5432 -j DROP
```

**预期行为**：
- 主调用失败 → 触发熔断
- 连续 5 次失败 → 熔断器打开
- 自动切换至备库
- `failover_switch_total` 指标 +1

**验证步骤**：
1. 启动应用，确认主库连接正常
2. 执行 iptables 规则
3. 等待 5 次失败（约 15s）
4. 检查 `GET /api/v1/erp/admin/failover/status` 返回 `current_state: "backup"`
5. 清理：`iptables -D OUTPUT -p tcp --dport 5432 -j DROP`
6. 等待 30s，检查自动回切
```

**Commit**：`docs(failover): 故障注入测试场景`

---

### T2.3 HTTP API 端点

**目标**：3 个端点 + handler

**步骤**：
1. 创建 `backend/src/handlers/failover_handler.rs`
2. 创建 `backend/src/routes/failover.rs`
3. 在 `backend/src/handlers/mod.rs` 添加 `pub mod failover_handler;`
4. 在 `backend/src/lib.rs` 注册路由

**代码模板**：

```rust
// backend/src/handlers/failover_handler.rs
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use crate::AppState;
use crate::services::failover_service::FailoverService;

pub async fn get_failover_status(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = FailoverService::new(&state);
    let statuses = service.get_statuses().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "statuses": statuses })))
}

pub async fn get_failover_metrics(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = FailoverService::new(&state);
    let metrics = service.get_metrics().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok((StatusCode::OK, [("content-type", "text/plain; version=0.0.4")], metrics))
}

pub async fn post_test_switch(
    State(state): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = FailoverService::new(&state);
    let function_name = req.get("function")
        .and_then(|v| v.as_str())
        .ok_or((StatusCode::BAD_REQUEST, "function required".to_string()))?;
    service.test_switch(function_name).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "status": "switched" })))
}
```

**Commit**：`feat(failover): HTTP API 端点（status / metrics / test/switch）`

---

### T2.4 admin 监控页面

**目标**：Vue 页面 + 3 个组件

**步骤**：
1. 创建 `frontend/src/views/admin/failover.vue`
2. 创建 `frontend/src/views/admin/components/FailoverStatusCard.vue`
3. 创建 `frontend/src/views/admin/components/FailoverEventList.vue`
4. 创建 `frontend/src/views/admin/components/FailoverMetrics.vue`
5. 添加 API `frontend/src/api/failover.ts`
6. 添加路由

**Commit**：`feat(failover): admin 监控页面`

---

### T3.1 部署文档

**目标**：`docs/failover-deployment-guide.md`

**步骤**：
1. 创建文档
2. 包含：环境要求 / 配置文件 / 启动步骤 / 故障注入测试 / 监控接入

**Commit**：`docs(failover): 部署指南`

---

### T3.2 Grafana dashboard + 告警规则

**目标**：`monitoring/grafana/failover-dashboard.json` + 更新 `alert_rules.yml`

**步骤**：
1. 创建 Grafana dashboard JSON
2. 更新 `monitoring/prometheus/alert_rules.yml` 添加 4 条规则

**Commit**：`feat(failover): Grafana dashboard + 告警规则`

---

### T3.3 TEST 测试版本

**目标**：`dist/test-version-P0-2/`

**步骤**：
1. 创建 `Dockerfile`（多阶段构建）
2. 创建 `docker-compose.yml`（含 PostgreSQL 主+备 + Redis 主 + 应用）
3. 创建 `config/failover.toml.example`
4. 创建 `start.sh`（一键启动）
5. 创建 `README.md`（部署说明）
6. 创建 `chaos-test-scenarios.md`（故障注入）
7. 创建 `monitoring-dashboard.json`（Grafana）

**Commit**：`docs(dist): P0-2 TEST 测试版本交付`

---

### T3.4 CHANGELOG + MEMORY + PR 合并

**目标**：完成所有文档更新 + 创建 PR

**步骤**：
1. 更新 `/workspace/CHANGELOG.md` 添加 P0-2 条目
2. 更新 `/workspace/MEMORY.md` 记录 P0-2 完成
3. 推送分支到 origin
4. 创建 PR：`feat(failover): 主备隔离模块（数据库 + 缓存 + 监控 + 测试版本）`
5. 合到 test 分支（**不要**合到 main）

**Commit**：`docs: CHANGELOG + MEMORY 更新 + PR 合并`

---

## 4. 关键代码文件清单

### 4.1 新增文件

| 路径 | 说明 |
|------|------|
| `backend/src/utils/failover/mod.rs` | FailoverCall trait |
| `backend/src/utils/failover/circuit_breaker.rs` | 熔断器 |
| `backend/src/utils/failover/database.rs` | 数据库主备 |
| `backend/src/utils/failover/cache.rs` | 缓存主备 |
| `backend/src/config/failover.rs` | 配置加载 |
| `backend/src/monitoring/failover_metrics.rs` | Prometheus 指标 |
| `backend/src/handlers/failover_handler.rs` | HTTP 端点 |
| `backend/src/services/failover_service.rs` | 业务逻辑 |
| `backend/src/models/failover_status.rs` | 模型 |
| `backend/src/models/failover_event.rs` | 模型 |
| `backend/src/models/failover_config.rs` | 模型 |
| `backend/src/routes/failover.rs` | 路由 |
| `backend/migrations/20260616000005_create_failover_tables/up.sql` | migration |
| `backend/migrations/20260616000005_create_failover_tables/down.sql` | migration |
| `backend/tests/failover_database_test.rs` | 集成测试 |
| `backend/tests/failover_cache_test.rs` | 集成测试 |
| `backend/tests/failover_circuit_test.rs` | 集成测试 |
| `backend/tests/failover_recovery_test.rs` | 集成测试 |
| `backend/tests/failover_api_test.rs` | 集成测试 |
| `frontend/src/views/admin/failover.vue` | 监控页面 |
| `frontend/src/views/admin/components/FailoverStatusCard.vue` | 状态卡片 |
| `frontend/src/views/admin/components/FailoverEventList.vue` | 事件列表 |
| `frontend/src/views/admin/components/FailoverMetrics.vue` | 指标展示 |
| `frontend/src/api/failover.ts` | API |
| `monitoring/grafana/failover-dashboard.json` | Grafana |
| `dist/test-version-P0-2/Dockerfile` | Docker |
| `dist/test-version-P0-2/docker-compose.yml` | compose |
| `dist/test-version-P0-2/config/failover.toml.example` | 配置 |
| `dist/test-version-P0-2/start.sh` | 启动脚本 |
| `dist/test-version-P0-2/README.md` | 部署说明 |
| `dist/test-version-P0-2/chaos-test-scenarios.md` | 故障注入 |
| `docs/failover-deployment-guide.md` | 部署文档 |

### 4.2 修改文件

| 路径 | 修改 |
|------|------|
| `backend/Cargo.toml` | 添加 moka + redis 依赖 |
| `backend/src/utils/mod.rs` | 添加 `pub mod failover;` |
| `backend/src/config/mod.rs` | 添加 `pub mod failover;` |
| `backend/src/monitoring/mod.rs` | 添加 `pub mod failover_metrics;` |
| `backend/src/handlers/mod.rs` | 添加 `pub mod failover_handler;` |
| `backend/src/models/mod.rs` | 添加 3 个 `pub mod` |
| `backend/src/lib.rs` | 注册路由 |
| `monitoring/prometheus/alert_rules.yml` | 添加 4 条告警 |
| `frontend/src/router/index.ts` | 添加路由 |
| `CHANGELOG.md` | 添加 P0-2 条目 |
| `MEMORY.md` | 更新记忆 |

---

## 5. Commit 规范

每完成一个 Task 提交一次，commit message 格式：

```
type(scope): 中文描述

- 详细说明 1
- 详细说明 2
```

**type 类型**：
- `feat`：新功能
- `fix`：修复
- `docs`：文档
- `test`：测试
- `refactor`：重构
- `chore`：杂项

**示例**：

```
feat(failover): 数据库 migration 3 张表

- 新增 failover_status 表（主备实时状态）
- 新增 failover_event 表（切换事件流水）
- 新增 failover_config 表（配置持久化）
- 创建 3 个 down.sql 回滚
```

---

## 6. PR 流程

### 6.1 创建分支

```bash
git checkout test
git pull origin test
git checkout -b trae/solo-agent-P0-2-{your-id}
```

### 6.2 推送

```bash
git push -u origin trae/solo-agent-P0-2-{your-id}
```

### 6.3 创建 PR

```bash
curl -X POST https://api.github.com/repos/57231307/1/pulls \
  -H "Authorization: token ${GITHUB_TOKEN}" \
  -d '{
    "title": "feat(failover): 主备隔离模块（数据库 + 缓存 + 监控 + 测试版本）",
    "head": "trae/solo-agent-P0-2-{your-id}",
    "base": "test",
    "body": "## 主要变更\n- FailoverCall trait + 熔断器\n- 数据库主备隔离 + Redis + moka LRU\n- 3 张表 migration + 3 个 HTTP 端点\n- admin 监控页面 + Grafana dashboard\n- 5 个集成测试 + 5 个故障注入场景\n- TEST 测试版本（Docker + compose）"
  }'
```

### 6.4 合并 PR

```bash
curl -X PUT https://api.github.com/repos/57231307/1/pulls/{N}/merge \
  -H "Authorization: token ${GITHUB_TOKEN}" \
  -d '{"merge_method": "merge"}'
```

### 6.5 清理

```bash
git push origin --delete trae/solo-agent-P0-2-{your-id}
```

---

## 7. 失败处理

| 失败场景 | 处理方式 |
|---------|---------|
| `cargo check` 错误 | 立即修复，记录到"已知问题"段落 |
| `cargo test` OOM | 跳过测试，依赖 CI 验证 |
| PR merge 冲突 | 用 P0-1 PR #126 模式：merge origin/test → 解冲突 → push → API merge |
| 沙箱 5.8GB 内存不够 | 用 rustc 1.94 单独编译，**不要**用 cargo test |
| rustc 1.94 下载失败 | 用 1.90 + 单文件编译验证关键路径 |
| moka 编译失败 | 降低 moka 版本到 0.11 |
| redis 编译失败 | 检查 feature flags |

### 7.1 已知问题（待补充）

（实施过程中遇到的问题记录在此）

---

## 8. 时间线

| 周 | 任务 | 产出 |
|----|------|------|
| Week 1（Day 1-5）| T1.1 - T1.6 | 后端基础 6 commit |
| Week 2（Day 6-10）| T2.1 - T2.4 | 业务接入 4 commit |
| Week 3（Day 11-15）| T3.1 - T3.4 | 文档 + 交付 4 commit |
| **总计** | **14 Task / 14 commit** | **PR 合到 test** |

---

## 9. 验收检查表

- [ ] 14 Task 全部完成
- [ ] 14 commit 全部推送到 origin
- [ ] PR 合到 test 分支
- [ ] main 分支未变
- [ ] CI/CD 4 job 全绿
- [ ] TEST 测试版本可启动（`docker-compose up -d`）
- [ ] 5 个 Prometheus 指标可抓取
- [ ] 4 条告警规则已添加
- [ ] Grafana dashboard 可导入
- [ ] CHANGELOG 已更新
- [ ] MEMORY 已更新
- [ ] 部署指南完整

---

**版本**: v1.0
**最后更新**: 2026-06-16
**状态**: ✅ 已批准，进入实施阶段
