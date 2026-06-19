# 主备隔离模块设计（P0 阶段）

> **设计时间**: 2026-06-16
> **设计状态**: ✅ 已批准
> **设计版本**: v1.0
> **项目**: 冰溪 ERP（Bingxi ERP）
> **设计报告**: [`/workspace/docs/superpowers/reports/2026-06-16-failover-design.md`](../reports/2026-06-16-failover-design.md)
> **范围**: P0 阶段（数据库 + 缓存）

---

## 0. 目标

实现 8 大核心功能主备隔离调用的 **P0 阶段**：数据库主备隔离 + 缓存主备隔离 + 进程内 LRU 备用 + 监控告警 + 故障注入测试。

### 0.1 业务价值

- **业务连续性**：主库/主缓存故障时自动切换至备用，业务不中断
- **可观测性**：5 个 Prometheus 指标 + 告警规则，主备状态实时可见
- **可测试性**：故障注入测试用例（chaos test）+ TEST 测试版本交付
- **可扩展性**：统一 `FailoverCall` trait 接口，P1/P2 阶段（MQ/存储/短信/邮件/搜索/OCR）复用同一框架

### 0.2 关联缺口

填补 `2026-06-16-failover-design.md` 评估中"数据库单点 + 缓存未生产化"的 P0 阶段缺口。

### 0.3 范围边界

| 范围 | 本次（P0）| 后续（P1/P2）|
|------|----------|------------|
| 数据库 | ✅ 主备隔离 + 进程内 LRU 备份（连接级） | - |
| 缓存 | ✅ Redis 主 + moka 进程内 LRU 备 | - |
| MQ | - | P1 阶段 |
| 存储 | - | P1 阶段 |
| 短信 | - | P1 阶段 |
| 邮件 | - | P1 阶段（部分已有）|
| 搜索 | - | P2 阶段 |
| OCR | - | P2 阶段 |

---

## 1. 整体架构

```
┌──────────────────────────────────────────────────────────────────────┐
│  前端 (Vue 3 + Element Plus)                                          │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │ /admin/failover  - 主备监控面板（状态/指标/手动切换）          │   │
│  └──────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────┘
                              ↓ REST API
┌──────────────────────────────────────────────────────────────────────┐
│  后端 (Rust + Axum + SeaORM)                                          │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │ handlers/failover_handler.rs        (HTTP 端点)                │   │
│  │   - GET  /api/v1/erp/admin/failover/status                    │   │
│  │   - GET  /api/v1/erp/admin/failover/metrics                   │   │
│  │   - POST /api/v1/erp/admin/failover/test/switch                │   │
│  │ services/failover_service.rs         (业务逻辑)                │   │
│  │ utils/failover/mod.rs                (FailoverCall trait)     │   │
│  │ utils/failover/database.rs           (数据库主备实现)          │   │
│  │ utils/failover/cache.rs              (缓存主备实现)            │   │
│  │ monitoring/failover_metrics.rs       (Prometheus 指标)        │   │
│  │ config/failover.rs                   (配置加载)               │   │
│  └──────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────┘
                              ↓ FailoverCall
┌──────────────────────────────────────────────────────────────────────┐
│  主调用层                          备用调用层                          │
│  - PostgreSQL 主库                  - PostgreSQL 备库                  │
│  - Redis 主缓存                     - moka 进程内 LRU                  │
└──────────────────────────────────────────────────────────────────────┘
                              ↓
┌──────────────────────────────────────────────────────────────────────┐
│  数据库 (PostgreSQL 16)                                               │
│  - failover_status (主备实时状态)                                      │
│  - failover_event (切换事件流水)                                       │
│  - failover_config (配置持久化)                                        │
└──────────────────────────────────────────────────────────────────────┘
```

### 1.1 核心约束

1. **仅主调用不可用时才切换**：主调用正常运行时禁用备用
2. **故障转移后支持回切**：主调用恢复后自动回切
3. **配置化**：通过 `failover.toml` 配置主备 URL + 超时 + 熔断参数
4. **统一抽象**：`FailoverCall<T, E>` trait，P1/P2 阶段可复用
5. **租户隔离**：所有查询走 `extract_tenant_id(&auth)?`，严禁 `auth.tenant_id.unwrap_or(0)`

---

## 2. 数据模型

### 2.1 failover_status（主备实时状态）

