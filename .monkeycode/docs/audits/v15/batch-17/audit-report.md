# V15 可观测性与运维审计报告（类二十·批次 17）

- **审计子代理**：V15 审计子代理（类二十可观测性与运维）
- **审计范围**：8 维度（20.1 trace / 20.2 metrics / 20.3 WebSocket / 20.4 failover / 20.5 慢查询 / 20.6 API 网关 / 20.7 系统版本 / 20.8 日志增强）
- **审计依据**：
  - `/workspace/.monkeycode/docs/audits/v15-review-plan-2026-07-15.md` 第 6369-6464 行
  - `/workspace/backend/src/observability/{mod.rs,config.rs,span.rs,trace_context.rs}`
  - `/workspace/backend/src/middleware/{trace_context.rs,metrics.rs,slow_query.rs,rate_limit.rs,omni_audit.rs,auth.rs}`
  - `/workspace/backend/src/services/{metrics_service.rs,business_metrics.rs,slow_query_collector.rs,failover_service.rs,enhanced_logger.rs,system_update_service.rs,audit_cleanup_service.rs,sensitive_action_alert.rs,event_kafka.rs,init_service.rs}`
  - `/workspace/backend/src/handlers/{health_handler.rs,failover_handler.rs,slow_query_handler.rs,api_gateway_handler.rs,system_update_handler.rs}`
  - `/workspace/backend/src/websocket/notifications.rs`
  - `/workspace/backend/src/models/{system_version.rs,failover_config.rs,failover_event.rs,failover_status.rs,slow_query.rs}`
  - `/workspace/backend/src/utils/log_config.rs`
  - `/workspace/backend/src/cli/util/backup.rs`
  - `/workspace/monitoring/{prometheus/prometheus.yml,prometheus/alert_rules.yml,prometheus/failover-alert-rules.yml,alertmanager/alertmanager.yml,grafana/dashboards/bingxi-erp-overview.json}`
- **审计方法**：Read 审计计划 + Grep 检索 + Read 关键文件 + 对照审计计划逐项核对
- **审计时间**：2026-07-16
- **审计原则**：只做审计不修改业务代码

---

## 维度 20.1：可观测性 trace 链路完整性审计

### 检查方法
- Read `backend/src/observability/{mod.rs,config.rs,span.rs,trace_context.rs}`
- Read `backend/src/middleware/trace_context.rs`
- Grep `traceparent|inject.*trace|propagat|kafka.*trace|reqwest.*trace` 跨服务传递
- Grep `sample_ratio|sampling|always_off|always_on|trace.*retention|trace.*7.*day` 采样与保留策略

### 发现

#### ✅ 已落实的项
1. **W3C Trace Context 标准实现**：`backend/src/observability/trace_context.rs:1-146` 实现完整 W3C Trace Context Level 2 协议（traceparent 解析、trace_id 生成、span_id 生成、sampled 标志位），与 Jaeger/Tempo/DataDog 等后端兼容。
2. **trace_id 单请求唯一**：`backend/src/middleware/trace_context.rs:27-68` 中间件为每个请求生成/复用 trace_id，放入 `Request::extensions()` 供下游 handler/service 读取，并在响应头 `X-Trace-Id` 回写（line 49-54），客户端可日志关联。
3. **span 上下文 parent_span_id 传递**：`backend/src/observability/trace_context.rs:135-146` `extract_or_new` 函数从入站 `traceparent` 解析父 span_id，并为本请求生成新 span_id；`backend/src/observability/span.rs:47-57` `root_span` 把 `parent_span_id` 写入 span 字段，支持调用链树状展示。
4. **traceparent 入站解析校验**：`backend/src/observability/trace_context.rs:54-95` `from_traceparent` 严格校验版本号/trace_id/parent_id/flags 格式（32 hex/16 hex/全 0 拒绝），fail-open 策略：解析失败 fallback 到新 root。
5. **trace 采样率可配置**：`backend/src/observability/config.rs:46-94` `ObservabilityConfig.sample_ratio` 通过 `OTEL_SAMPLE_RATIO` 环境变量可配置（0.0-1.0）。
6. **业务域 span 宏**：`backend/src/observability/span.rs:30-38` `span_business!` 宏统一业务 span 创建，自动添加 `business.` 前缀。
7. **trace.complete 日志输出**：`backend/src/middleware/trace_context.rs:56-65` 每请求结束输出结构化日志，含 trace_id/span_id/method/path/status/elapsed_ms。

#### ❌ 缺陷项

**缺陷 20.1-A：trace 跨服务（HTTP 下游调用）传递缺失**
- **风险等级：P1**
- **证据**：
  - Grep `reqwest.*header.*trace|inject.*trace|propagat.*trace` 全工程仅命中 1 处（test 函数），未发现业务代码在调用下游服务时注入 `traceparent` header。
  - `backend/src/services/system_update_service.rs:604-625` `fetch_latest_release` 调用 GitHub API 时未注入 traceparent header。
  - `backend/src/services/ai_extend_service.rs` 调用 AI 服务时未注入 traceparent（可验证）。
- **业务影响**：跨服务调用链断裂，无法在 Jaeger 中看到完整调用链（如 backend → GitHub API / AI 服务），故障定位困难。
- **修复建议**：实现 `inject_trace_context(headers: &mut HeaderMap, ctx: &TraceContext)` 工具函数，在所有 reqwest 调用前注入 `traceparent` header。

**缺陷 20.1-B：trace 跨事件总线（Kafka）传递缺失**
- **风险等级：P1**
- **证据**：`backend/src/services/event_kafka.rs:222-227`
  ```rust
  let record = Record {
      key: None,
      value: Some(payload_json),
      headers: BTreeMap::new(),   // ← 空 headers，未注入 traceparent
      timestamp: chrono::Utc::now(),
  };
  ```
- **业务影响**：事件消费方无法关联生产方的 trace_id，跨微服务事件链路追踪断裂。Kafka 消息消费失败时无法定位是哪个生产请求触发的。
- **修复建议**：在 `publish` 中从当前 `tracing::Span` 提取 trace_id/span_id，构造 W3C traceparent 字符串放入 `headers`（如 `"traceparent" → "00-{trace_id}-{span_id}-01"`）；消费方在 `subscribe` 中解析 header 并新建 child span。

**缺陷 20.1-C：trace 采样策略不符合生产最佳实践**
- **风险等级：P2**
- **证据**：`backend/src/observability/config.rs:54-63`
  ```rust
  sample_ratio: 1.0,   // ← 默认 100% 采样
  ```
  计划要求"生产环境采样率必须可配置（默认 10%），错误请求 100% 采样"。当前实现：
  - 默认 100% 采样（高负载下产生海量 span，影响后端存储与查询性能）
  - 无错误请求强制采样逻辑（无论请求成功失败都按 sample_ratio 采样）
- **业务影响**：生产环境 100% 采样会导致 Jaeger/Tempo 存储爆炸；错误请求被采样丢弃时无法定位问题。
- **修复建议**：
  1. 默认 `sample_ratio` 改为 0.1（10%）
  2. 实现 tail-based sampling：错误请求（5xx）/慢请求（P95+）强制 100% 采样，可在 OTel Collector 层配置

**缺陷 20.1-D：trace 数据保留期未配置**
- **风险等级：P2**
- **证据**：Grep `trace.*retention|trace.*archive|trace.*7.*day|trace.*过期` 全工程无命中。
- **业务影响**：trace 数据无保留策略，Jaeger/Tempo 后端存储无限增长；过期 trace 数据未归档为统计数据，无法长期趋势分析。
- **修复建议**：在 `deploy/observability` 的 Jaeger/Tempo 配置中设置 retention 7 天，超期归档；OTel Collector 配置 metrics transformation 把 trace 转为 RED 指标（Rate/Errors/Duration）长期保留。

---

## 维度 20.2：metrics 指标体系与告警审计

### 检查方法
- Read `backend/src/services/metrics_service.rs`
- Read `backend/src/middleware/metrics.rs`
- Read `backend/src/services/business_metrics.rs`（grep 验证）
- Read `monitoring/prometheus/{prometheus.yml,alert_rules.yml,failover-alert-rules.yml}`
- Read `monitoring/alertmanager/alertmanager.yml`
- Read `monitoring/grafana/dashboards/bingxi-erp-overview.json`

### 发现

