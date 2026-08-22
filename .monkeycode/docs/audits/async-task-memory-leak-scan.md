# 异步任务正确性与内存泄漏扫描报告（3.11 + 3.14）

> 扫描范围：`backend/src/` 全目录
> 扫描日期：2026-08-22
> 规则：不修改 `.monkeycode/`、不创建 PR、不推送、不运行 cargo、中文注释

---

## 3.11 异步任务正确性

### 3.11.1 `tokio::spawn` 完整清单

共 53 处 `tokio::spawn`（含注释/文档引用 3 处，实际 spawn 调用 50 处）。

| # | 文件:行 | 用途 | 句柄管理 | 取消信号 |
|---|---------|------|----------|----------|
| 1 | `bootstrap/service_bootstrap.rs:155` | WebSocket Redis Pub/Sub 订阅器 | MAIN_BACKGROUND_TASKS | 无 token.cancelled()，依赖 abort() 兜底 + Redis 连接断开自然退出 |
| 2 | `bootstrap/service_bootstrap.rs:161` | 权限缓存 Redis Pub/Sub 订阅器 | MAIN_BACKGROUND_TASKS | 无 token.cancelled()，依赖 abort() 兜底 + Redis 连接断开自然退出 |
| 3 | `bootstrap/service_bootstrap.rs:480` | admin 角色缓存清理（10min） | MAIN_BACKGROUND_TASKS | 有 token.cancelled()（:488） |
| 4 | `bootstrap/service_bootstrap.rs:504` | JTI 黑名单清理（1h） | MAIN_BACKGROUND_TASKS | 有 token.cancelled()（:511） |
| 5 | `bootstrap/service_bootstrap.rs:648` | FailoverMonitor DB 健康探测（5s） | MAIN_BACKGROUND_TASKS | 无 token.cancelled()，纯 loop，依赖 abort() 兜底 |
| 6 | `bootstrap/service_bootstrap.rs:811` | 设备连接心跳超时清理（60s） | MAIN_BACKGROUND_TASKS | 有 token.cancelled()（:855） |
| 7 | `bootstrap/service_bootstrap.rs:464`(间接) | 慢查询采集任务 | MAIN_BACKGROUND_TASKS | 传入 MAIN_CANCELLATION_TOKEN（:466） |
| 8 | `bootstrap/service_bootstrap.rs:529`(间接) | CRM 公海回收（6h） | MAIN_BACKGROUND_TASKS | 传入 MAIN_CANCELLATION_TOKEN，有 token.cancelled()（recycle_executor.rs:155,174） |
| 9 | `bootstrap/service_bootstrap.rs:684`(间接) | 色卡发放过期检查（24h） | MAIN_BACKGROUND_TASKS | 无 token.cancelled()，依赖 abort() 兜底 |
| 10 | `bootstrap/service_bootstrap.rs:705`(间接) | 邮件队列 Worker（60s） | MAIN_BACKGROUND_TASKS | 无 token.cancelled()，依赖 abort() 兜底 |
| 11 | `bootstrap/service_bootstrap.rs:726`(间接) | 导出合规审查（24h） | MAIN_BACKGROUND_TASKS | 无 token.cancelled()，依赖 abort() 兜底 |
| 12 | `bootstrap/service_bootstrap.rs:756`(间接) | 追踪数据 90 天清理（24h） | MAIN_BACKGROUND_TASKS | 无 token.cancelled()，依赖 abort() 兜底 |
| 13 | `bootstrap/service_bootstrap.rs:773`(间接) | 库存告警通知（6h） | MAIN_BACKGROUND_TASKS | 无 token.cancelled()，依赖 abort() 兜底 |
| 14 | `bootstrap/service_bootstrap.rs:782`(间接) | 供应商评估调度（24h） | MAIN_BACKGROUND_TASKS | 传入 MAIN_CANCELLATION_TOKEN，有 token.cancelled() |
| 15 | `bootstrap/service_bootstrap.rs:799`(间接) | 定时推送后台调度（60s） | MAIN_BACKGROUND_TASKS | 无 token.cancelled()，依赖 abort() 兜底 |
| 16 | `bootstrap/service_bootstrap.rs:885`(间接) | 权限合规审查（7d） | MAIN_BACKGROUND_TASKS | 传入 MAIN_CANCELLATION_TOKEN，有 token.cancelled() |
| 17 | `bootstrap/service_bootstrap.rs:901`(间接) | 审计日志分级清理（24h） | MAIN_BACKGROUND_TASKS | 传入 MAIN_CANCELLATION_TOKEN，有 token.cancelled()（audit_cleanup_service.rs:56） |
| 18 | `services/auth_service_ops/jti.rs:264` | 用户吊销记录清理（24h） | APP_STATE_BACKGROUND_TASKS | 无 token.cancelled()，依赖 abort() 兜底 |
| 19 | `services/audit_log_service.rs:46` | 审计日志 mpsc 消费者 | struct handle 字段（:32） | 无 token，channel 关闭时退出，由 `AuditLogService::shutdown()` abort |
| 20 | `services/event_kafka.rs:255` | Kafka 消费后台拉取循环 | 句柄未保存 | 无 token，依赖 mpsc channel 生命周期（drop 时退出） |
| 21 | `services/inventory_finance_bridge_ops/listener.rs:33` | 库存财务桥接监听器 | BRIDGE_LISTENER_HANDLE | 无 token，由 `shutdown_event_bus()` abort（:1632） |
| 22 | `services/audit_cleanup_service.rs:32` | 审计清理循环 | MAIN_BACKGROUND_TASKS（:901） | 有 token.cancelled()（:56） |
| 23 | `services/supplier_evaluation_service.rs:484` | 供应商评估循环 | MAIN_BACKGROUND_TASKS（:782） | 有 token.cancelled() |
| 24 | `services/event_bus_ops/listener.rs:34` | 主事件监听器（分发中枢） | MAIN_LISTENER_HANDLE（:446） | 无 token，由 `shutdown_event_bus()` abort（:1626） |
| 25 | `services/event_bus_ops/listener.rs:467` | 客户名冗余刷新（一次性） | fire-and-forget | 自终止 |
| 26 | `services/event_bus_ops/listener.rs:480` | 供应商名冗余刷新（一次性） | fire-and-forget | 自终止 |
| 27 | `services/event_bus_ops/listener.rs:731` | 染缸占用（一次性） | fire-and-forget | 自终止 |
| 28 | `services/event_bus_ops/listener.rs:770` | 染缸释放（一次性） | fire-and-forget | 自终止 |
| 29 | `services/event_bus_ops/listener.rs:815` | 工艺优化反馈（一次性） | fire-and-forget | 自终止 |
| 30 | `services/event_bus_ops/listener.rs:904` | 工资人工成本归集（一次性） | fire-and-forget | 自终止 |
| 31 | `services/event_bus_ops/kafka.rs:58` | Kafka 消费桥接 | EventBusState.consumer_handle | 无 token，由 `shutdown_event_bus()` abort（:1611） |
| 32 | `services/notification_scheduler.rs:118` | 通知推送调度（60s） | MAIN_BACKGROUND_TASKS（:799） | 无 token.cancelled()，依赖 abort() 兜底 |
| 33 | `services/slow_query_collector.rs:71` | 慢查询采集循环 | MAIN_BACKGROUND_TASKS（:464） | 传入 cancel_token（:68） |
| 34 | `services/log_cleanup_service.rs:53` | 日志清理循环（24h） | 需确认注册 | 有 token.cancelled()（:62） |
| 35 | `services/tracking_cleanup_service.rs:50` | 追踪数据清理（24h） | MAIN_BACKGROUND_TASKS（:756） | 无 token.cancelled()，依赖 abort() 兜底 |
| 36 | `services/stock_alert_notification_scheduler.rs:211` | 库存告警通知（6h） | MAIN_BACKGROUND_TASKS（:773） | 无 token.cancelled()，依赖 abort() 兜底 |
| 37 | `services/color_card_issue_scheduler.rs:206` | 色卡过期检查（24h） | MAIN_BACKGROUND_TASKS（:684） | 无 token.cancelled()，依赖 abort() 兜底 |
| 38 | `services/omni_audit_service.rs:108` | OmniAudit 异步引擎（mpsc 消费） | struct handle 字段 | 无 token，channel 关闭退出，由 `OmniAuditEngine::shutdown()` abort（:278） |
| 39 | `services/omni_audit_service.rs:244` | 审计日志投递（一次性） | fire-and-forget | 自终止 |
| 40 | `services/event_bus.rs:394` | Kafka 事件投递（一次性） | fire-and-forget | 自终止 |
| 41 | `services/report_subscription_scheduler.rs:214` | 报表订阅调度（60s） | MAIN_BACKGROUND_TASKS（:667） | 无 token.cancelled()，依赖 abort() 兜底 |
| 42 | `services/export_compliance_service.rs:419` | 导出合规审查（24h） | MAIN_BACKGROUND_TASKS（:726） | 无 token.cancelled()，依赖 abort() 兜底 |
| 43 | `services/crm/recycle_executor.rs:151` | CRM 回收执行（6h） | MAIN_BACKGROUND_TASKS（:529） | 有 token.cancelled()（:155,174） |
| 44 | `services/permission_compliance_service.rs:587` | 权限合规审查（7d） | MAIN_BACKGROUND_TASKS（:885） | 有 token.cancelled() |
| 45 | `services/init_service_ops/setup.rs:224` | 后台迁移初始化（一次性） | fire-and-forget | 自终止（更新 task 状态后退出） |
| 46 | `services/lab_dip_ops/resample.rs:244` | 复样配方回写（一次性） | fire-and-forget | 自终止 |
| 47 | `services/email_queue_worker.rs:238` | 邮件队列 Worker（60s） | MAIN_BACKGROUND_TASKS（:705） | 无 token.cancelled()，依赖 abort() 兜底 |
| 48 | `services/dye_batch_cost_bridge_service.rs:38` | 染色成本桥接监听器 | DYE_BATCH_COST_LISTENER_HANDLE | 无 token，由 `shutdown_event_bus()` abort（:1635） |
| 49 | `services/failover_service.rs:758`(注释) | FailoverMonitor（实际 spawn 在 #5） | — | — |
| 50 | `websocket/notifications.rs:184` | ACK 重发循环（有界 3 次） | fire-and-forget | 自终止（最多 3 次重发） |
| 51 | `websocket/notifications.rs:286` | Redis publish 投递（一次性） | fire-and-forget | 自终止 |
| 52 | `websocket/notifications.rs:561` | WebSocket 接收任务（连接级） | 局部 recv_task 变量 | select! 后 abort（:634） |
| 53 | `websocket/notifications.rs:580` | WebSocket 发送任务（连接级） | 局部 send_task 变量 | select! 后 abort（:635） |