```sql
CREATE TABLE failover_status (
    id BIGSERIAL PRIMARY KEY,
    function_name VARCHAR(100) NOT NULL UNIQUE,  -- 'database' / 'cache'
    current_state VARCHAR(20) NOT NULL,          -- 'primary' / 'backup' / 'both_down'
    circuit_state VARCHAR(20) NOT NULL,          -- 'closed' / 'open' / 'half_open'
    primary_url VARCHAR(500),                    -- 主调用 URL（脱敏）
    backup_type VARCHAR(50),                     -- 备用类型（postgres/redis/lru）
    last_switch_at TIMESTAMPTZ,                  -- 最近一次切换时间
    last_success_at TIMESTAMPTZ,                 -- 最近一次成功调用时间
    consecutive_failures INTEGER NOT NULL DEFAULT 0,  -- 连续失败次数
    total_primary_calls BIGINT NOT NULL DEFAULT 0,
    total_backup_calls BIGINT NOT NULL DEFAULT 0,
    total_switches BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_failover_status_func ON failover_status(function_name);
```

### 2.2 failover_event（切换事件流水）

```sql
CREATE TABLE failover_event (
    id BIGSERIAL PRIMARY KEY,
    function_name VARCHAR(100) NOT NULL,
    event_type VARCHAR(50) NOT NULL,             -- 'switch_to_backup' / 'switch_back' / 'primary_recovered' / 'both_failed'
    from_state VARCHAR(20),
    to_state VARCHAR(20),
    reason TEXT,                                 -- 失败原因
    latency_ms INTEGER,
    tenant_id BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_failover_event_func_time ON failover_event(function_name, created_at DESC);
CREATE INDEX idx_failover_event_type ON failover_event(event_type);
```

### 2.3 failover_config（配置持久化）

```sql
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

---

## 3. 后端设计

### 3.1 目录结构

```
backend/src/
├── config/
│   └── failover.rs                  # 加载 config/failover.toml
├── utils/
│   └── failover/
│       ├── mod.rs                   # FailoverCall trait + FailoverError
│       ├── circuit_breaker.rs       # 熔断器
│       ├── database.rs              # FailoverDatabase 实现
│       └── cache.rs                 # FailoverCache 实现
├── monitoring/
│   └── failover_metrics.rs          # Prometheus 指标
├── handlers/
│   └── failover_handler.rs          # 3 个 HTTP 端点
├── services/
│   └── failover_service.rs          # 业务逻辑
├── models/
│   ├── failover_status.rs
│   ├── failover_event.rs
│   └── failover_config.rs
└── routes/
    └── failover.rs                  # 路由注册
```

### 3.2 FailoverCall trait（统一接口）

```rust
// backend/src/utils/failover/mod.rs
use std::time::Duration;
use async_trait::async_trait;
use thiserror::Error;

/// 主备调用错误
#[derive(Debug, Error)]
pub enum FailoverError<E> {
    #[error("primary failed: {0}")]
    PrimaryFailed(E),
    #[error("backup failed: {0}")]
    BackupFailed(E),
    #[error("both failed: primary={0:?}, backup={1:?}")]
    BothFailed(E, E),
    #[error("both timeout")]
    BothTimeout,
    #[error("circuit open")]
    CircuitOpen,
}

/// 主备调用 trait
#[async_trait]
pub trait FailoverCall<T, E> {
    /// 主调用
    async fn primary_call(&self) -> Result<T, E>;
    /// 备用调用
    async fn backup_call(&self) -> Result<T, E>;
    /// 主调用超时
    fn primary_timeout(&self) -> Duration;
    /// 备用调用超时
    fn backup_timeout(&self) -> Duration;
    /// 函数名（监控用）
    fn function_name(&self) -> &str;

    /// 带主备隔离的调用
    async fn call(&self) -> Result<T, FailoverError<E>> {
        // 检查熔断器
        if self.circuit_is_open() {
            return self.try_backup().await;
        }

        // 尝试主调用
        match tokio::time::timeout(self.primary_timeout(), self.primary_call()).await {
            Ok(Ok(v)) => {
                self.record_primary_success();
                Ok(v)
            }
            Ok(Err(e)) => {
                self.record_primary_failure();
                self.try_backup().await
            }
            Err(_) => {
                self.record_primary_timeout();
                self.try_backup().await
            }
        }
    }
}
```

### 3.3 熔断器

```rust
// backend/src/utils/failover/circuit_breaker.rs
pub enum CircuitState {
    Closed,    // 关闭（正常）
    Open,      // 打开（熔断）
    HalfOpen,  // 半开（探测）
}