#### ✅ 已落实的项
1. **基础指标完整采集**：`backend/src/services/metrics_service.rs:78-104` 注册 7 类基础指标：`http_requests_total` / `http_requests_in_flight` / `http_request_duration_seconds` / `db_connections` / `db_query_duration_seconds` / `business_operations_total` / `errors_total`。
2. **带标签指标（per-route/per-status）**：`metrics_service.rs:96-104` 注册 4 类带标签指标：`http_requests_by_route{method,route,status}` / `http_request_duration_by_route{method,route}` / `http_requests_by_status_class{class}` / `business_operations_by_type{operation}`，覆盖 QPS/延迟/错误率三大维度。
3. **业务指标扩展**：`metrics_service.rs:316-332` `BusinessMetrics` 注册 20+ `erp_*` 指标（orders/cache_hits/login 等），通过同一 Registry 暴露在 `/metrics` 端点。
4. **指标自动采集中间件**：`backend/src/middleware/metrics.rs:32-75` `metrics_middleware` 自动按 method/route/status 记录带标签指标 + 基础指标 + 错误计数 + 慢请求检测。
5. **Prometheus 抓取配置**：`monitoring/prometheus/prometheus.yml:23-53` 配置 4 个 job：`prometheus` 自监控 / `bingxi-backend`（host.docker.internal:8080/metrics）/ `node-exporter`（系统指标）/ `postgresql`（DB 指标），抓取间隔 15s。
6. **告警规则配置完整**：`monitoring/prometheus/alert_rules.yml` 配置 6 组共 16 条告警规则：
   - HTTP 告警：HighHTTPErrorRate（5xx > 5%，critical）/ HTTPRequestDrop / HighHTTPLatency（P95 > 2s）/ TooManyInFlightRequests
   - 数据库告警：SlowDatabaseQuery（P95 > 1s）/ HighDatabaseConnections（> 50）/ LowDatabaseConnections
   - 业务告警：HighBusinessErrorRate / BusinessOperationDrop
   - 系统告警：HighCPUUsage（> 80%）/ HighMemoryUsage（> 85%）/ HighDiskUsage（> 85%）/ DiskSpaceLow（> 90%，critical）/ HighSystemLoad
   - 应用告警：ServiceDown（critical）/ TargetDown / FrequentRateLimiting
   - 日志告警：HighErrorLogRate
7. **告警分级（P0/P1/P2）**：`alert_rules.yml:18-19,159,181` 等通过 `severity` 标签分级：critical（如 HighHTTPErrorRate/DiskSpaceLow/ServiceDown）/ warning / info，符合 P0/P1/P2 分级要求。
8. **告警通知分级路由**：`monitoring/alertmanager/alertmanager.yml:75-124` 配置 4 个 receiver（default-receiver/critical-alerts/warning-alerts/info-alerts），按 severity 路由到不同邮箱，critical 立即通知（group_wait 10s，repeat_interval 1h）。
9. **告警去重（5min 内不重复）**：`alertmanager.yml:80-83`
   ```yaml
   group_by: ['alertname', 'severity', 'service']
   group_wait: 30s
   group_interval: 5m
   repeat_interval: 4h   # critical 1h，warning 4h，info 12h
   ```
   通过 `group_interval: 5m` + `repeat_interval` 实现告警去重，5min 内同组告警合并。
10. **告警抑制规则**：`alertmanager.yml:127-147` 配置 3 条抑制规则：ServiceDown 抑制其他告警 / DiskSpaceLow 抑制 HighDiskUsage / HighCPUUsage 抑制 HighSystemLoad，避免告警风暴。
11. **Grafana 看板存在**：`monitoring/grafana/dashboards/bingxi-erp-overview.json` 提供"ERP 系统监控总览"看板，10s 自动刷新，时区 Asia/Shanghai，含 HTTP 请求总数/活跃请求/错误率/响应时间等面板。
12. **SMTP 密码环境变量化**：`alertmanager.yml:14` `smtp_auth_password: '${SMTP_AUTH_PASSWORD}'`（v10 P1-5 修复），不硬编码密码。
13. **failover 专属告警规则**：`monitoring/prometheus/failover-alert-rules.yml` 配置 4 条 P0/P1/P2 告警（FailoverSwitchFrequent/FailoverBackupFailureRate/FailoverCircuitOpenLong/FailoverBothDown），含 runbook_url。
14. **敏感操作告警**：`backend/src/services/sensitive_action_alert.rs:74-134` `SensitiveActionAlert::check_and_alert` 自动识别 10 类敏感操作（Delete/PermissionChange/UserManagement/RoleManagement/SystemConfig/DataExport/BatchOperation/LoginFailure/PasswordChange/FinancialOperation），按级别（Critical/High/Medium）输出告警日志，集成在 `omni_audit` 中间件中（`middleware/omni_audit.rs:235-241`）。

#### ❌ 缺陷项

**缺陷 20.2-A：网络指标（network IO）未采集**
- **风险等级：P3**
- **证据**：
  - Grep `network|cpu|memory|disk|QPS|network_io` 在 `business_metrics.rs` 无命中。
  - `monitoring/prometheus/prometheus.yml` 配置了 `node-exporter` job（line 40-44），可采集 node_network_* 指标，但 `monitoring/grafana/dashboards/bingxi-erp-overview.json` Grep `node_cpu|node_memory|node_filesystem|node_network` 无命中，看板未展示系统级指标。
- **业务影响**：网络 IO 不在监控看板，网络瓶颈无法可视化发现。
- **修复建议**：在 `bingxi-erp-overview.json` 看板新增系统资源面板（CPU/内存/磁盘/网络），数据源使用 `node_*` 指标。

**缺陷 20.2-B：告警通知渠道仅邮箱，企业微信/钉钉未启用**
- **风险等级：P3**
- **证据**：`monitoring/alertmanager/alertmanager.yml:53-60`
  ```yaml
  # 企业微信 webhook（可选）
  # webhook_configs:
  #   - url: 'https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=YOUR_KEY'
  #     send_resolved: true
  # 钉钉 webhook（可选）
  # webhook_configs:
  #   - url: 'https://oapi.dingtalk.com/robot/send?access_token=YOUR_TOKEN'
  ```
  企业微信/钉钉配置全部被注释，仅 default/critical/warning/info 4 个 receiver 都用邮件通知。
- **业务影响**：critical 告警仅邮件通知，运维人员可能错过处理时机；计划要求"告警必须通知到负责人（邮件/短信/钉钉）"。
- **修复建议**：critical-alerts receiver 启用企业微信/钉钉 webhook（实际部署时填入真实 key），短信通知可对接阿里云短信。

**缺陷 20.2-C：Alertmanager 目标未在 prometheus.yml 中启用**
- **风险等级：P3**
- **证据**：`monitoring/prometheus/prometheus.yml:16-21`
  ```yaml
  alerting:
    alertmanagers:
      - static_configs:
          - targets:
            # - alertmanager:9093   ← 被注释
  ```
  Prometheus 未实际连接 Alertmanager，告警规则触发后无处发送。
- **业务影响**：告警链路断裂，告警规则触发后无法到达通知渠道。
- **修复建议**：部署 Alertmanager 后取消注释 ` - alertmanager:9093`，确保告警链路畅通。

**缺陷 20.2-D：看板未包含系统资源（CPU/内存/磁盘/网络）面板**
- **风险等级：P3**
- **证据**：`monitoring/grafana/dashboards/bingxi-erp-overview.json` 仅展示应用层指标（HTTP/DB/业务），Grep `node_cpu|node_memory|node_filesystem|node_network` 无命中。
- **业务影响**：CPU/内存/磁盘/网络等关键系统资源未在看板展示，运维需切换多个看板才能定位问题。
- **修复建议**：在 `bingxi-erp-overview.json` 新增系统资源 row，包含 4 个面板（CPU 使用率/内存使用率/磁盘使用率/网络吞吐）。

---

## 维度 20.3：WebSocket 实时推送可靠性审计

### 检查方法
- Read `backend/src/websocket/{mod.rs,notifications.rs}`
- Grep `Redis.*Pub.*Sub|redis_pubsub|broadcast.*redis|多实例.*广播` 多实例广播
- Grep `ping.*30|heartbeat|timeout|60.*sec|秒.*超时|ACK|ack` 心跳与 ACK

### 发现

