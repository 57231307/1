# 日志结构化与上下文传递扫描报告

- 审计日期：2026-08-22
- 审计范围：`backend/src/`
- 审计任务：3.13 日志结构化与上下文传递扫描

## 1. 总体统计

| 指标 | 数量 | 说明 |
|------|------|------|
| `tracing::info!/warn!/error!/debug!` 全限定调用 | 779 | 直接使用 `tracing::` 前缀的结构化日志 |
| `tracing::` 全部相关行（含 use/常量等） | 897 | 包含导入与配置 |
| 简短形式调用（`info!`/`warn!`/`error!`/`debug!`，需经 `use tracing::{...}` 导入） | 992 | 业务代码主流写法 |
| 结构化日志调用合计（全限定 + 简短） | 约 1771 | 真正使用 tracing 宏的总量 |
| 非结构化日志（`println!`/`eprintln!`/`print!`，排除注释/test/fixture） | 348 | 集中在 `cli/` 与少量脚本 |
| `cli/` 目录非结构化日志数 | 328 | 占非结构化日志的 94% |
| `cli/` 目录结构化日志数 | 3 | 几乎完全依赖 println |

## 2. 带上下文字段的日志比例

| 指标 | 数量 |
|------|------|
| 带上下文字段（`user_id`/`order_id`/`error`/`path`/`method`）的 `tracing::` 行 | 243 |
| 全限定结构化日志样本 | 779 |
| 服务层简短调用 | 429 |
| 服务层携带 `trace_id` 的日志 | 0 |
| 使用 `{}` 格式化的日志（非结构化字段） | 192 |
| 使用 `key = value` 结构化字段的日志 | 101 |

### 比例评估

- 全限定形式中带结构化字段的比例：`243 / 779 ≈ 31%`
- 若把全部 `tracing::` 宏调用（约 1771）作为分母，带显式上下文字段的比例约为 `243 / 1771 ≈ 13.7%`
- 大量服务层日志使用 `info!("用户 {} 正在创建固定资产：{}", user_id, asset_no)` 形式，属于"位置参数格式化"而非"键值对字段"，对结构化日志采集不友好
- 服务层日志几乎完全不携带 `trace_id`（0/429），跨层链路追踪能力薄弱

## 3. trace_id / span_id 传递情况

| 指标 | 数量 |
|------|------|
| `trace_id`/`span_id`/`traceparent` 出现行数 | 181 |
| 涉及文件数 | 20 |

### 已实现的传递链路

中间件层已建立完整的 trace 上下文传递机制：

1. `middleware/trace_context.rs` — 入口中间件
   - 从请求头 `traceparent` 解析或生成新的 `TraceContext`
   - 将 ctx 存入 `Request::extensions()` 供下游 handler/service 读取
   - 创建 root `tracing::Span`，把 trace_id/span_id 写入 span 字段
   - 响应头回写 `X-Trace-Id`，便于客户端关联
   - 支持 V15 P2 20.1-C tail-based sampling：5xx 或慢请求强制采样
   - 日志输出 `trace_id = %ctx.trace_id, span_id = %ctx.span_id, method, path, status, elapsed_ms` 完整字段

2. `middleware/audit_context.rs` — 审计上下文
   - 在 `trace_context` 之后挂载，复用其注入的 trace_id 作为 request_id
   - 找不到时本地生成 UUID v4 兜底

3. `middleware/omni_audit.rs` — 统一审计
   - trace_id 在请求/响应审计日志中显式传递
   - `log_request_start` / `log_request_complete` 携带 trace_id

4. `observability/span.rs`、`observability/trace_context.rs`、`observability/mod.rs`
   - 提供 root_span 构造与 TraceContext extract/extract_or_new 能力

### 传递链路的断点