pub struct CircuitBreaker {
    threshold: u32,         // 熔断阈值（默认 5）
    duration: Duration,     // 熔断时长（默认 30s）
    state: AtomicU8,       // 当前状态
    consecutive_failures: AtomicU32,
    opened_at: AtomicI64,  // 熔断开始时间
}
```

### 3.4 数据库主备实现

```rust
// backend/src/utils/failover/database.rs
pub struct FailoverDatabase {
    primary: DatabaseConnection,         // PostgreSQL 主
    backup: DatabaseConnection,          // PostgreSQL 备
    circuit: Arc<CircuitBreaker>,
    config: DatabaseFailoverConfig,
}

#[async_trait]
impl FailoverCall<Vec<Row>, DbErr> for FailoverDatabase {
    async fn primary_call(&self) -> Result<Vec<Row>, DbErr> {
        // 通过主连接执行 ping 查询
        ping(&self.primary).await
    }
    async fn backup_call(&self) -> Result<Vec<Row>, DbErr> {
        ping(&self.backup).await
    }
    // ...
}
```

### 3.5 缓存主备实现

```rust
// backend/src/utils/failover/cache.rs
use moka::future::Cache;

pub struct FailoverCache {
    primary: redis::Client,              // Redis 主
    backup: Cache<String, Vec<u8>>,     // moka LRU 备
    circuit: Arc<CircuitBreaker>,
    config: CacheFailoverConfig,
}

#[async_trait]
impl FailoverCall<Option<Vec<u8>>, CacheError> for FailoverCache {
    async fn primary_call(&self) -> Result<Option<Vec<u8>>, CacheError> {
        // 实际查询 Redis
        self.primary.get(&self.key).await
    }
    async fn backup_call(&self) -> Result<Option<Vec<u8>>, CacheError> {
        // 查询进程内 LRU
        Ok(self.backup.get(&self.key).await)
    }
}
```

### 3.6 监控指标

```rust
// backend/src/monitoring/failover_metrics.rs
use prometheus::{IntCounterVec, IntGaugeVec};