#### ✅ 已落实的项
1. **WebSocket 路由设计**：`backend/src/websocket/mod.rs:1-18` 路由设计合理：
   - `POST /api/v1/erp/ws/ticket`：签发一次性短时票据（需 JWT 认证）
   - `GET /api/v1/erp/ws/notifications?ticket=<票据>`：通知实时推送
2. **票据鉴权（替代 URL query JWT）**：`backend/src/websocket/notifications.rs:172-287` v12 P1-4 修复：
   - 票据 30 秒过期（`WS_TICKET_TTL: Duration = Duration::from_secs(30)`，line 175）
   - 一次性消费（`validate_and_consume` 调用 `tickets.remove(ticket)`，line 253）
   - 256 bit 随机量（UUID v4 两次拼接 = 64 字符 hex，line 217-221）
   - 懒清理（每签发 128 张票据清理一次过期票据，line 232-237）
3. **客户端 ping 心跳支持**：`notifications.rs:368-373` 客户端可发送 `WsMessage::Ping { timestamp }`，服务端记录 debug 日志。
4. **连接管理器（按 user_id 分组）**：`notifications.rs:65-122` `ConnectionManager` 使用 `DashMap<i64, broadcast::Sender<String>>`，一个用户可有多个连接（多端登录），初始容量 100，自动扩容。
5. **全局单例广播器**：`notifications.rs:155-170` `NOTIFICATION_BROADCASTER` 全局单例（OnceLock），ws handler 与 notification_service 共享同一份 manager（批次 24 v6 P0-2 修复）。
6. **panic 隔离**：`notifications.rs:363-419,427-452` 接收/推送任务均用 `AssertUnwindSafe + catch_unwind` 隔离单次 panic，不退出循环。
7. **任务 abort 防泄漏**：`notifications.rs:459-472` select! 后显式 `recv_task.abort()` + `send_task.abort()`，避免 detached task 泄漏（L-31 修复）。
8. **断开自动清理**：`notifications.rs:475` `manager.unregister(auth.user_id)` 在连接结束时清理。

#### ❌ 缺陷项

**缺陷 20.3-A：消息 ACK 机制缺失**
- **风险等级：P1**
- **证据**：
  - `backend/src/websocket/notifications.rs:32-46` `WsMessage` 枚举仅 5 个变体：Notification/Ping/Pong/Error/MarkAsRead，**无 Ack 变体**。
  - Grep `ACK|ack` 在 `notifications.rs` 无命中。
  - 服务端广播使用 `broadcast::Sender<String>`（line 109-121），send 失败仅记录 warn 日志，**不重发**。
- **业务影响**：消息推送后客户端未收到时无重发机制，通知可能丢失（如客户端网络抖动）。计划要求"消息必须支持 ACK 机制，未确认消息重发（最多 3 次）"。
- **修复建议**：
  1. 在 `WsMessage` 新增 `Ack { message_id: String }` 变体
  2. 服务端广播时记录 `message_id` + 发送时间，启动 5s 超时定时器
  3. 收到 Ack 后移除待确认记录；超时未收到则重发（最多 3 次，每次间隔 5s/10s/30s）

**缺陷 20.3-B：多实例广播（Redis Pub/Sub）缺失**
- **风险等级：P1**
- **证据**：
  - Grep `Redis.*Pub.*Sub|redis_pubsub|broadcast.*redis|多实例.*广播` 全工程无命中。
  - `notifications.rs:65-122` `ConnectionManager` 使用本地 `DashMap`，**连接信息仅在当前实例内存中**。
  - `notifications.rs:155-170` `NOTIFICATION_BROADCASTER` 是进程内单例，跨实例不共享。
- **业务影响**：多实例部署时，user A 连接在实例 1，notification_service 在实例 2 调用 `broadcast_notification`，消息只能广播到实例 2 的连接，**实例 1 的 user A 收不到通知**。计划要求"多实例部署时必须用 Redis Pub/Sub 广播消息，确保所有连接收到"。
- **修复建议**：
  1. 在 `NotificationBroadcaster::broadcast_notification` 中，本地广播 + 同时 publish 到 Redis channel `ws:notifications:{user_id}`
  2. 每个实例启动时 subscribe `ws:notifications:*` pattern，收到 Redis 消息后调用本地 `manager.broadcast` 推送给本实例的连接
  3. 可复用 `RateLimitMiddleware` 的 Redis 连接（`RATE_LIMIT_REDIS_URL`）

**缺陷 20.3-C：服务端主动心跳与超时断开缺失**
- **风险等级：P2**
- **证据**：
  - `backend/src/websocket/notifications.rs:368-373` 客户端 ping 服务端只记录 debug 日志，**服务端不主动发送 Ping**。
  - Grep `ping.*30|heartbeat|timeout|60.*sec|秒.*超时` 在 `notifications.rs` 无命中。
  - 没有"30s ping + 60s 超时断开"机制。
- **业务影响**：
  1. 客户端网络半开连接（如 NAT 超时）服务端无法检测，连接长期挂在 `ConnectionManager` 中占用资源
  2. 计划要求"连接必须有超时/重连/心跳机制（30s ping，超时 60s 断开）"
- **修复建议**：
  1. 在 `handle_socket` 推送任务中增加 `tokio::time::interval(Duration::from_secs(30))`，定时发送 `WsMessage::Ping`
  2. 接收任务记录最后收到客户端消息的时间，超过 60s 无任何消息（含 Pong）则主动断开连接

---

## 维度 20.4：故障转移主备切换回切审计

### 检查方法
- Read `backend/src/services/failover_service.rs`
- Read `backend/src/handlers/failover_handler.rs`
- Read `backend/src/routes/failover.rs`
- Read `backend/src/models/{failover_config.rs,failover_event.rs,failover_status.rs}`
- Grep `consecutive_failures|circuit.*open|circuit.*half|健康检查.*5s|5.*秒.*健康|streaming.*replication|流复制|failover.*executor|failover_exec|主备切换.*自动|auto.*failover`

### 发现

#### ✅ 已落实的项
1. **failover 数据模型完整**：`backend/src/models/{failover_config.rs,failover_event.rs,failover_status.rs}` 提供配置表 / 事件表 / 状态表三张表，记录 function_name / current_state / circuit_state / consecutive_failures / total_primary_calls / total_backup_calls / total_switches 等字段。
2. **手动切换端点**：`backend/src/handlers/failover_handler.rs:72-87` `POST /api/v1/erp/admin/failover/test/switch` 手动触发切换，仅 admin 路由（`/api/v1/erp/admin/`），function 限定为 database/cache。
3. **切换事件可追溯**：`backend/src/services/failover_service.rs:208-233` `record_event` 记录 function_name/event_type/from_state/to_state/reason/latency_ms/created_at，支持切换历史回溯。
4. **状态更新事务化**：`failover_service.rs:236-290` `update_status` 使用事务包裹 find + update/insert（P1-15 + P1-4a 修复），防止并发场景下重复 insert。
5. **健康检查端点**：`failover_service.rs:321-349` `health_check` 检查数据库 + 缓存连接状态。
6. **Prometheus 指标完整**：`failover_service.rs:28-122` `FailoverMetrics` 注册 5 类指标：primary_total / primary_failed_total / backup_total / switch_total / circuit_state，按 function 标签分类。
7. **failover 告警规则**：`monitoring/prometheus/failover-alert-rules.yml` 配置 4 条告警（FailoverSwitchFrequent/FailoverBackupFailureRate/FailoverCircuitOpenLong/FailoverBothDown），P0/P1/P2 分级，含 runbook_url。
8. **Grafana 故障切换看板**：`monitoring/grafana/failover-dashboard.json` 提供独立看板。
9. **failover 配置表 model 保留**：`backend/src/models/failover_config.rs` 保留 `failover_config` 表 model（标注 `#![allow(dead_code)]` + TODO tech-debt），便于未来接入业务。

#### ❌ 缺陷项

**缺陷 20.4-A：自动故障检测机制缺失（5s 间隔 / 连续 3 次失败触发）**
- **风险等级：P0**
- **证据**：
  - Grep `consecutive_failures|circuit.*open|circuit.*half|健康检查.*5s|5.*秒.*健康` 仅在 `failover_status.rs:43,73,90` 和 `failover_service.rs:273` 命中（consecutive_failures 字段，但仅初始化为 0，**无业务逻辑递增**）。
  - `backend/src/services/failover_service.rs:293-313` `test_switch` 是手动切换，无自动触发逻辑。
  - `backend/src/handlers/failover_handler.rs:101-111` 注释明确指出"v11 P1-6 修复（批次 143）：原 `FailoverConfig` 加载逻辑已删除，因 **failover 执行器在 v8 已删除**，配置层从未被业务读取"。
  - **没有定时健康检查任务（5s 间隔）**，没有"连续 3 次失败触发故障转移"逻辑。
