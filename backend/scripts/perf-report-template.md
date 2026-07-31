# 后端性能报告模板

> **用途**：后端 API/数据库/内存/并发性能基线测试报告模板。
> **使用方式**：复制本模板为 `perf-report-YYYY-MM-DD.md`，填充实测数据后归档至 `backend/scripts/`。
> **配套工具**：`backend/scripts/p2-2-slow-query.sql`（慢查询采集）、`backend/src/middleware/slow_query.rs`（慢查询中间件）、`backend/benches/`（criterion 基准）、`backend/src/middleware/metrics.rs`（Prometheus 指标）。
> **前端性能报告**：见 [`frontend/scripts/p2-3-perf-report.md`](../../frontend/scripts/p2-3-perf-report.md)（V2Table 渲染性能，TTI/FPS）。

---

## 1. 报告元信息

| 项 | 值 |
|----|----|
| 执行日期 | YYYY-MM-DD |
| 后端版本 | （填 `bingxi-backend` 版本号，见 `backend/Cargo.toml`） |
| Git commit | （填 commit hash） |
| 测试人 | （填姓名/账号） |
| 测试环境 | （沙箱 / Staging / Pre-prod） |
| 数据库 | PostgreSQL xx.x（连接串脱敏：`postgres://***@host:5432/db`） |
| Redis | （版本 + 是否启用） |
| Kafka | （版本 + 是否启用，影响事件总线后端） |

---

## 2. 验收标准（基线阈值）

> 以下阈值依据业务 SLA 制定，超阈值项需在 §7 列出优化计划。

| 维度 | 指标 | 基线阈值 | 说明 |
|------|------|----------|------|
| API 响应 | P50 | < 100 ms | 列表/详情类接口 |
| API 响应 | P95 | < 500 ms | 含分页/过滤的查询接口 |
| API 响应 | P99 | < 1000 ms | 含事务写入的接口 |
| 数据库 | 慢查询数 | 0 条 > 500 ms | 采样期内的慢查询（见 `slow_query` 中间件） |
| 数据库 | N+1 查询 | 0 处 | 关联加载需用 `find_also_related` / 预加载 |
| 内存 | RSS 峰值 | < 512 MB | 单实例稳态内存（排除启动期） |
| 内存 | 内存泄漏 | 无持续增长 | 连续压测 30 min RSS 无单调上升 |
| 并发 | 吞吐量 | > 500 RPS | 关键只读接口（如 `GET /inventory/stocks`） |
| 并发 | 错误率 | < 0.1% | 压测期 5xx / 超时占比 |
| 并发 | P99（高压） | < 2000 ms | 1.5x 预期峰值并发下 |

---

## 3. API 响应时间基准

> **采集方法**：对关键端点发起 N 次请求（建议 N ≥ 200），记录 P50/P95/P99。
> **工具建议**：`wrk` / `hey` / `k6`；或读取 Prometheus `http_request_duration_seconds` 直方图。

| 端点 | 方法 | 并发 | 样本数 | P50 (ms) | P95 (ms) | P99 (ms) | 状态 |
|------|------|------|--------|----------|----------|----------|------|
| `/health` | GET | 1 | 200 | | | | |
| `/auth/me` | GET | 1 | 200 | | | | |
| `/inventory/stocks` | GET | 10 | 200 | | | | |
| `/inventory/counts` | GET | 10 | 200 | | | | |
| `/inventory/counts` | POST | 5 | 100 | | | | |
| `/dye-batches` | GET | 10 | 200 | | | | |
| `/sales-orders` | GET | 10 | 200 | | | | |
| `/purchase-orders` | GET | 10 | 200 | | | | |
| `/ar/invoices` | GET | 10 | 200 | | | | |
| `/ap/payments` | GET | 10 | 200 | | | | |

**结论**：（填：是否全部达标 / 超阈值端点列表）

---

## 4. 数据库查询性能

> **采集方法**：
> 1. 启用 `slow_query` 中间件（`backend/src/middleware/slow_query.rs`），阈值设为 500 ms；
> 2. 执行 `backend/scripts/p2-2-slow-query.sql` 采集 `pg_stat_statements` Top 慢查询；
> 3. 检查 `EXPLAIN ANALYZE` 执行计划是否命中索引（Seq Scan on 大表为风险项）。

### 4.1 慢查询清单（> 500 ms）

| 序号 | SQL 摘要 | 调用次数 | 平均耗时 (ms) | 最大耗时 (ms) | 总行扫描 | 命中索引 | 风险等级 |
|------|---------|---------|--------------|--------------|---------|---------|---------|
| 1 | | | | | | | |
| 2 | | | | | | | |

