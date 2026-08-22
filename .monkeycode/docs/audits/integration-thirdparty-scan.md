# 接口集成扫描审计报告

- 审计日期：2026-08-22
- 审计范围：backend/src/services 下的第三方集成、幂等性、数据同步、降级熔断、Webhook 可靠性、契约测试、故障演练
- 审计模式：只读扫描，未修改任何代码文件
- 输出目录：`.monkeycode/docs/audits/`

## 1. 扫描命令与原始结果

### 28.1 第三方鉴权

```bash
grep -rn "api_key\|bearer\|oauth\|hmac\|signature" backend/src/services/email_service.rs backend/src/services/event_kafka.rs | head -10
```

关键命中（节选）：
- `email_service.rs:10` `use hmac::{Hmac, Mac};`
- `email_service.rs:228` `pub api_key: String`
- `email_service.rs:341` `let api_key = std::env::var("EMAIL_API_KEY").ok()?;`
- `email_service.rs:519` `.header("Authorization", format!("Bearer {}", self.config.api_key))`
- `email_service.rs:546` 阿里云 DirectMail：HMAC-SHA1 + Base64（RPC V1 签名）
- `email_service.rs:621` 腾讯云 SES：TC3-HMAC-SHA256（V3 签名）
- `email_service.rs:807` `let signature = BASE64_STANDARD.encode(mac.finalize().into_bytes());`

`event_kafka.rs`：无 `api_key/bearer/oauth/hmac/signature` 命中（Kafka 连接依赖底层 `KafkaSettings`，鉴权细节未出现在 service 层）。

### 28.2 幂等性

```bash
grep -rn "idempotency\|幂等\|request_id\|dedup\|processed_events\|event_dead_letter" backend/src/services/ | head -10
```

关键命中：
- `fixed_asset_service.rs:873` 折旧计提幂等由 `uk_fa_depreciation_records_asset_period` 唯一约束保证
- `custom_order_state_service.rs` 状态门校验关联字段非空
- `bulk_color_approval_service.rs:1192/1233` 使用 `dedup_key` 字段
- `collection_task_service.rs:194/313/355` 催收任务幂等检查
- `audit_log_service.rs:88` 显式声明"幂等：多次调用安全，仅首次调用实际 abort"
- `audit_log_service.rs:250/365/407/437/409` 记录 `request_id`
- `notification_service.rs:52/72/73/77` `dedup_key` + 5 分钟窗口去重

未命中：`processed_events`、`event_dead_letter`（Kafka 侧无消费去重表）。

### 28.3 数据同步

```bash
grep -rn "sync\|双向\|replicate" backend/src/services/ | grep -v "async\|Arc\|Pin" | head -10
```

关键命中：
- `quality_inspection_service.rs:311/433/450/488/499` `sync_receipt_inspection_status`、`sync_stock_grade_for_downgrade`、`sync_sales_price_for_downgrade`（业务字段级单向同步）
- `product_service.rs:5` `sync`：ES 同步辅助（`build_product_doc` / `sync_product_to_es`）
- `audit_log_service.rs` 命中均为 `tokio::sync` / `std::sync` 标准库 trait bound（非业务同步）

未命中：`双向`、`replicate`（无双向同步、无跨库复制抽象）。

### 28.4 降级熔断

```bash
grep -rn "fallback\|降级\|degrade\|circuit" backend/src/ | head -10
```

关键命中：
- `cli/util/upgrade.rs:215/240/243/480` 版本降级检查（禁止降级除非 `--force-downgrade`，非服务熔断）
- `cli/util/mod.rs:367` `unwrap_or_default` 安全降级
- `routes/static.rs:53/62/63/111/137/143/147/159/166/170` 静态资源 fallback 路径
- `routes/bulk_color_approval.rs:12` 质量降级业务接口
- `bootstrap/routes_bootstrap.rs:52/54/57/116` 锁中毒优雅降级（`e.into_inner()`）
- `failover_service.rs:64-75` `failover_circuit_state` 指标，但仅是 Prometheus Gauge，无实际熔断状态机

未命中：`circuit breaker` 完整实现（half-open/open/closed 状态机）。

### 28.5 Webhook 可靠性

```bash
grep -rn "webhook\|callback\|retry\|dead_letter" backend/src/services/webhook_service.rs | head -10
```