- **业务影响**：数据库/缓存故障时无法自动切换到备用，需运维手动调 `POST /api/v1/erp/admin/failover/test/switch`，故障恢复时间（RTO）大幅拉长，可能超过 SLA。计划要求"必须有健康检查（5s 间隔），连续 3 次失败触发故障转移"。
- **修复建议**：
  1. 实现 `FailoverMonitor` 后台任务，每 5s 调用 `health_check`，连续 3 次失败时自动调用 `test_switch`
  2. 在 `main.rs` 启动该任务，可配置开关 `FAILOVER_AUTO_SWITCH_ENABLED`

**缺陷 20.4-B：主备切换自动完成（10s 内）缺失**
- **风险等级：P0**
- **证据**：
  - `failover_service.rs:293-313` `test_switch` 仅更新 status 表 + 记录 event，**无实际业务调用切换**（不切换 DB 连接到备用，不切换缓存到备用）。
  - `failover_handler.rs:101-111` 注释明确指出"failover 执行器在 v8 已删除"。
  - 没有在 `MetricsService` / `AppState` 层切换数据库连接的逻辑。
- **业务影响**：即使手动调用 `test_switch`，也只是把 status 表标记为 "backup"，业务代码仍使用原数据库连接，**故障切换无效**。计划要求"切换必须自动（10s 内完成），切换记录可追溯，禁止脑裂"。
- **修复建议**：
  1. 重新设计 `FailoverExecutor`：维护 primary/backup 两个 DatabaseConnection，切换时通过 `ArcSwap<DatabaseConnection>` 原子替换
  2. 业务层通过 `state.db.get_current()` 获取当前活跃连接
  3. 切换过程加分布式锁（Redis）防止脑裂

**缺陷 20.4-C：数据同步一致性（PostgreSQL 流复制）缺失**
- **风险等级：P1**
- **证据**：Grep `streaming.*replication|流复制` 全工程无命中。
  - 没有主备 PostgreSQL 流复制配置
  - 没有 failover 后数据同步校验逻辑
- **业务影响**：故障切换后备用数据库数据可能落后于主库（未同步的事务丢失），违反 RPO=0 要求。计划要求"主备数据必须实时同步（如 PostgreSQL 流复制），切换后无数据丢失"。
- **修复建议**：
  1. 部署层面配置 PostgreSQL 流复制（主库 + 备库 + repmgr 自动故障切换）
  2. 应用层 failover 切换前等待备库 catch up（`pg_stat_replication` 检查 sync_state）

**缺陷 20.4-D：故障回切机制（人工确认）缺失**
- **风险等级：P2**
- **证据**：
  - `failover_service.rs` 无 `failback` / `switch_to_primary` 方法
  - `failover_handler.rs` 无回切端点
  - 计划要求"故障恢复后必须人工确认回切（不能自动回切，防止抖动）"
- **业务影响**：故障恢复后无法回切到主库，业务长期跑在备用资源上（性能可能更差）。
- **修复建议**：
  1. 新增 `POST /api/v1/erp/admin/failover/failback` 端点（仅 admin）
  2. 实现前置校验：检查原主库健康（连续 5 次健康检查通过）+ 数据同步完成
  3. 人工确认后执行回切，记录 event_type=switch_to_primary

---

## 维度 20.5：慢查询阈值告警优化审计

### 检查方法
- Read `backend/src/middleware/slow_query.rs`
- Read `backend/src/services/slow_query_collector.rs`
- Read `backend/src/handlers/slow_query_handler.rs`
- Read `backend/src/models/slow_query.rs`
- Grep `slow.*query.*alert|慢查询.*告警|jira|工单|优化.*任务|周报|weekly.*report`

### 发现

#### ✅ 已落实的项
1. **慢查询阈值可配置**：`backend/src/middleware/slow_query.rs:30-46` `SLOW_QUERY_THRESHOLD_MS` 通过 `BINGXI_SLOW_QUERY_MS` 环境变量可配置（LazyLock + 启动时打印当前阈值，L-38 修复消除 silent default）。
2. **慢查询记录器（业务层 RAII）**：`slow_query.rs:53-88` `SlowQueryRecorder` 提供 `start(label, metrics)` + `finish()` API，业务层在 SQL 前后调用，超阈值时记录 `tracing::warn!` + Prometheus 指标。
3. **慢查询后台采集服务**：`backend/src/services/slow_query_collector.rs:31-203` `SlowQueryCollector`：
   - 通过 `pg_stat_statements` 视图查询 `mean_exec_time > threshold_ms` 的 SQL
   - 使用参数化查询（L2 修复，`build_query_sql` 使用 `$1/$2` 占位符，line 47-54）
   - 写入 `slow_query_log` 表
   - panic 隔离（AssertUnwindSafe + catch_unwind，line 91-130）
   - 单条失败不阻断后续插入（line 187-194）
4. **慢查询列表 + 统计接口**：`backend/src/handlers/slow_query_handler.rs`：
   - `GET /api/v1/erp/slow-queries`：分页 + 多维筛选（时间范围 / 最小执行时间 / 关键词），LIKE 模式注入防护（safe_like_pattern，line 113）
   - `GET /api/v1/erp/slow-queries/stats`：TOP 10 聚合统计（按 query_text 分组，max/sum/avg/count）
   - `POST /api/v1/erp/slow-queries/refresh`：手动触发一次采集
5. **后台采集任务启动**：`backend/src/main.rs:520-541` 按 `settings.slow_query.enabled` 开关启动，默认间隔 5 分钟，可配置阈值/limit_rows/interval_secs。
6. **Prometheus 慢查询指标**：`backend/src/services/metrics_service.rs:240-253` `record_slow_query` 记录耗时到 `db_query_duration_seconds` 直方图 + `tracing::warn!` 日志（含 SQL/耗时/阈值）。
7. **Prometheus 慢查询告警规则**：`monitoring/prometheus/alert_rules.yml:68-77` `SlowDatabaseQuery` 规则：`histogram_quantile(0.95, db_query_duration_seconds_bucket) > 1` 持续 5m 触发 warning 告警。

#### ❌ 缺陷项

**缺陷 20.5-A：慢查询阈值默认 100ms 与计划要求 500ms 不一致**
- **风险等级：P3**
- **证据**：
  - `backend/src/middleware/slow_query.rs:34` `unwrap_or(100)` 默认 100ms
  - `backend/src/services/slow_query_collector.rs:63` 默认 100ms（注释 line 62 "默认 100ms（与 plan 一致）"）
  - `backend/src/handlers/slow_query_handler.rs:224` 手动刷新时硬编码 `100.0`
  - 计划要求"慢查询阈值必须可配置（默认 500ms），按业务场景差异化"
- **业务影响**：100ms 阈值过于敏感，正常 OLTP 查询（如 JOIN 大表）也可能被标记为慢查询，产生大量噪音；但 500ms 又可能漏掉真正的慢查询。具体阈值需根据业务场景评估。
- **修复建议**：保留 100ms 默认（更严格的监控有助于早期发现性能问题），但在文档中明确说明与计划要求的差异，并按业务场景差异化（如报表查询阈值 2s，OLTP 查询阈值 200ms）。

**缺陷 20.5-B：慢查询自动告警（含 SQL/耗时/调用方 + 每小时聚合去重）缺失**
- **风险等级：P2**
- **证据**：
  - `backend/src/middleware/slow_query.rs:73-87` `finish()` 仅 `tracing::warn!` 记录，**不发送告警通知**
  - `backend/src/services/slow_query_collector.rs:142-202` `collect_once` 仅写入数据库表，**无告警逻辑**
  - Grep `slow.*query.*alert|慢查询.*告警` 在源代码中无命中（仅在 prometheus alert_rules.yml 有规则）
  - 没有每小时聚合去重逻辑
