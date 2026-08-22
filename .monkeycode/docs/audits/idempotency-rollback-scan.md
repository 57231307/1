# 接口幂等性 & 部署回滚监控审计报告

- 审计日期: 2026-08-22
- 审计范围: `backend/src/services/` 事件消费幂等机制（28.2）+ 系统更新回滚监控（26.9）
- 审计基线: 不修改业务逻辑，仅确认现状 + 必要处加注释/tracing 提醒

---

## 一、28.2 接口幂等性

### 1.1 现有幂等机制 grep 结果

执行 `grep -rn "idempotency|request_id|dedup|processed_events|event_dead_letter" backend/src/services/`，关键命中：

| 文件 | 行号 | 内容摘要 |
|------|------|----------|
| `event_idempotency_service.rs` | 1-99 | 幂等服务主体，基于 `processed_events` 表（`consumer_id` + `event_key` 主键）实现 `try_mark_processed_txn` / `try_mark_processed` / `unmark_processed` |
| `event_bus_ops/listener.rs` | 613, 667, 928, 1056, 1195, 1359, 1382, 1535 | 8 处 handler 调用 `EventIdempotencyService::try_mark_processed`，覆盖 QualityInspectionCompleted / ProcessStepReported / WageConfirmed / FabricInspectionGraded / BpmProcessFinished / LowStockAlert / MaterialShortageAlert 等事件 |
| `event_retry_service.rs` | 7, 93, 118 | 死信队列 `event_dead_letter` 表，记录无法处理的事件 |
| `po/receipt.rs` | 35, 102 | 采购收货幂等 `check_receipt_idempotency` |
| `event_notification_service.rs` | 82-780 | 通知层 `dedup_key` 去重（inventory_alert / order_submitted / order_approved 等） |

### 1.2 Kafka 消费侧去重确认

读取 `backend/src/services/event_kafka.rs`（453 行）：

- `run_consumer_loop`（303 行）：轮询所有 partition fetch records，仅维护 `last_offsets` 防重复消费同一 offset（Kafka 消费位点语义去重）。
- `process_kafka_record`（409 行）：反序列化为 `EventPayload` → `BusinessEvent`，推送到 `tokio::sync::mpsc` 通道。
- **Kafka 消费侧本身不做业务级 request_id 去重**，仅做 offset 级别的不重复拉取。

读取 `backend/src/services/event_bus_ops/kafka.rs`（106 行）：

- `spawn_kafka_consumer`（54 行）：从 `KafkaBackend::subscribe` 获取事件流，逐条 `local_tx.send(event)` 推入全局 `broadcast::Sender`（`EVENT_BUS`）。

读取 `backend/src/services/event_bus_ops/listener.rs`（1826 行）：

- `start_event_listener`（28 行）：`EVENT_BUS.subscribe()` 获取 broadcast receiver，`run_event_loop` 循环 `receiver.recv()` → `dispatch_business_event` 统一分发。
- `dispatch_business_event`（63 行）：match 分发到各 `handle_*` handler，**每个 handler 内部均调用 `EventIdempotencyService::try_mark_processed`**（consumer_id = `"event_bus_main"`，event_key 按业务 ID 拼接）。

### 1.3 幂等覆盖链路

```
Kafka record
  → event_kafka.rs::process_kafka_record (反序列化, offset 去重)
  → event_bus_ops/kafka.rs::spawn_kafka_consumer (推入 broadcast)
  → EVENT_BUS (broadcast channel)
  → listener.rs::run_event_loop (recv)
  → dispatch_business_event (match 分发)
  → handle_* (各 handler 调用 EventIdempotencyService::try_mark_processed)
```

**结论：Kafka 消费侧虽无独立的 request_id 去重，但事件经 broadcast 桥接到统一 listener 后，每个 handler 均通过 `processed_events` 表实现业务级幂等。** Broadcast 后端（非 Kafka 场景）与 Kafka 后端走同一分发路径，幂等覆盖一致。

### 1.4 幂等服务设计要点

- `try_mark_processed_txn`：在业务事务内插入 `processed_events`，主键冲突返回 `Ok(false)`，保证原子性（推荐用法）。
- `try_mark_processed`：无事务版本，存在并发窗口（不推荐）。
- `unmark_processed`：业务失败后清除幂等记录，使事件可重放（V15 P0 修复）。
- listener 中 8 处调用均使用无事务版本 `try_mark_processed`（非 `_txn`），幂等标记与业务副作用不在同一事务内，存在「幂等标记成功但业务失败」导致事件被误跳过的风险——属于已知设计权衡，不在本次审计修改范围。