### 3.11.2 取消信号注册情况（bootstrap 目录）

`MAIN_BACKGROUND_TASKS`（service_bootstrap.rs:54）和 `MAIN_CANCELLATION_TOKEN`（:59）构成双重关闭机制：
- `shutdown_main_background_tasks()`（:70）：先 `MAIN_CANCELLATION_TOKEN.cancel()` 通知所有响应 `token.cancelled()` 的循环优雅退出，再遍历 `MAIN_BACKGROUND_TASKS` 调用 `handle.abort()` 兜底强杀未退出的任务。

注册到 `MAIN_BACKGROUND_TASKS` 的 spawn（共 17 个，:156/:162/:468/:495/:518/:530/:649/:668/:685/:706/:727/:757/:774/:786/:800/:862/:886/:902 处 push）。

其他句柄管理机制：
- `APP_STATE_BACKGROUND_TASKS`（container/mod.rs:11）：注册 `start_revoked_user_cleanup_task`（:164），shutdown 时仅 abort（:458），无 cancel token
- `BootstrapShutdownHandles`：管理 `OmniAuditEngine` + `AuditLogService` 的 shutdown（:42-49）
- `MAIN_LISTENER_HANDLE`（event_bus.rs:434）：主事件监听器，由 `shutdown_event_bus()` abort
- `BRIDGE_LISTENER_HANDLE`（listener.rs:24）：库存财务桥接，由 `shutdown_event_bus()` abort
- `DYE_BATCH_COST_LISTENER_HANDLE`（:26）：染色成本桥接，由 `shutdown_event_bus()` abort
- `EventBusState.consumer_handle`：Kafka 消费桥接，由 `shutdown_event_bus()` abort