- **业务影响**：慢查询仅在 Prometheus 告警规则层面告警（P95 > 1s 持续 5m），**单条慢查询不会触发告警**；同一慢查询每小时可能触发多次告警（无去重）。计划要求"慢查询超阈值必须自动告警（含 SQL/耗时/调用方），每小时聚合去重"。
- **修复建议**：
  1. 在 `SlowQueryRecorder::finish()` 中，超阈值时调用 `AlertService.send_slow_query_alert(sql, elapsed, label)`
  2. `AlertService` 使用 Redis 维护 1 小时去重窗口（key=`slow_query:alert:{hash(sql)}`，TTL 3600s）
  3. 告警内容含 SQL/耗时/调用方（label）/阈值

**缺陷 20.5-C：慢查询优化任务追踪（Jira/工单）缺失**
- **风险等级：P2**
- **证据**：Grep `jira|工单|优化.*任务|optimization.*task` 全工程无命中。
- **业务影响**：慢查询发现后无工单跟踪优化进度，可能被遗忘；无法衡量优化效果。
- **修复建议**：
  1. 在 `slow_query_log` 表新增 `optimization_status`（pending/in_progress/resolved/wont_fix）+ `assigned_to` + `jira_ticket` 字段
  2. 提供 `POST /api/v1/erp/slow-queries/:id/optimization` 端点创建优化任务
  3. 集成 Jira API 自动创建工单（可选）

**缺陷 20.5-D：慢查询周报（TOP 10/趋势/优化进展）缺失**
- **风险等级：P3**
- **证据**：
  - `backend/src/handlers/slow_query_handler.rs:144-212` `get_slow_query_stats` 仅返回当前 TOP 10，**无周报生成**
  - Grep `周报|weekly.*report` 全工程无命中
- **业务影响**：运维需手动调用 stats 接口拼接周报，效率低；无法导出。
- **修复建议**：
  1. 新增 `GET /api/v1/erp/slow-queries/weekly-report` 端点
  2. 返回结构：本周 TOP 10 / 上周 TOP 10 对比趋势 / 本周优化进展（已解决/进行中/待处理）
  3. 支持 CSV/PDF 导出

---

## 维度 20.6：API 网关路由转发限流熔断审计

### 检查方法
- Read `backend/src/handlers/api_gateway_handler.rs`
- Read `backend/src/middleware/rate_limit.rs`
- Grep `circuit.*break|熔断|dynamic.*route|动态.*路由|gateway.*auth|网关.*鉴权`

### 发现

#### ✅ 已落实的项
1. **API 端点 CRUD（注册元数据）**：`backend/src/handlers/api_gateway_handler.rs:166-357` 提供完整端点 CRUD：
   - `GET /api-gateway/endpoints` 分页 + 关键词 + 状态 + method 过滤
   - `POST /api-gateway/endpoints` 创建（含唯一性检查 + UK 约束兜底，line 239-278）
   - `PUT /api-gateway/endpoints/:id` 更新
   - `DELETE /api-gateway/endpoints/:id` 删除
2. **API 调用日志查询**：`api_gateway_handler.rs:362-417` 复用 `log_api_accesses` 表，支持分页 + 关键词 + method + status 过滤（2xx/4xx/5xx 区间过滤）。
3. **API 密钥管理**：`api_gateway_handler.rs:482-679` 完整密钥 CRUD + regenerate：
   - 创建含权限序列化失败保护（批次 407 修复，line 533-539）
   - 创建/更新注入真实 user_id（批次 112 P1-9，line 560,665）
   - 撤销时同步清理缓存（line 645）
4. **API 网关统计接口**：`api_gateway_handler.rs:422-473` `get_api_stats` 返回 total_endpoints/active_endpoints/total_keys/active_keys/total_requests/total_errors/avg_response_time_ms。
5. **限流中间件（按 IP + UserID 双维度）**：`backend/src/middleware/rate_limit.rs:259-304` `rate_limit_by_ip`：
   - 180 req/min/user（line 114）
   - Redis 分布式限流优先 + 内存回退（漏洞 #6 修复，line 231-252）
   - 超限返回 429 + `retry_after: 60`（line 297-300）
   - IP 提取三级降级（X-Real-IP / X-Forwarded-For / ConnectInfo，失败返回 400）
6. **防暴力攻击限流**：`rate_limit.rs:308-341` `anti_brute_force` 针对登录端点，5 次/5 分钟，IP + Username 双维度。
7. **限流配置可观测**：`rate_limit.rs:148-160` 未配置 Redis 时生产环境 warn / 开发环境 info（L-42 修复消除 silent default）。
8. **限流中间件安全加固**：`rate_limit.rs:54-88` 使用 `try_lock` 防御 PoisonError，锁中毒时 fail-open 不 panic（低危 #3 修复）。

#### ❌ 缺陷项

**缺陷 20.6-A：API 网关动态路由（不重启服务）缺失**
- **风险等级：P2**
- **证据**：
  - `backend/src/handlers/api_gateway_handler.rs:166-357` 仅对 `api_endpoints` 表做 CRUD，**不实际影响运行时路由**
  - 路由注册在 `backend/src/routes/*.rs` 静态定义，启动时通过 `Router::new().route(...)` 注册，**运行时无法动态加载/卸载**
  - Grep `dynamic.*route|动态.*路由` 全工程无命中
- **业务影响**：新增/下线 API 需修改代码 + 重启服务，无法快速响应业务变化。计划要求"路由必须可配置（动态加载），路由变更不重启服务"。
- **修复建议**：
  1. 实现 `DynamicRouter` 中间件，根据 `api_endpoints` 表的 status 字段（active/inactive）动态放行/拒绝
  2. 配合路由分组（如 `/api/v1/erp/legacy/*` 标记 deprecated）
  3. 短期方案：通过 nginx 配置实现路由动态加载（OpenResty + lua）

**缺陷 20.6-B：API 网关熔断（5s 内失败率 > 50% 触发）缺失**
- **风险等级：P1**
- **证据**：
  - Grep `circuit.*break|熔断` 在 `api_gateway_handler.rs` 和 `middleware/` 下无命中（仅在 `failover_service.rs` 的 `circuit_state` 字段有概念，但 failover 执行器已删除）
  - 没有 `CircuitBreaker` 中间件
  - 没有下游服务故障时的熔断逻辑
- **业务影响**：下游服务（如 AI 服务 / 第三方 API）故障时，本服务会持续发起请求 + 等待超时，导致线程池耗尽 + 级联故障。计划要求"下游服务故障必须熔断（5s 内失败率 > 50% 触发），熔断后快速失败"。
- **修复建议**：
  1. 引入 `tower::limit::ConcurrencyLimit` + 自定义 `CircuitBreakerLayer`
  2. 维护滑动窗口（5s），统计失败率，> 50% 时进入 open 状态（直接返回 503）
  3. 30s 后进入 half-open 状态，放行 1 个探测请求，成功则 close，失败则继续 open

**缺陷 20.6-C：API 网关统一鉴权（JWT 校验 + 下游服务信任网关身份）缺失**
- **风险等级：P2**
- **证据**：
  - 业务系统直接通过 `auth_middleware` 鉴权（每个服务都校验 JWT），**无网关层统一鉴权**
  - `api_gateway_handler.rs` 中的 endpoints CRUD 仅元数据管理，不实际执行鉴权
  - 计划要求"网关必须统一鉴权（JWT 校验），下游服务信任网关身份"
- **业务影响**：每个微服务都重复实现 JWT 校验逻辑，增加维护成本；若某服务遗漏鉴权中间件，存在安全风险。
- **修复建议**：
  1. 部署独立 API 网关（如 Kong / APISIX / 自研 axum 网关）
  2. 网关层校验 JWT 后，注入 `X-User-Id` / `X-Role-Id` header 转发给下游
  3. 下游服务配置信任网关 IP 白名单 + 校验 header

**缺陷 20.6-D：限流 Retry-After 头返回但单位为秒，与 RFC 7231 一致性需确认**
- **风险等级：P3**
- **证据**：`backend/src/middleware/rate_limit.rs:297-300`
  ```rust
  return Err(AppError::TooManyRequests {
      retry_after: Some(60),   // ← 单位为秒
      message: "请求过于频繁".to_string(),
  });
  ```
  RFC 7231 §7.1.3 规定 `Retry-After` 可为秒数或 HTTP-date，需确认 `AppError::TooManyRequests` 序列化时是否正确设置 `Retry-After: 60` header。