关键命中（webhook_service.rs 共 378 行）：
- `:9` `Webhook 最大重试次数上限（超阈值后停增 retry_count 并置 last_error）`
- `:87` 创建时 `retry_count: Set(0)`
- `:160` 批次 251：支持 retry 重投（持久化 payload+event）
- `:198-219` 持久化最终状态，成功重置 retry_count，失败递增
- `:226-236` `apply_retry_increment`：达上限标记永久失败
- `:360-361` `GET /webhooks/:id/logs` 返回 `last_triggered_at / last_status / retry_count`，注释明确"当前未独立持久化调用日志（无 webhook_logs 表）"

未命中：`dead_letter`（无死信队列）、`callback`（无回调签名校验）、退避策略（`exponential`/`backoff`）。

### 28.6 契约测试

```bash
grep -rn "contract" backend/tests/ | head
```

关键命中：
- `utils_docx_export_test.rs:51/57` sales_contract 文件名断言（非契约测试）
- `middleware_permission_test.rs:122/123/135/136` 资源路由名 `contracts` 断言
- `handlers_print_handler_test.rs:47/55` print handler 覆盖 `sales_contract`
- `quotation_e2e_legacy_test.rs:227/236` `test_app_error_validation_contract` / `test_app_error_not_found_contract`（函数名含 contract，实为 AppError 单元测试）
- `handlers_sales_contract_handler_test.rs` 销售合同模型序列化与金额计算单元测试

未命中：`contract test`、`pact`、`schemathesis`、`openapi test`（无真正的消费者/提供者契约测试工具）。

OpenAPI 覆盖率（`docs.rs`）：
- `#[derive(OpenApi)]` at `docs.rs:18`
- 已注册 paths：14 个（auth 8 + user 5 + health 1）
- 文档自述："当前覆盖率：14/115 handlers（~12%）"
- 全仓库 `utoipa::path` 注解 22 处，`utoipa::ToSchema` 等 22 处
- 路由声明总数（`Router::new`/`.route(`）约 1721 处（含子路由）
- 实际有效 handler 约 115 个，注册率 ~12%

### 28.7 故障演练

```bash
grep -rn "failover\|backup\|fallback.*mode" backend/src/services/failover_service.rs | head -10
```

关键命中（failover_service.rs 共 835 行）：
- `:21-22` failover_event / failover_status 模型
- `:31-39` `primary_total` / `primary_failed_total` / `backup_total` / `switch_total` / `circuit_state` 指标
- `:187-203` `FailoverExecutor`（可选），`with_executor` 构造
- `:207-209` `get_active_connection`：配置 executor 时返回 ArcSwap 当前指向连接
- `:268-301` `test_switch`：先 `wait_for_backup_catchup(Duration::from_secs(10))`，再 `switch_to_backup`，更新 status 表为 `"backup" / "open"`
- `:301` `self.update_status_on_switch(function_name, "backup", "open")`

测试覆盖：
- `backend/tests/failover_metrics_test.rs`
- `backend/tests/services_failover_service_test.rs`（仅 `test_metrics_creation`，且已清理死代码测试）

未命中：`fallback.*mode`（无显式降级模式枚举）、`chaos`（无混沌工程注入）、`inject.*failure`（无故障注入框架）。

## 2. 七项评估结论