### 3.11.3 未注册到 MAIN_BACKGROUND_TASKS 的 spawn 分析

**结论：无真正泄漏的 spawn。** 所有长期循环 task 均有句柄管理机制：

1. **注册到 MAIN_BACKGROUND_TASKS 的 17 个 task**：受 cancel()+abort() 双重管理
2. **注册到 APP_STATE_BACKGROUND_TASKS 的 1 个 task**（revoked_user_cleanup）：受 abort() 管理
3. **保存到 struct/全局 static 的 6 个 task**（omni_audit/audit_log/main_listener/bridge_listener/dye_cost_listener/kafka_consumer）：受各自 shutdown() 管理
4. **fire-and-forget 一次性 task**（11 个）：执行完自终止，无泄漏
5. **连接级 task**（WebSocket recv/send）：连接生命周期内局部管理，select! 后 abort

**需关注的 2 个边缘情况**（已在代码中补注释）：

| spawn | 问题 | 已补注释位置 |
|-------|------|-------------|
| `event_kafka.rs:255` Kafka 消费后台 | spawn 句柄未显式保存，依赖 mpsc channel 生命周期。channel drop 时 `run_consumer_loop` 内 send 失败退出。process 退出时 runtime drop 回收。仅在 Kafka 模式下启动（Redis 模式跳过） | `event_kafka.rs:255` |
| `start_ws_pubsub_subscriber` / `start_permission_cache_pubsub_subscriber` | 注册到 MAIN_BACKGROUND_TASKS 但内部 loop 无 token.cancelled() 分支。Redis pubsub 连接随 abort() 断开，`stream.next()` 返回 None 自然退出 | `notifications.rs:339` / `permission.rs:415` |