- **业务影响**：若未正确序列化 header，客户端无法知道何时重试。
- **修复建议**：审计 `AppError::TooManyRequests` 的 `IntoResponse` 实现，确认 `Retry-After` header 正确设置（如已实现可降为 P3 文档问题）。

---

## 维度 20.7：系统版本与升级管理审计

### 检查方法
- Read `backend/src/models/system_version.rs`
- Read `backend/src/services/system_update_service.rs`
- Read `backend/src/handlers/system_update_handler.rs`（前 100 行）
- Read `backend/src/cli/util/backup.rs`
- Glob `backend/migrations/*/down.sql`（验证回滚脚本配套）
- Grep `canary|灰度|rollback.*script|回滚脚本|deprecat`

### 发现

#### ✅ 已落实的项
1. **版本号管理（语义化版本）**：`backend/src/services/system_update_service.rs:783-785` `parse_version(v: &str) -> Vec<u32>` 解析 major.minor.patch，支持预发布标签（"1.0.0-beta" → [1, 0, 0]）；`compare_versions`（line 632-650）正确比较版本大小。
2. **VERSION 文件持久化**：`system_update_service.rs:224-231` `get_current_version` 从 `app_dir/VERSION` 文件读取当前版本，缺失时回退 "1.0.0"。
3. **升级流程完整**：`system_update_service.rs:300-346` `do_update` 5 步流程：
   - 步骤 1：创建备份（备份 backend/frontend/config + VERSION 文件，line 348-374）
   - 步骤 2：解压更新包（zip 解压 + 路径穿越校验 + 权限掩码，line 395-412）
   - 步骤 3：验证更新包（VERSION + UPDATE_MANIFEST.json 双文件校验，line 425-441）
   - 步骤 4：应用更新（旧目录 rename 为 .old + 新目录复制，line 450-483）
   - 步骤 5：验证更新结果（backend_exe 或 VERSION 文件存在，line 485-498）
4. **回滚机制**：`system_update_service.rs:501-538` `rollback` 备份恢复 + `rollback_to_version` 按版本号回滚：
   - 失败时自动回滚（line 319-329）
   - 验证失败时回滚（line 333-339）
   - 支持手动回滚到任意历史版本（line 528-538）
5. **回滚脚本配套**：`backend/migrations/` 所有 80 个迁移都有 `up.sql` + `down.sql`，命名格式 `YYYYMMDDHHMMSS_description` 严格递增。
6. **并发更新保护**：`system_update_service.rs:284-298` `apply_update` 使用 `AtomicBool::swap` 防止并发更新（`AlreadyUpdating` 错误）。
7. **备份自动清理**：`system_update_service.rs:540-550` `cleanup_old_backups` 保留最近 3 个备份，旧备份自动删除。
8. **升级日志**：`system_update_service.rs:552-567` `log_update` 写入 `update.log`，含时间戳。
9. **GitHub Release 集成**：`system_update_service.rs:569-630` `check_for_updates` + `fetch_latest_release` 通过 GitHub API 拉取最新 release，含 SSRF 防护（DNS Rebinding 防御 + 域名白名单 + HTTPS 强制）。
10. **下载安全加固**：
    - `validate_download_url`（line 867-889）：仅允许 `github.com` / `objects.githubusercontent.com`，强制 HTTPS
    - `validate_asset_name`（line 894-917）：拒绝路径穿越 + 特殊字符
    - `extract_zip_entry`（line 799-842）：`enclosed_name` + `starts_with` 双重 Tar Slip 防护 + 权限掩码（文件 0o600 / 目录 0o755）
11. **升级端点 admin 角色校验**：`backend/src/handlers/system_update_handler.rs:24-44` `require_admin_role` 在 handler 层显式校验 admin 角色（P0 7-2 修复，深度防御）。
12. **备份与恢复 CLI 命令**：`backend/src/cli/util/backup.rs`：
    - `cmd_backup`：pg_dump + 配置文件备份 + tar 压缩 + 0o600 权限
    - `cmd_restore`：tar 内容校验 + 路径穿越防护 + psql 恢复 + 配置文件恢复
    - 临时目录使用 UUID 防止符号链接竞争（TOCTOU）
13. **本地 release 管理**：`system_update_service.rs:134-170` `list_local_releases` 扫描 `releases/bingxi-erp-*.zip` 文件，按版本号倒序排序。

#### ❌ 缺陷项

**缺陷 20.7-A：灰度升级（10% → 50% → 100%）缺失**
- **风险等级：P1**
- **证据**：
  - `backend/src/services/system_update_service.rs:300-346` `do_update` 是全量替换，**无灰度机制**
  - Grep `canary|灰度` 全工程无命中
  - 计划要求"升级必须走灰度（先 10% → 50% → 100%），支持回滚"
- **业务影响**：升级故障直接影响 100% 用户，无法在小范围验证后再全量。
- **修复建议**：
  1. 部署层面使用 Kubernetes 滚动更新（`maxSurge: 10%` + `maxUnavailable: 0`）
  2. 应用层支持双版本共存（如 v1 路由 `/api/v1/*`，v2 路由 `/api/v2/*`），通过 nginx upstream 权重灰度切流
  3. 灰度期间监控关键指标（错误率 / 延迟），异常自动回滚

**缺陷 20.7-B：API 向后兼容性 / deprecation 标注缺失**
- **风险等级：P2**
- **证据**：
  - `backend/src/models/api_endpoint.rs` 的 `version` 字段可标注 API 版本，但**无 deprecation 机制**
  - Grep `deprecat` 在源代码中无命中（仅在第三方依赖中有）
  - 没有路由层强制 `/api/v1/*` 与 `/api/v2/*` 共存策略
- **业务影响**：API 升级时旧客户端可能因字段/行为变化而崩溃，无法平滑迁移。计划要求"升级必须向后兼容（旧 API 至少保留 1 个版本），废弃 API 标注 deprecation"。
- **修复建议**：
  1. 在 `api_endpoints` 表新增 `deprecated_at` + `sunset_at` 字段
  2. 响应头添加 `Deprecation: true` + `Sunset: <date>`（RFC 8594）
  3. 文档中明确每个 API 的支持周期

**缺陷 20.7-C：迁移跳跃检测缺失**
- **风险等级：P3**
- **证据**：
  - `backend/migration/src/lib.rs` 使用 sea-orm-migration，按文件名时间戳顺序应用
  - 没有"禁止跳跃"校验（如 m0001 → m0003 跳过 m0002 时是否报错）
- **业务影响**：迁移被人为跳过时，数据库 schema 可能不一致，运行时出现奇怪错误。
- **修复建议**：sea-orm-migration 默认按顺序应用，但可增加启动时校验 `migration` 表的 last_applied 与最新 mXXX 之间是否有 gap。

**缺陷 20.7-D：`system_version` 表 model 未接入业务**
- **风险等级：P3**
- **证据**：
  - `backend/src/models/system_version.rs:1-2` 标注 `#![allow(dead_code)]` + TODO tech-debt
  - Grep `system_version::Entity` 在源代码中无业务调用
  - `system_update_service.rs:224-231` 通过 VERSION 文件读版本，未使用 `system_version` 表
- **业务影响**：版本变更历史未持久化到数据库，无法在管理后台查看版本变更记录。
- **修复建议**：在 `apply_update` 成功后，向 `system_version` 表插入新版本记录（version/release_date/changelog/is_current=true，旧版本 is_current=false）。

---

## 维度 20.8：日志增强与系统日志完整性审计

### 检查方法
- Read `backend/src/utils/log_config.rs`
- Read `backend/src/services/enhanced_logger.rs`
- Read `backend/src/services/audit_cleanup_service.rs`
- Read `backend/src/middleware/omni_audit.rs`（脱敏部分）
- Read `backend/src/middleware/auth.rs`（脱敏部分，grep 验证）
- Grep `tracing_subscriber|EnvFilter|fmt\(\)\.json|with_max_level|json\(\)` 日志格式

### 发现