pub struct FailoverMetrics {
    pub primary_total: IntCounterVec,           // failover_primary_total
    pub primary_failed_total: IntCounterVec,    // failover_primary_failed_total
    pub backup_total: IntCounterVec,            // failover_backup_total
    pub switch_total: IntCounterVec,            // failover_switch_total
    pub circuit_state: IntGaugeVec,             // failover_circuit_state
}
```

| 指标名 | 类型 | 标签 | 说明 |
|--------|------|------|------|
| `failover_primary_total` | Counter | function | 主调用次数 |
| `failover_primary_failed_total` | Counter | function | 主调用失败次数 |
| `failover_backup_total` | Counter | function | 备用调用次数 |
| `failover_switch_total` | Counter | function | 切换次数 |
| `failover_circuit_state` | Gauge | function | 熔断器状态（0=关闭,1=打开,2=半开）|

### 3.7 告警规则

| 规则 | 阈值 | 级别 | 文件 |
|------|------|------|------|
| 主备切换频率 | > 5 次/小时 | P2 | `monitoring/prometheus/alert_rules.yml` |
| 备用调用失败率 | > 10% | P1 | 同上 |
| 熔断器持续打开 | > 5 分钟 | P1 | 同上 |
| 主备同时不可用 | 任意时长 | P0 | 同上 |

### 3.8 配置加载

```rust
// backend/src/config/failover.rs
#[derive(Debug, Deserialize, Clone)]
pub struct FailoverConfig {
    pub database: DatabaseFailoverConfig,
    pub cache: CacheFailoverConfig,
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
```

### 3.9 HTTP API 端点

| 方法 | 路径 | 说明 | 权限 |
|------|------|------|------|
| GET | `/api/v1/erp/admin/failover/status` | 主备实时状态 | 管理员 |
| GET | `/api/v1/erp/admin/failover/metrics` | Prometheus 指标 | 管理员 |
| POST | `/api/v1/erp/admin/failover/test/switch` | 手动触发切换 | 管理员 |

### 3.10 响应示例

```json
// GET /api/v1/erp/admin/failover/status
{
  "statuses": [
    {
      "function_name": "database",
      "current_state": "primary",
      "circuit_state": "closed",
      "consecutive_failures": 0,
      "total_primary_calls": 12345,
      "total_backup_calls": 0,
      "total_switches": 0,
      "last_success_at": "2026-06-16T10:30:00Z"
    },
    {
      "function_name": "cache",
      "current_state": "backup",
      "circuit_state": "open",
      "consecutive_failures": 5,
      "total_primary_calls": 10000,
      "total_backup_calls": 234,
      "total_switches": 1,
      "last_switch_at": "2026-06-16T10:25:00Z"
    }
  ]
}
```

---

## 4. 前端设计

### 4.1 监控页面

| 页面 | 路径 | 功能 |
|------|------|------|
| 主备监控 | `/admin/failover` | 状态卡片 + 切换历史 + 手动切换按钮 |

### 4.2 页面布局

```
┌────────────────────────────────────────────────────────────┐
│  主备隔离监控                                               │
├────────────────────────────────────────────────────────────┤
│  [数据库] 主调用运行中 ✅                                    │
│    - 主库: postgresql://primary:5432/bingxi (脱敏)         │
│    - 备库: postgresql://backup:5432/bingxi (脱敏)          │
│    - 熔断: 关闭 | 切换次数: 0 | 主调用: 12,345              │
│    - [手动切换]                                             │
├────────────────────────────────────────────────────────────┤
│  [缓存] 备用调用中 ⚠️                                       │
│    - 主缓存: redis://primary:6379 (脱敏)                   │
│    - 备缓存: 进程内 LRU (max 10,000 entries)                │
│    - 熔断: 打开 | 切换次数: 1 | 主调用: 10,000 / 备用: 234  │
│    - [强制回切主调用]                                       │
├────────────────────────────────────────────────────────────┤
│  切换历史（最近 20 条）                                      │
│  时间                  | 函数   | 事件           | 延迟      │
│  2026-06-16 10:25:00  | cache  | 切换至备用     | 3200ms   │
│  2026-06-16 10:24:30  | cache  | 主调用超时     | 3000ms   │
└────────────────────────────────────────────────────────────┘
```

### 4.3 文件结构

```
frontend/src/views/admin/
├── failover.vue              # 主页面
└── components/
    ├── FailoverStatusCard.vue  # 状态卡片
    ├── FailoverEventList.vue   # 切换历史列表
    └── FailoverMetrics.vue     # 指标展示
```

---

## 5. 测试设计

### 5.1 单元测试

| 文件 | 测试内容 |
|------|---------|
| `failover/mod.rs` | FailoverCall trait 单元测试（mock 主备） |
| `failover/circuit_breaker.rs` | 熔断器状态转换测试 |
| `failover/database.rs` | 主备切换逻辑测试 |
| `failover/cache.rs` | Redis + LRU 切换测试 |

### 5.2 集成测试

| 文件 | 测试场景 |
|------|---------|
| `tests/failover_database_test.rs` | 主库连接失败 → 自动切备库 |
| `tests/failover_cache_test.rs` | Redis 超时 → 自动切 LRU |
| `tests/failover_circuit_test.rs` | 连续 5 次失败 → 熔断器打开 |
| `tests/failover_recovery_test.rs` | 主库恢复 → 半开探测 → 自动回切 |
| `tests/failover_api_test.rs` | 3 个 HTTP 端点集成测试 |

### 5.3 故障注入（chaos test）

| 场景 | 注入方式 | 预期行为 |
|------|---------|---------|
| 主库连接拒绝 | `iptables -A OUTPUT -p tcp --dport 5432 -j DROP` | 自动切备库 |
| 主库连接超时 | `tc qdisc add dev eth0 root netem delay 5000ms` | 主调用超时 → 切备 |
| Redis 不可用 | `docker stop redis-primary` | 自动切 LRU |
| 熔断后探测 | 主库恢复后等待 30s | 半开探测 → 回切 |
| 双不可用 | 同时关闭主备 | 返回 `BothFailed` 错误 |

### 5.4 监控验证

- ✅ 5 个 Prometheus 指标可抓取
- ✅ 切换事件写入 `failover_event` 表
- ✅ 告警规则触发条件正确
- ✅ Grafana dashboard 导入后展示主备状态

---

## 6. 部署设计

### 6.1 配置文件

```toml
# config/failover.toml.example
[database]
primary_url = "postgresql://user:pass@primary.example.com:5432/bingxi"
backup_url = "postgresql://user:pass@backup.example.com:5432/bingxi"
primary_timeout_ms = 3000
backup_timeout_ms = 5000
circuit_breaker_threshold = 5
circuit_breaker_duration_s = 30

[cache]
primary_url = "redis://redis.example.com:6379"
backup_max_entries = 10000
primary_timeout_ms = 1000
backup_timeout_ms = 0

[monitoring]
metrics_enabled = true
log_level = "info"
```

### 6.2 Docker 镜像

```dockerfile
# 多阶段构建：builder + runtime
FROM rust:1.94-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin server

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/server /usr/local/bin/server
COPY config /app/config
EXPOSE 8080
CMD ["server"]
```

### 6.3 docker-compose

```yaml
version: "3.9"
services:
  postgres-primary:
    image: postgres:16
    environment:
      POSTGRES_PASSWORD: ${PRIMARY_DB_PASSWORD}
  