| # | 维度 | 评估结论 | 风险等级 |
|---|------|----------|----------|
| 28.1 | 第三方鉴权 | 邮件服务实现了阿里云 RPC V1 (HMAC-SHA1) 与腾讯云 TC3-HMAC-SHA256 双签名，密钥通过 `EMAIL_API_KEY` 环境变量注入；但 `event_kafka.rs` 的 service 层无鉴权相关代码，Kafka SASL/SCRAM 鉴权细节未在 service 层体现，无法判断是否启用。**风险**：Kafka 鉴权下沉到 `KafkaSettings` 配置层，service 层无可见校验，需到配置层确认。 | 中 |
| 28.2 | 幂等性 | 业务层幂等机制较完善：唯一约束（折旧）、dedup_key（通知/审批）、幂等检查（催收）、安全 abort（审计日志）。**缺口**：Kafka 消费侧（`event_kafka.rs`）无 `processed_events` 去重表、无 `event_dead_letter`，offset 仅在内存 `last_offsets` 维护，消费失败仅重连 3 次后关闭流，存在 at-least-once 下的重复消费风险。 | 高 |
| 28.3 | 数据同步 | 仅有单向业务字段同步（质检状态→库存等级/售价、ES 同步），无双向同步、无跨库 `replicate` 抽象。**风险**：failover 主备切换时若备库 catch-up 未完成即切换，存在数据不一致（代码已 warn 但仍继续切换）。 | 中 |
| 28.4 | 降级熔断 | 仅有局部降级（锁中毒 `into_inner`、静态资源 fallback、版本降级检查）。**缺口**：`failover_circuit_state` 仅是 Prometheus Gauge 指标，无真实熔断状态机（open/half-open/closed）、无自动熔断触发逻辑、无降级 fallback 函数注册机制。 | 高 |
| 28.5 | Webhook 可靠性 | 有 retry_count 持久化与上限保护（批次 251）。**缺口**：无死信队列（`dead_letter`）、无回调签名校验（`callback` 签名验证）、无退避策略（指数退避/抖动）、无独立 webhook_logs 表（仅状态字段汇总）、无异步投递 worker（同步触发）。 | 高 |
| 28.6 | 契约测试 | 无真正的契约测试。`tests/` 中 "contract" 均为合同业务实体名或 AppError 单测，无 pact/schemathesis/openapi 校验。OpenAPI 覆盖率仅 ~12%（14/115），远低于生产可用阈值。 | 高 |
| 28.7 | 故障演练 | 有 `test_switch` 手动切换接口 + `wait_for_backup_catchup`，但仅 Prometheus 指标埋点。**缺口**：无 chaos 注入、无故障演练编排、无自动 failover 触发（仅手动 `test_switch`）、无 fallback mode 枚举、测试覆盖极薄（仅 `test_metrics_creation`）。 | 高 |

## 3. 汇总

- **绿灯项（0 项）**：无。
- **黄灯项（2 项）**：28.1 第三方鉴权（邮件完善，Kafka 鉴权下沉不可见）、28.3 数据同步（单向同步存在，主备切换存在数据丢失风险）。
- **红灯项（5 项）**：28.2 幂等性（Kafka 消费侧去重缺失）、28.4 降级熔断（无真实熔断状态机）、28.5 Webhook 可靠性（无死信/退避/签名校验）、28.6 契约测试（无契约测试工具，OpenAPI 覆盖率 12%）、28.7 故障演练（无 chaos 注入，测试覆盖极薄）。

## 4. 关键修复建议（按优先级）

1. **P0 - Kafka 消费幂等**：在 `event_kafka.rs` 增加 `processed_events` 去重表或事务性 offset 提交，消费失败转入 `event_dead_letter` 队列。
2. **P0 - Webhook 死信与退避**：引入 `webhook_logs` 表、死信队列、指数退避 + 抖动、HMAC 回调签名校验、异步投递 worker。
3. **P1 - 熔断状态机**：在 `failover_service.rs` 实现真实 CircuitBreaker（open/half-open/closed），驱动 `failover_circuit_state` 指标，而非仅埋点。
4. **P1 - 契约测试与 OpenAPI**：引入 pact 或 schemathesis，将 OpenAPI 覆盖率从 12% 提升至 ≥80%。
5. **P2 - 故障演练**：引入 chaos 注入框架（如 chaos-mesh 风格的进程级故障注入），覆盖 Kafka 断连、DB 主库不可用、邮件服务商限流场景。
6. **P2 - 双向同步与备库 catch-up**：`test_switch` 在备库 catch-up 超时时应阻断切换而非 warn 后继续。

## 5. 附录：文件清单

| 文件 | 行数 | 角色 |
|------|------|------|
| `backend/src/services/email_service.rs` | 987 | 邮件第三方鉴权 |
| `backend/src/services/event_kafka.rs` | 453 | 事件总线/消费 |
| `backend/src/services/webhook_service.rs` | 378 | Webhook 投递 |
| `backend/src/services/failover_service.rs` | 835 | 主备切换/熔断指标 |
| `backend/src/docs.rs` | 86 | OpenApi 注册 |
| `backend/tests/services_failover_service_test.rs` | 9 | failover 测试（极薄） |
| `backend/tests/failover_metrics_test.rs` | - | failover 指标测试 |