#### ✅ 已落实的项
1. **日志分级（DEBUG/INFO/WARN/ERROR）**：`backend/src/utils/log_config.rs:24-27,147-149` 通过 `tracing_subscriber::EnvFilter` 支持 RUST_LOG 环境变量配置日志级别，默认 `bingxi_backend={log_level},tower_http=debug`。
2. **日志按 target 分层**：`log_config.rs:81-160` 配置 10 个独立 layer：
   - main_layer（主日志）/ financial_layer（资金审计）/ permission_layer（权限审计）/ security_layer（安全事件）/ database_layer（数据库审计）/ performance_layer（性能监控）/ business_layer（业务审计）/ health_layer（系统健康）/ error_layer（错误日志）/ console_layer（开发控制台）
   - 按目录分文件：`/audit/` `/security/` `/performance/`
3. **日志按天归档**：`log_config.rs:48-79` 使用 `tracing_appender::rolling::RollingFileAppender` + `Rotation::DAILY`，每日轮转。
4. **容器环境适配**：`log_config.rs:13-33` 检测 `/.dockerenv` 或 `KUBERNETES_SERVICE_HOST`，容器环境使用 stdout 输出（适合 ELK/Loki 采集）。
5. **审计日志保留策略**：`backend/src/services/audit_cleanup_service.rs:1-91` `AuditCleanupService`：
   - 每天定时清理（`start_cleanup_task` 24h 间隔，line 22）
   - 默认保留 365 天（main.rs:489-508，环境变量 `AUDIT_RETENTION_DAYS` 可配置）
   - 参数化查询防 SQL 注入（批次 94 P2-1 修复，line 53-57）
   - panic 隔离（AssertUnwindSafe + catch_unwind，line 28-45）
6. **审计日志敏感路径请求体脱敏**：`backend/src/middleware/omni_audit.rs:107-118,301-320` P1 7-5 修复：
   - `is_sensitive_request_body_path` 匹配 `/auth/change-password` / `/auth/reset-password` / `/users/change-password` / `/auth/setup-totp` 等敏感路径
   - 命中时 request_body 脱敏为 `"[REDACTED]"`
7. **Authorization 头日志脱敏**：`backend/src/middleware/auth.rs:17-39` `mask_authorization` 截断 token，仅显示前缀与长度（如 `Bearer abc***(len=143)`），不输出完整 token。
8. **用户名 PII 脱敏**：`backend/src/middleware/auth.rs:41-52` `mask_username` 按字符截断（如 `al***`），支持中文字符（按字符而非字节截断）。
9. **错误响应脱敏**：`backend/src/utils/error.rs:85-95,400-500` 生产环境（`APP_ENV=production`）返回通用脱敏文案，不暴露 Display 完整内容（漏洞 #11/#4 修复）。
10. **业务字段脱敏**：`backend/src/utils/field_mask.rs:4-16` 提供 `mask_sensitive_fields` 工具，按角色脱敏成本价/敏感金额（role_id != 1 时脱敏）。
11. **登录安全日志增强**：`backend/src/services/enhanced_logger.rs:1-119` `EnhancedLogger::log_login_security`：
    - 按 risk_level（LOW/MEDIUM/HIGH/CRITICAL）选择日志级别（DEBUG/INFO/WARN）
    - 详细日志结构化 JSON 输出（serde_json::to_string）
    - 含 device_info / geo_info / security_info / failure_info 4 个维度
12. **审计日志统计接口**：`audit_cleanup_service.rs:94-125` `get_stats` 返回 total_omni_logs/total_audit_logs/today_omni_logs/today_audit_logs/oldest_log/newest_log。
13. **响应体按 Unicode 字符截断**：`middleware/omni_audit.rs:122-127,209-214,245-249` P2 7-12 修复：原 `&body[..5000]` 按字节切片会 panic，改为 `chars().take(5000).collect()` 按 Unicode 字符截断。
14. **审计日志跳过敏感路径**：`middleware/omni_audit.rs:218-228` 跳过 PUBLIC_PATHS（登录/刷新含密码字段）+ /metrics /health /swagger-ui /api-docs /static，避免敏感信息泄露与无意义审计。

#### ❌ 缺陷项

**缺陷 20.8-A：日志未结构化（JSON 格式）**
- **风险等级：P1**
- **证据**：
  - `backend/src/utils/log_config.rs:18-29,82-160` 所有 `tracing_subscriber::fmt::layer()` **均未调用 `.json()`**
  - Grep `fmt\(\)\.json` 全工程无命中
  - 使用默认 plain text 格式，含 ANSI 颜色码（`with_ansi(false)` 仅在文件层关闭，console_layer 保留 `with_ansi(true)`）
- **业务影响**：
  1. 日志解析依赖正则，ELK/Loki Grok pattern 复杂且易错
  2. 结构化字段（如 trace_id/user_id/duration_ms）无法直接索引
  3. 计划要求"日志必须结构化（JSON 格式），支持 ELK/Loki 检索"
- **修复建议**：
  1. 文件 layer 调用 `.json()` 启用 JSON 格式
  2. 容器环境 stdout 也使用 JSON 格式（便于 Loki 直接采集）
  3. console_layer 保留 plain text（开发体验）

**缺陷 20.8-B：日志保留 90 天 / 自动清理缺失**
- **风险等级：P1**
- **证据**：
  - `backend/src/utils/log_config.rs:48-79` 使用 `RollingFileAppender::DAILY`，**tracing_appender 不会自动清理过期日志文件**
  - Grep `log.*retention|log.*archive|log.*90|log.*cleanup|cleanup.*log|delete.*old.*log` 全工程无命中（仅 `audit_cleanup_service.rs` 清理数据库审计日志，不清理日志文件）
  - 计划要求"日志必须按天归档，保留 90 天，超期自动清理"
- **业务影响**：日志文件无限增长，磁盘空间耗尽风险；过期日志未自动清理，运维需手动处理。
- **修复建议**：
  1. 启动后台任务每天扫描 log_dir，删除修改时间 > 90 天的 `*.log` 文件
  2. 或使用 `tracing_appender::non_blocking::NonBlocking` + 自定义 `DeleteOldFiles` 滚动策略
  3. 配置可环境变量化：`LOG_RETENTION_DAYS=90`

**缺陷 20.8-C：日志中手机号/身份证/密码统一脱敏机制缺失**
- **风险等级：P2**
- **证据**：
  - Grep `mask.*phone|脱敏|mask.*password|mask.*id_card|desensitiz|mask.*mobile` 在 `utils/log_config.rs` 和 `enhanced_logger.rs` 无命中（仅 omni_audit/auth/field_mask 局部脱敏）
  - 业务日志（如 `tracing::info!("用户手机号: {}", phone)`）可能直接输出原始手机号
  - 没有统一的 `tracing_subscriber::layer::Filter` 或 `tracing::field::Visit` 实现自动脱敏
- **业务影响**：手机号/身份证等 PII 数据可能被记录到日志文件，违反《个人信息保护法》/ GDPR。
- **修复建议**：
  1. 实现 `MaskingLayer` 包装 `tracing_subscriber::Layer`，对 message 中的手机号（1[3-9]\d{9}）/身份证（\d{17}[\dXx]）/邮箱自动 mask
  2. 或在 `enhanced_logger.rs` 中提供 `mask_pii(s: &str) -> String` 工具函数，业务侧调用

**缺陷 20.8-D：日志归档（冷数据迁移）缺失**
- **风险等级：P3**
- **证据**：
  - `backend/src/main.rs:484-486` 注释明确"归档逻辑（1-3 年冷数据迁移）作为后续技术债单独实现"
  - 计划要求"trace 明细数据保留 7 天，超期归档为统计数据"（同样适用于日志）
- **业务影响**：长期日志全部保留在热存储（本地磁盘），存储成本高；冷数据归档未实现。
- **修复建议**：
  1. 实现 `LogArchiveService`，定期把 > 30 天的日志 tar.gz 归档到对象存储（OSS/S3）
  2. 归档后在本地保留索引（文件名 + 时间范围）便于按需下载查询

---

## 审计结果汇总

| 维度 | P0 | P1 | P2 | P3 | 已落实 | 总检查项 |
|------|----|----|----|----|--------|----------|
| 20.1 trace 链路 | 0 | 2 | 2 | 0 | 7 | 11 |
| 20.2 metrics 与告警 | 0 | 0 | 0 | 4 | 14 | 18 |
| 20.3 WebSocket | 0 | 2 | 1 | 0 | 8 | 11 |
| 20.4 failover | 2 | 1 | 1 | 0 | 9 | 13 |
| 20.5 慢查询 | 0 | 0 | 2 | 2 | 7 | 11 |
| 20.6 API 网关 | 0 | 1 | 2 | 1 | 8 | 12 |
| 20.7 系统版本 | 0 | 1 | 1 | 2 | 13 | 17 |
| 20.8 日志增强 | 0 | 2 | 1 | 1 | 14 | 18 |
| **合计** | **2** | **9** | **10** | **10** | **80** | **111** |