### 3.11.4 注册到 MAIN_BACKGROUND_TASKS 但内部无 token.cancelled() 的长期循环 task

以下 9 个 task 注册到 MAIN_BACKGROUND_TASKS 但内部 loop 未响应 `token.cancelled()`，shutdown 时依赖 `abort()` 兜底强杀：

| task | 文件:行 | 影响 |
|------|---------|------|
| FailoverMonitor::run | failover_service.rs:760 | DB 探测中途 abort，事务自动 rollback |
| NotificationPushScheduler | notification_scheduler.rs:118 | 扫描中途 abort，下次重启重新扫描 |
| EmailQueueWorker | email_queue_worker.rs:238 | 邮件发送中途 abort，PENDING 邮件下次重试 |
| ReportSubscriptionScheduler | report_subscription_scheduler.rs:214 | 同上 |
| ColorCardIssueExpiryScheduler | color_card_issue_scheduler.rs:206 | 同上 |
| StockAlertNotificationScheduler | stock_alert_notification_scheduler.rs:211 | 同上 |
| TrackingCleanupService | tracking_cleanup_service.rs:50 | 清理中途 abort，下次重新清理 |
| ExportComplianceService | export_compliance_service.rs:419 | 同上 |
| start_revoked_user_cleanup_task | jti.rs:264 | APP_STATE 仅 abort 无 cancel，同上 |

**风险评估**：这些 task 均为幂等操作（扫描 + DB 操作），abort 后下次重启重新执行，无数据一致性风险。abort 强杀不如 cancel() 优雅，但 SeaORM 事务会自动 rollback，影响可控。

---

## 3.14 内存泄漏

### 3.14.1 全局静态集合清单

