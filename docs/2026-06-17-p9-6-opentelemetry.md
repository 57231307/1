# P9-6 OpenTelemetry 集成报告

> 创建日期：2026-06-17
> 范围：后端 OpenTelemetry 一体化（trace + metrics + log）
> 三方兼容：Jaeger / Tempo / SigNoz / Honeycomb / Datadog

## 一、目标

P9-6 任务为冰溪 ERP 后端引入 **OpenTelemetry（OTel）** 标准，实现：

1. **Trace**：分布式追踪（HTTP 请求 / DB 查询 / 业务流）
2. **Metrics**：与 Prometheus 兼容的指标导出
3. **Log**：与 `tracing` 框架深度集成的结构化日志

实现 OTel 标准后，可与以下任意后端配合使用：
- **Jaeger**（开源首选）
- **Tempo**（Grafana 套件）
- **SigNoz**（现代化替代品）
- **Honeycomb**（SaaS）
- **Datadog**（商业方案）

## 二、模块设计

### 1. `backend/src/telemetry.rs` — 统一入口

提供：
- 资源属性常量：`SERVICE_NAME = "bingxi-backend"`、`SERVICE_NAMESPACE = "erp"`
- 环境变量读取：`OTEL_ENABLED`、`OTEL_EXPORTER_OTLP_ENDPOINT`、`ENV`
- 三个核心信号模块：
  - `signals::trace`（Span、SpanKind、SpanAttrs）
  - `signals::metrics`（Counter、Histogram）
  - `signals::log`（LogLevel、LogEntry）
- 预定义常量：`span_names::*` 和 `metric_names::*`
- 入口函数：`telemetry::init()` 返回 `TelemetryGuard`

### 2. `backend/src/middleware/trace.rs` — HTTP 追踪中间件

提供：
- `HttpTraceCtx`：HTTP 追踪上下文（method/path/tenant_id/trace_id/span_id）
- `HttpTraceResponse`：响应信息（status/bytes/duration）
- `TraceTimer`：耗时计时器
- W3C `traceparent` 生成与解析

**关键设计**：
- 自动跳过 `/health` `/healthz` `/ready` 健康检查
- 透传 trace_id 到下游（DB / Redis / 外部 API）
- 记录 trace_id 到响应头 `X-Trace-Id`

### 3. `backend/src/observability/config.rs` — 配置管理

提供：
- `ObservabilityConfig`：完整配置（service/endpoint/sample_ratio 等）
- `ResourceAttrs`：资源属性映射
- `from_env()`：从环境变量加载

### 4. 部署：`deploy/observability/docker-compose.yml`

包含 5 个服务：
| 服务 | 端口 | 用途 |
|------|------|------|
| **Jaeger** | 16686 / 4317 | 追踪 UI + OTLP gRPC |
| **OTel Collector** | 4317 / 4318 | 生产环境 OTLP 收集 |
| **Prometheus** | 9090 | 指标采集 |
| **Grafana** | 3000 | 可视化 |
| **Backend** | 8080 | 后端服务（dev profile） |

## 三、三位一体信号设计

### 1. Trace（分布式追踪）

| 字段 | 说明 |
|------|------|
| `service.name` | `bingxi-backend` |
| `service.namespace` | `erp` |
| `service.version` | 编译期注入 |
| `deployment.environment` | `ENV` 环境变量 |
| `tenant.id` | 多租户隔离字段 |

预定义 Span：
- `http.request` — HTTP 请求
- `db.query` — 数据库查询
- `redis.op` — Redis 操作
- `business.flow` — 业务流
- `sales.order.create` / `purchase.order.create` / `inventory.transfer` / `ar.payment` — 业务事件

### 2. Metrics（指标）

预定义指标（与 Prometheus 兼容）：
- `http_requests_total` — HTTP 请求总数
- `http_request_duration_seconds` — HTTP 请求耗时
- `db_queries_total` — DB 查询总数
- `db_query_duration_seconds` — DB 查询耗时
- `business_events_total` — 业务事件总数
- `active_tenants` — 活跃租户数

实现：
- `Counter`（原子计数器）
- `Histogram`（直方图）

### 3. Log（日志）

与 `tracing` 框架深度集成：
- 自动注入 `trace_id` / `span_id` 到日志字段
- 支持 5 个级别：Trace / Debug / Info / Warn / Error
- JSON 格式输出（生产）/ 文本格式输出（开发）

## 四、单元测试覆盖

| 模块 | 测试数 |
|------|--------|
| `telemetry::tests` | 11 |
| `middleware::trace::tests` | 6 |
| `observability::config::tests` | 3 |
| **合计** | **20 测试** |

覆盖：
- 服务元数据默认值
- Span kind 字符串映射
- Span 属性构造
- Counter / Histogram 操作
- Log level 排序
- 预定义常量正确性
- HTTP 上下文创建与 traceparent 生成
- 健康检查跳过
- 计时器精度

## 五、启用 OTel SDK（可选）

默认情况下，本模块使用 `tracing` 框架记录 span，**不引入重依赖**。

要启用完整 OTel 导出，添加以下依赖到 `backend/Cargo.toml`：

```toml
opentelemetry = { version = "0.24", features = ["trace"] }
opentelemetry-otlp = { version = "0.17", features = ["grpc-tonic"] }
opentelemetry_sdk = { version = "0.24", features = ["rt-tokio"] }
tracing-opentelemetry = "0.25"
```

然后在 `main.rs` 中：

```rust
use opentelemetry::global;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::TracerProvider;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt;

let tracer = opentelemetry_otlp::new_pipeline()
    .tracing()
    .with_exporter(opentelemetry_otlp::new_exporter().tonic().with_endpoint("http://localhost:4317"))
    .install_batch(opentelemetry_sdk::runtime::Tokio)?;

let subscriber = tracing_subscriber::registry()
    .with(OpenTelemetryLayer::new(tracer))
    .with(tracing_subscriber::EnvFilter::from_default_env());

tracing::subscriber::set_global_default(subscriber)?;
```

## 六、本地启动

```bash
cd deploy/observability
docker-compose up -d  # 启动 Jaeger + Prometheus + Grafana

# 访问
# - Jaeger UI:    http://localhost:16686
# - Prometheus:   http://localhost:9090
# - Grafana:      http://localhost:3000 (admin / admin)

# 启动后端
export OTEL_ENABLED=true
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
cargo run --bin bingxi-backend
```

## 七、约束遵守

- ✅ **零硬编码**：所有端点从环境变量读取
- ✅ **多租户隔离**：trace 中携带 `tenant_id`
- ✅ **无破坏性变更**：telemetry 模块为新增，tracing 框架保留
- ✅ **可选依赖**：OTel SDK 在用户启用时才引入
- ✅ **资源属性标准化**：遵循 OTel Resource SemConv

## 八、风险评估

| 风险 | 等级 | 缓解措施 |
|------|------|----------|
| OTel SDK 引入编译失败 | 中 | 默认不引入，仅提供 trait 与文档 |
| 性能开销 | 低 | 采样率可配（默认 1.0） |
| 内存泄漏 | 低 | TelemetryGuard 在 drop 时 flush |
| 与现有 tracing 冲突 | 极低 | 新增 layer，不替换 Subscriber |