## 修复优先级队列

### P0（阻塞，必须立即修复）

1. **缺陷 20.4-A：自动故障检测机制缺失（5s 间隔 / 连续 3 次失败触发）**
   - 文件：`backend/src/services/failover_service.rs`
   - 修复：实现 `FailoverMonitor` 后台任务，每 5s 调用 `health_check`，连续 3 次失败自动调用 `test_switch`

2. **缺陷 20.4-B：主备切换自动完成（10s 内）缺失**
   - 文件：`backend/src/services/failover_service.rs` + `backend/src/utils/app_state.rs`
   - 修复：重新设计 `FailoverExecutor`，使用 `ArcSwap<DatabaseConnection>` 原子切换，业务层通过 `state.db.get_current()` 获取当前活跃连接

### P1（高，本迭代内修复）

3. **缺陷 20.1-A：trace 跨服务（HTTP 下游调用）传递缺失**
   - 文件：`backend/src/observability/trace_context.rs`（新增 `inject` 函数）+ 业务侧 reqwest 调用点
   - 修复：实现 `inject_trace_context(headers, ctx)` 工具函数，所有 reqwest 调用前注入 `traceparent`

4. **缺陷 20.1-B：trace 跨事件总线（Kafka）传递缺失**
   - 文件：`backend/src/services/event_kafka.rs:222-227`
   - 修复：在 `publish` 中从当前 span 提取 trace_id/span_id，构造 traceparent 放入 headers；`subscribe` 中解析 header 创建 child span

5. **缺陷 20.3-A：WebSocket 消息 ACK 机制缺失**
   - 文件：`backend/src/websocket/notifications.rs`
   - 修复：`WsMessage` 新增 Ack 变体，服务端记录 message_id + 5s 超时定时器，未确认重发（最多 3 次）

6. **缺陷 20.3-B：WebSocket 多实例广播（Redis Pub/Sub）缺失**
   - 文件：`backend/src/websocket/notifications.rs`
   - 修复：`broadcast_notification` 中本地广播 + Redis publish；每实例 subscribe Redis channel，收到消息后本地广播

7. **缺陷 20.4-C：数据同步一致性（PostgreSQL 流复制）缺失**
   - 部署层面 + 应用层
   - 修复：部署 PostgreSQL 流复制（主库 + 备库 + repmgr），应用层 failover 切换前等待备库 catch up

8. **缺陷 20.6-B：API 网关熔断（5s 内失败率 > 50% 触发）缺失**
   - 文件：新增 `backend/src/middleware/circuit_breaker.rs`
   - 修复：实现 `CircuitBreakerLayer`（滑动窗口 5s，失败率 > 50% 进入 open 状态，30s 后 half-open 探测）

9. **缺陷 20.7-A：灰度升级（10% → 50% → 100%）缺失**
   - 部署层面 + 应用层
   - 修复：Kubernetes 滚动更新（maxSurge 10%）+ nginx upstream 权重灰度切流 + 监控自动回滚

10. **缺陷 20.8-A：日志未结构化（JSON 格式）**
    - 文件：`backend/src/utils/log_config.rs`
    - 修复：文件 layer 调用 `.json()` 启用 JSON 格式，容器环境 stdout 也用 JSON

11. **缺陷 20.8-B：日志保留 90 天 / 自动清理缺失**
    - 文件：新增 `backend/src/services/log_cleanup_service.rs`
    - 修复：后台任务每天扫描 log_dir，删除修改时间 > 90 天的 `*.log` 文件

### P2（中，下迭代修复）

12. **缺陷 20.1-C：trace 采样策略不符合生产最佳实践**（默认 100% → 10% + tail-based sampling）
13. **缺陷 20.1-D：trace 数据保留期未配置**（Jaeger/Tempo 配置 7 天 retention）
14. **缺陷 20.3-C：WebSocket 服务端主动心跳与超时断开缺失**（30s ping + 60s 超时）
15. **缺陷 20.4-D：故障回切机制（人工确认）缺失**（新增 failback 端点）
16. **缺陷 20.5-B：慢查询自动告警（含 SQL/耗时/调用方 + 每小时聚合去重）缺失**
17. **缺陷 20.5-C：慢查询优化任务追踪（Jira/工单）缺失**
18. **缺陷 20.6-A：API 网关动态路由（不重启服务）缺失**
19. **缺陷 20.6-C：API 网关统一鉴权（JWT 校验 + 下游服务信任网关身份）缺失**
20. **缺陷 20.7-B：API 向后兼容性 / deprecation 标注缺失**
21. **缺陷 20.8-C：日志中手机号/身份证/密码统一脱敏机制缺失**

### P3（低，技术债清单）

22. **缺陷 20.2-A：网络指标（network IO）未采集**
23. **缺陷 20.2-B：告警通知渠道仅邮箱，企业微信/钉钉未启用**
24. **缺陷 20.2-C：Alertmanager 目标未在 prometheus.yml 中启用**
25. **缺陷 20.2-D：看板未包含系统资源（CPU/内存/磁盘/网络）面板**
26. **缺陷 20.5-A：慢查询阈值默认 100ms 与计划要求 500ms 不一致**
27. **缺陷 20.5-D：慢查询周报（TOP 10/趋势/优化进展）缺失**
28. **缺陷 20.6-D：限流 Retry-After 头返回但单位为秒，与 RFC 7231 一致性需确认**
29. **缺陷 20.7-C：迁移跳跃检测缺失**
30. **缺陷 20.7-D：`system_version` 表 model 未接入业务**
31. **缺陷 20.8-D：日志归档（冷数据迁移）缺失**

---

## 关键发现总结

1. **可观测性基础设施完善**：W3C Trace Context 完整实现 + Prometheus 指标 7 类基础 + 4 类带标签 + 20+ 业务指标 + Grafana 看板 + Alertmanager 4 级通知 + 16 条告警规则，**已具备生产级可观测性骨架**。

2. **failover 模块严重不完整**：failover 执行器在 v8 已删除，当前仅保留状态查询/手动切换/健康检查端点，**自动故障检测 + 自动切换 + 数据同步 + 回切机制全部缺失**，2 个 P0 缺陷需立即修复。

3. **WebSocket 实时推送安全性优秀但可靠性不足**：票据鉴权设计良好（30s 过期 + 一次性消费 + 256bit 随机量），但 **ACK 机制 + 多实例广播 + 服务端心跳** 三项关键可靠性能力缺失，多实例部署下消息会丢失。

4. **日志系统分层完善但结构化缺失**：10 个 layer 按业务域分文件，但 **全部使用 plain text 格式**（无 `.json()`），ELK/Loki 采集需复杂 Grok pattern；**日志文件 90 天自动清理缺失**，磁盘空间耗尽风险。

5. **trace 跨服务传递断裂**：HTTP 下游调用 + Kafka 消息 **均未注入 traceparent**，跨服务调用链断裂，Jaeger 中无法看到完整调用链。

6. **慢查询审计功能完整但告警/工单缺失**：阈值可配置 + 后台采集 + 列表/统计/刷新接口齐全，但 **单条慢查询不告警 + 无工单跟踪 + 无周报**，优化闭环未形成。

7. **API 网关仅元数据管理**：endpoints/keys/logs/stats CRUD 完整，但 **动态路由 + 熔断 + 统一鉴权** 三项核心网关能力缺失，当前更像"API 注册中心"而非真正的"网关"。

8. **系统升级流程工业级**：5 步升级流程 + 备份/回滚 + GitHub Release 集成 + SSRF 防护 + Tar Slip 防护 + 权限掩码 + admin 角色校验，**升级安全性极高**；但 **灰度升级 + API deprecation 机制缺失**。

9. **审计日志保留策略合理**：默认 365 天 + 环境变量可配置 + 参数化查询 + panic 隔离，符合审计日志保留最佳实践。

10. **告警配置完整但链路未打通**：Alertmanager 目标在 prometheus.yml 中被注释，告警规则触发后无处发送；企业微信/钉钉 webhook 全部被注释，仅邮件通知。