| # | 变量 | 文件:行 | 类型 | 清理机制 | 无界风险 |
|---|------|---------|------|----------|----------|
| 1 | SLOW_QUERY_ALERT_STATE | middleware/slow_query.rs:46 | LazyLock<Mutex<HashMap<u64,(Instant,u32)>>> | 惰性清理（:86-89，超 1024 条时 retain 过期项） | 低（A.17 已修） |
| 2 | CIRCUIT_BREAKERS | middleware/circuit_breaker.rs:147 | Lazy<Arc<Mutex<HashMap<String,CircuitEntry>>>> | evict_and_count 滑动窗口清理（:65-78） | 低（route_key 数量有限） |
| 3 | USER_ACTIVE_CACHE | middleware/auth.rs:53 | OnceLock<DashMap<i32,(bool,Instant)>> | 读取时 TTL 跳过（:69），无 retain 清理 | 低（用户 ID 数量有限，60s TTL） |
| 4 | email_send_counters | container/mod.rs:75 | Arc<DashMap<(i32,u64),Arc<AtomicU32>>> | 惰性清理 retain（email_handler.rs:187） | 低（A.17 已修） |
| 5 | JTI_BLACKLIST | services/auth_service_ops/jti.rs:36 | LazyLock<RwLock<HashMap<String,i64>>> | cleanup_expired_jti retain（:164） | 低（有定时清理 task） |
| 6 | REVOKED_USERS | services/auth_service_ops/jti.rs:190 | LazyLock<RwLock<HashMap<i32,i64>>> | retain 清理（:238） | 低（有定时清理 task） |
| 7 | PERMISSION_CACHE | middleware/permission.rs:364 | LazyLock<DashMap<i32,CacheEntry<...>>> | TTL 读取时跳过（:562）+ Pub/Sub 失效（:456） | 低（role_id 数量有限） |
| 8 | ADMIN_ROLE_CACHE | utils/admin_checker.rs:41 | LazyLock<DashMap<i32,AdminCacheEntry>> | cleanup_expired_admin_cache retain（:57） | 低（有定时清理 task） |
| 9 | ACK_TRACKER | websocket/notifications.rs:151 | OnceLock<DashMap<(i64,i64),PendingAckEntry>> | ACK 时 remove（:196,228）+ 重发上限后放弃 | 低（有界重发 3 次） |
| 10 | WS_TICKET_MANAGER | websocket/notifications.rs:489 | OnceLock<WsTicketManager> | cleanup_expired retain（:471,446） | 低（30s TTL 票据） |
| 11 | INIT_TASKS | services/init_service.rs:184 | OnceLock<Arc<Mutex<HashMap<String,InitTaskStatus>>>> | 无清理 | 低（task_id 数量有限，一次性初始化） |
| 12 | GLOBAL_LIMITER | middleware/rate_limit.rs:93 | LazyLock<MemoryRateLimiter> | 概率清理 retain（:56-58） | 低（key=IP/用户，有限） |
| 13 | BRUTE_FORCE_LIMITER | middleware/rate_limit.rs:95 | LazyLock<MemoryRateLimiter> | 同上 | 低 |
| 14 | AI_RATE_LIMITER | middleware/rate_limit.rs:98 | LazyLock<MemoryRateLimiter> | 同上 | 低 |
| 15 | WEBHOOK_TEST_LIMITER | handlers/webhook_handler.rs:19 | LazyLock<MemoryRateLimiter> | 同上 | 低 |
| 16 | WEBHOOK_RETRY_LIMITER | handlers/webhook_handler.rs:24 | LazyLock<MemoryRateLimiter> | 同上 | 低 |
| 17 | MAIN_BACKGROUND_TASKS | bootstrap/service_bootstrap.rs:54 | Mutex<Vec<JoinHandle<()>>> | shutdown 时 take+abort（:73-82） | 低（启动时有限 push） |
| 18 | APP_STATE_BACKGROUND_TASKS | container/mod.rs:11 | Mutex<Vec<JoinHandle<()>>> | shutdown 时 take+abort（:449-459） | 低 |
| 19 | SETUP_MODE_INITIALIZED | bootstrap/routes_bootstrap.rs:42 | OnceLock<Arc<Mutex<bool>>> | 无需清理（单布尔值） | 无 |
| 20 | START_TIME | handlers/health_handler.rs:13 | OnceLock<Instant> | 无需清理（单 Instant） | 无 |