  postgres-backup:
    image: postgres:16
    environment:
      POSTGRES_PASSWORD: ${BACKUP_DB_PASSWORD}
  
  redis-primary:
    image: redis:7
  
  app:
    build: .
    depends_on:
      - postgres-primary
      - postgres-backup
      - redis-primary
    environment:
      DATABASE_URL_PRIMARY: ${DATABASE_URL_PRIMARY}
      DATABASE_URL_BACKUP: ${DATABASE_URL_BACKUP}
      REDIS_URL: ${REDIS_URL}
```

### 6.4 启动脚本

```bash
#!/bin/bash
# start.sh
set -e
echo "启动 P0-2 主备隔离 TEST 测试版本..."
cp config/failover.toml.example config/failover.toml
docker-compose up -d
echo "✅ 启动成功，访问 http://localhost:8080/admin/failover"
```

---

## 7. 验收标准

### 7.1 功能验收

- [ ] 数据库主备隔离：主库失败 → 自动切备库，延迟 < 100ms
- [ ] 缓存主备隔离：Redis 不可用 → 自动切 LRU
- [ ] 熔断器：连续 5 次失败 → 打开，30s 后进入半开
- [ ] 回切：主库恢复后，半开探测成功 → 自动回切
- [ ] 配置化：通过 `failover.toml` 修改主备 URL 不需要改代码
- [ ] 双不可用：返回 `FailoverError::BothFailed` 错误

### 7.2 监控验收

- [ ] 5 个 Prometheus 指标可抓取
- [ ] 切换事件写入 `failover_event` 表
- [ ] 告警规则 4 条（切换/失败率/熔断/双不可用）
- [ ] Grafana dashboard 可导入

### 7.3 测试验收

- [ ] 单元测试覆盖 `FailoverCall` trait + 熔断器 + 主备逻辑
- [ ] 集成测试 5 个场景全部通过
- [ ] 故障注入 5 个场景全部通过
- [ ] TEST 测试版本可在 Docker 中启动

### 7.4 部署验收

- [ ] `dist/test-version-P0-2/` 包含 Dockerfile + docker-compose + 配置 + 启动脚本
- [ ] `docs/failover-deployment-guide.md` 部署文档完整
- [ ] PR 合到 test 分支（main 不动）
- [ ] CI/CD 4 job 全绿

---

## 8. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| 主备数据延迟 | 备库可能落后主库几秒 | 备库仅用于只读 ping / 容灾，重要数据双写 |
| 进程内 LRU 重启丢失 | Redis 故障期间写入的缓存会丢失 | LRU 设置合理 TTL，Redis 恢复后异步预热 |
| 熔断阈值过低 | 偶发失败导致频繁切换 | 默认阈值 5 次，可配置 |
| 沙箱 OOM 无法跑 cargo test | 本地无法验证 | 依赖 CI/CD 验证 + `cargo check --lib` |

---

## 9. 不在本范围

- MQ 主备隔离（P1 阶段）
- 文件存储主备隔离（P1 阶段）
- 短信/邮件主备隔离（P1 阶段）
- 搜索引擎/OCR 主备隔离（P2 阶段）
- 跨区域多活（后续阶段）
- 数据库主从同步配置（运维侧，不在代码范围）

---

## 10. 文档清单

| 文档 | 路径 |
|------|------|
| 设计 spec | `/workspace/docs/superpowers/specs/2026-06-16-failover-isolation-design.md`（本文档）|
| 实施 plan | `/workspace/docs/superpowers/plans/2026-06-16-failover-isolation-plan.md` |
| 部署指南 | `/workspace/docs/failover-deployment-guide.md` |
| 设计报告 | `/workspace/docs/superpowers/reports/2026-06-16-failover-design.md` |
| TEST 测试版本 | `/workspace/dist/test-version-P0-2/` |
| Grafana dashboard | `/workspace/monitoring/grafana/failover-dashboard.json` |
| 告警规则 | `/workspace/monitoring/prometheus/alert_rules.yml` |

---

## 11. 时间线（3 周）

- **Week 1**（后端基础）：FailoverCall trait + 熔断器 + 数据库主备 + 缓存主备 + 配置 + 监控
- **Week 2**（业务接入）：集成测试 + chaos test + API + admin 监控页面
- **Week 3**（交付）：文档 + TEST 测试版本（Docker + 配置 + 启动脚本）+ PR 合并

---

**设计版本**: v1.0
**最后更新**: 2026-06-16
**状态**: ✅ 已批准，进入实施阶段