### 4.2 N+1 查询排查

| 模块 | 调用点（文件:行） | 关联实体 | 是否预加载 | 修复建议 |
|------|----------------|---------|-----------|---------|
| | | | | |

### 4.3 索引覆盖核查

- [ ] `inventory_stocks`（warehouse_id / product_id 复合索引）
- [ ] `dye_batches`（batch_no 唯一索引、status 索引）
- [ ] `sales_orders` / `purchase_orders`（status + created_at 复合索引）
- [ ] `audit_logs`（created_at 分区索引）

**结论**：（填：慢查询数量、是否需补索引/改查询）

---

## 5. 内存使用

> **采集方法**：
> 1. `/proc/{pid}/status` 读取 `VmRSS`（或 `sysinfo` crate 数据，见 `health_handler.rs`）；
> 2. 连续压测 30 min，每 30s 采样一次 RSS，绘制趋势图；
> 3. `moka` 缓存命中率（`/metrics` Prometheus 端点）。

| 指标 | 启动后 1 min | 稳态（10 min） | 压测峰值（30 min） | 停压后 5 min | 是否泄漏 |
|------|-------------|---------------|------------------|-------------|---------|
| RSS (MB) | | | | | |
| Heap（如启用 jemalloc） | | | | | |
| 缓存命中率 | — | | | — | — |
| 活跃 DB 连接数 | | | | | |

**泄漏判定**：停压后 RSS 回落至稳态 ±10% 内视为无泄漏；持续不回落需排查（建议 `valgrind --tool=massif` 或 `cargo run --features dhat`）。

**结论**：（填：内存是否达标 / 缓存命中率 / 泄漏排查结论）

---

## 6. 并发处理能力

> **采集方法**：对关键只读接口阶梯式加压（50 → 100 → 200 → 500 RPS），每档持续 2 min，记录吞吐量、错误率、P99。

| 端点 | 目标 RPS | 实际 RPS | 错误率 | P50 (ms) | P99 (ms) | DB 连接池占用 | 状态 |
|------|---------|---------|--------|----------|----------|-------------|------|
| `/inventory/stocks` | 500 | | | | | / | |
| `/sales-orders` | 300 | | | | | / | |
| `/dye-batches` | 300 | | | | | / | |

### 6.1 资源瓶颈识别

- **DB 连接池**：上限 `max_connections`（见 `config.yaml`），压测期是否打满？
- **Tokio 阻塞**：是否有同步 IO 阻塞 runtime 线程？（建议 `tokio-console` 排查）
- **事件总线**：Kafka 模式下投递延迟是否反压？（见 `EVENT_BUS.publish` 日志）
- **限流熔断**：`rate_limit` / `circuit_breaker` 中间件是否误触发？

**结论**：（填：最大稳定吞吐 / 瓶颈点 / 容量规划建议）

---

## 7. 结论与下一步

### 7.1 总体达标情况

| 维度 | 达标 | 超阈值 | 未测试 |
|------|------|--------|--------|
| API 响应 | | | |
| 数据库 | | | |
| 内存 | | | |
| 并发 | | | |

### 7.2 超阈值项优化计划

| 序号 | 超阈值项 | 当前值 | 目标值 | 优化方案 | 负责人 | 预计批次 |
|------|---------|--------|--------|---------|--------|---------|
| 1 | | | | | | |

### 7.3 下次测试计划

- 触发条件：（如：重大版本发布 / 数据量增长 2x / 架构调整后）
- 下次执行日期：YYYY-MM-DD

---

## 附录 A：测试命令参考

```bash
# API 响应时间（wrk 示例）
wrk -t4 -c50 -d30s --latency http://localhost:8080/inventory/stocks

# 慢查询采集
psql -h <host> -d <db> -f backend/scripts/p2-2-slow-query.sql

# Criterion 基准（需启用 bench feature）
cargo bench --features bench --bench inventory_calculation_bench

# 内存采样
while true; do grep VmRSS /proc/$(pgrep -f "target/release/server")/status; sleep 30; done
```

## 附录 B：相关报告

- 前端性能报告：[`frontend/scripts/p2-3-perf-report.md`](../../frontend/scripts/p2-3-perf-report.md)
- 慢查询采集脚本：[`backend/scripts/p2-2-slow-query.sql`](p2-2-slow-query.sql)
- 性能基准：[`backend/benches/`](../benches/)
- Prometheus 指标：`/metrics` 端点（见 `backend/src/middleware/metrics.rs`）