### 1.5 28.2 结论

**幂等机制已存在且覆盖 Kafka 消费路径，无需新增 request_id 去重。** 本项仅确认，不修改代码。

---

## 二、26.9 部署后自动回滚监控

### 2.1 现有回滚逻辑 grep 结果

执行 `grep -rn "rollback|回滚|health.*check.*deploy|version.*check" backend/src/services/system_update_service.rs`：

- `system_update_service.rs` 为 facade（302 行），仅保留类型定义 + 纯函数，**回滚业务逻辑已迁移到 `system_update_ops/` 子模块**。

实际回滚逻辑位于：

| 文件 | 方法 | 行号 | 职责 |
|------|------|------|------|
| `system_update_ops/backup.rs` | `rollback` | 67-92 | 从备份路径恢复 backend/frontend/config/VERSION 文件 |
| `system_update_ops/backup.rs` | `rollback_to_version` | 94-104 | 按版本号查找备份并调用 `rollback` |
| `system_update_ops/apply.rs` | `do_update` | 52-112 | 更新主流程，步骤4/5失败时调用 `rollback` |

### 2.2 DB migration 回滚检查

读取 `backup.rs::rollback`（67 行）与 `apply.rs::do_update`（52 行）：

- `rollback` 仅操作文件系统：`fs::remove_dir_all` + `copy_dir` 恢复 backend/frontend/config 目录 + VERSION 文件。
- **不包含任何 DB migration 回滚逻辑**（无 `sea_orm_migration` down 调用、无 DB 备份恢复）。
- 项目 migration 在 `init_service_ops/setup.rs:274 run_migrations` 调用 `Migrator::up`，仅前向迁移，无 down 通道。

**确认：之前审计发现"仅回滚文件不回滚 DB"的问题仍然存在。** 若新版本包含破坏性 migration（如 DROP COLUMN），回滚后旧版二进制将面对不兼容的 DB schema。

### 2.3 回滚后健康检查

读取 `apply.rs::do_update`：

- 更新成功路径：步骤5调用 `verify_update()`（215 行，检查 backend 可执行文件或 VERSION 文件存在）。
- **回滚路径（步骤4失败 line 74 / 步骤5失败 line 87）：调用 `rollback` 后直接返回 `Err`，无任何健康检查**（不探活 HTTP /health、不冒烟测试服务可用性）。

`verify_update` 仅在更新成功时调用，回滚后不执行任何验证。

### 2.4 已执行的修改

在 `backup.rs::rollback` 方法添加（不改业务逻辑）：

1. **方法文档注释**：说明 limitation——仅回滚文件不回滚 DB migration，且回滚后无健康检查，属于已知技术债务，后续需补充。
2. `tracing::warn!`（回滚开始前）：提醒本次回滚不包含 DB migration 回滚。
3. `tracing::warn!`（回滚完成后）：提醒未执行健康检查，运维需人工确认。

### 2.5 26.9 结论

**回滚逻辑存在 DB migration 不回滚 + 回滚后无健康检查两项缺陷。** 本次按审计要求仅加注释 + tracing::warn 提醒，未改业务逻辑。后续需：
- 引入 migration 回滚机制（sea-orm-migration down 步骤或 DB 备份/恢复流程）。
- 回滚完成后补充自动健康检查（HTTP /health 探活 + 可执行文件冒烟测试），失败时触发告警或二次回滚。

---

## 三、修改文件列表

| 文件 | 修改类型 | 说明 |
|------|----------|------|
| `backend/src/services/system_update_ops/backup.rs` | 注释 + tracing | `rollback` 方法添加 limitation 文档注释 + 2 处 `tracing::warn!` 提醒（DB 不回滚 + 无健康检查），不改业务逻辑 |

---

## 四、两项审计结论汇总

| 任务 | 结论 | 是否改代码 |
|------|------|-----------|
| 28.2 接口幂等性 | 幂等机制已存在（`event_idempotency_service.rs` + `processed_events` 表），Kafka 消费路径经 broadcast 桥接到统一 listener，8 个 handler 均调用 `try_mark_processed`，覆盖完整 | 否，仅确认 |
| 26.9 部署回滚监控 | 回滚仅恢复文件不回滚 DB migration；回滚后无健康检查。已加注释 + tracing::warn 提醒，属已知技术债务 | 是，仅注释+tracing，不改业务逻辑 |