| 层级 | trace_id 携带情况 |
|------|-------------------|
| 中间件层（bootstrap/middleware） | 完整携带 |
| handler 层 | 部分携带（`omni_audit_handler`、`crm_handler` 有引用） |
| service 层 | 几乎不携带（0/429） |
| model 层 | 无 |
| utils/error.rs | 仅在错误转换时保留 |

**核心结论**：trace_id 在中间件层生成并写入 span/响应头，但未通过参数或 `tracing::Span` current context 自动下沉到 service 层日志，导致业务日志无法与请求 trace 关联。

## 4. 不规范日志示例

### 4.1 非结构化日志（println/eprintln）

集中出现在 CLI 与脚本工具中：

```
backend/src/cli/migrate.rs:54:            println!("开始执行数据库迁移...");
backend/src/cli/migrate.rs:56:            println!("迁移执行完成！");
backend/src/cli/admin.rs:82:    eprintln!("=== 密码哈希生成成功 ===");
backend/src/cli/admin.rs:83:    eprintln!("Argon2 哈希: {}", password_hash);
backend/src/cli/util/misc.rs:8:  println!("=== 清理系统 ===\n");
backend/src/cli/util/misc.rs:24: println!("[WARN] 清理日志失败（可忽略）: {}", e);
backend/src/bin/hash_password.rs: ...
```

问题：
- 无法被 tracing 订阅器采集，生产环境运行时日志丢失
- `eprintln!("Argon2 哈希: {}", password_hash)` 直接输出敏感凭据到 stderr
- `[WARN]` 前缀手工模拟日志级别，无法按级别过滤

### 4.2 格式化而非结构化字段

```
backend/src/services/fixed_asset_service.rs:108:
    info!("用户 {} 正在创建固定资产：{}", user_id, asset_no);
```

问题：
- `user_id`、`asset_no` 以位置参数嵌入消息字符串，日志采集端无法按字段检索
- 应改为 `info!(user_id = %user_id, asset_no = %asset_no, "固定资产创建")`

### 4.3 服务层日志缺少 trace_id

```
backend/src/services/fixed_asset_service.rs:430:
    info!(...)  // 无 trace_id 字段
```

问题：与中间件注入的 span 上下文脱节，无法关联到具体请求。

## 5. 建议

### 5.1 高优先级

1. **CLI 脚本日志迁移到 tracing**：将 `cli/` 下 328 处 `println!/eprintln!` 改为 `tracing::info!` 等宏，保证生产运行 CLI 时日志可被统一采集；敏感字段（密码哈希、token）不得直接输出。
2. **service 层补齐 trace_id**：在 service 函数签名或通过 `tracing::Span::current()` 获取当前 span 上下文，确保业务日志至少携带 `trace_id`，实现端到端链路关联。
3. **格式化日志改键值对**：将 `info!("用户 {} ...：{}", user_id, x)` 形式批量重构为 `info!(user_id = %user_id, x = %x, "消息")`，提升结构化采集可检索性。

### 5.2 中优先级

4. **统一日志字段命名**：`user_id`/`order_id`/`error`/`path`/`method` 已有使用，建议在 `.monkeycode/docs/` 增补日志字段规范，约定 `trace_id`、`span_id`、`user_id`、`entity_type`、`entity_id`、`error`、`duration_ms` 等标准字段。
5. **span 自动下沉**：在 handler 入口 `#[tracing::instrument]` 标注关键函数，让 span 自动传播到 service 调用栈，避免手工传递 trace_id 参数。
6. **敏感信息扫描补强**：`eprintln!("Argon2 哈希: {}", password_hash)` 应在 `log-sensitive-info-scan.md` 跟踪复核。

### 5.3 低优先级

7. **错误转换链路**：`utils/error.rs` 中 trace_id 引用应确保在错误向上传播时保留上下文，建议复核 `From`/`Into` 实现是否丢失 trace 字段。
8. **慢请求采样阈值**：`trace_context.rs` 的 `OTEL_SLOW_REQUEST_MS` 默认 2000ms，建议结合 P95 告警数据复核是否需要按环境差异化配置。