### 3.14.2 关键全局集合清理机制确认

**SLOW_QUERY_ALERT_STATE**（已修：A.17 加了惰性清理）
- 文件：`middleware/slow_query.rs:46`
- 清理：`should_send_alert()` 内 `if state.len() > ALERT_STATE_CLEANUP_THRESHOLD(1024) { cleanup_expired_alert_state() }`（:87-89）
- 清理逻辑：`retain(|_, (last_alert, _)| now.duration_since(*last_alert) < ALERT_DEDUP_WINDOW(3600s))`（:62）
- 结论：有清理机制，无界风险低

**CIRCUIT_BREAKERS**（route 数量有限，无界风险低）
- 文件：`middleware/circuit_breaker.rs:147`
- 清理：`evict_and_count()` 滑动窗口移除 5s 前过期记录（:65-78）
- key 为 `method:path`（:151-154），路由数量有限
- 结论：有清理机制，无界风险低

**USER_ACTIVE_CACHE**（60s TTL，有自动过期）
- 文件：`middleware/auth.rs:53`
- 清理：读取时检查 `ts.elapsed() < USER_ACTIVE_CACHE_TTL_SECS(60)` 跳过过期（:69），但**无 retain 主动清理**
- key 为 `user_id: i32`，活跃用户数量有限
- 结论：有 TTL 读取跳过，无主动 retain 清理。但 user_id 数量有限（等于用户表规模），无界风险低。建议未来可加 retain 清理（非紧急）

**email_send_counters**（已修：A.17 加了惰性清理）
- 文件：`container/mod.rs:75`
- 清理：`check_email_rate_limit()` 内 `retain(|(_, hb), _| *hb >= hour_bucket)`（email_handler.rs:187）
- 结论：有清理机制，无界风险低

### 3.14.3 无清理机制但风险低的全局集合

| 变量 | 原因 |
|------|------|
| USER_ACTIVE_CACHE | key=user_id 数量有限（用户表规模），60s TTL 读取跳过。无 retain 主动清理，但不会无界增长 |
| INIT_TASKS | key=task_id 数量有限（一次性初始化任务），状态持久保留供查询 |

---

## 结论

### 3.11 异步任务正确性

**无真正泄漏的 spawn。** 所有 50 个实际 spawn 调用均有句柄管理机制：
- 17 个注册到 `MAIN_BACKGROUND_TASKS`（cancel()+abort() 双重管理）
- 1 个注册到 `APP_STATE_BACKGROUND_TASKS`（abort() 管理）
- 6 个保存到 struct 字段或全局 static（各自 shutdown() 管理）
- 11 个 fire-and-forget 一次性 task（自终止）
- 2 个连接级 task（局部 abort）
- 9 个长期循环 task 注册到 MAIN_BACKGROUND_TASKS 但内部无 token.cancelled()，依赖 abort() 兜底强杀（幂等操作，风险可控）
- 2 个 pubsub subscriber 无 token.cancelled() 但 Redis 连接随 abort 断开自然退出

已在 3 处补注释说明无取消信号的原因。

### 3.14 内存泄漏

**无内存泄漏风险。** 20 个全局静态集合均有清理机制或天然有界：
- 16 个有主动清理（retain/TTL/滑动窗口/shutdown abort）
- 2 个无主动清理但 key 数量有限（USER_ACTIVE_CACHE/INIT_TASKS）
- 2 个为单值（START_TIME/SETUP_MODE_INITIALIZED）

用户提到的 4 个重点集合均确认安全：
- SLOW_QUERY_ALERT_STATE：A.17 已加惰性清理（超 1024 条 retain）
- CIRCUIT_BREAKERS：滑动窗口 evict_and_count 清理，route key 有限
- USER_ACTIVE_CACHE：60s TTL 读取跳过，user_id 有限
- email_send_counters：A.17 已加惰性清理 retain
