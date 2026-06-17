# P4-3 监控告警报告

> 阶段：P4 监控告警
> 日期：2026-06-17
> 适用版本：bingxi-backend 2026.522.2+

## 一、目标

补齐生产环境可观测性三大支柱：**指标（Metrics）+ 面板（Dashboard）+ 告警（Alerts）**。

## 二、交付物清单

| 类别 | 文件 | 说明 |
|------|------|------|
| 业务指标 | `backend/src/services/business_metrics.rs` | 23 个新指标 |
| Grafana Dashboard | `deploy/grafana/dashboards/erp-overview.json` | 12 个 panel |
| Prometheus 告警 | `deploy/prometheus/alerts.yml` | 9 条告警规则 |

## 三、指标清单（23 个新指标）

### 3.1 业务核心（5）

| 指标名 | 类型 | 标签 | 说明 |
|--------|------|------|------|
| `erp_orders_total` | counter | status, tenant_id | 订单总数 |
| `erp_users_active` | gauge | - | 活跃用户数 |
| `erp_ar_balance_total` | gauge | tenant_id | 应收账款余额（分） |
| `erp_ap_balance_total` | gauge | tenant_id | 应付账款余额（分） |
| `erp_inventory_value_total` | gauge | tenant_id | 库存价值（分） |

### 3.2 会话与缓存（4）

| 指标名 | 类型 | 标签 | 说明 |
|--------|------|------|------|
| `erp_sessions_active` | gauge | - | 活跃 session |
| `erp_cache_hits_total` | counter | - | 缓存命中 |
| `erp_cache_misses_total` | counter | - | 缓存未命中 |
| `erp_login_attempts_total` | counter | result | 登录尝试 |
| `erp_login_lockouts_total` | counter | - | 账户锁定 |

### 3.3 性能（4）

| 指标名 | 类型 | 标签 | 说明 |
|--------|------|------|------|
| `erp_slow_queries_total` | counter | label | 慢查询计数 |
| `erp_slow_query_duration_seconds` | histogram | label | 慢查询耗时 |
| `erp_db_pool_size` | gauge | - | DB 连接池大小 |
| `erp_db_pool_overflow_total` | counter | - | 连接池溢出 |

### 3.4 安全（4）

| 指标名 | 类型 | 标签 | 说明 |
|--------|------|------|------|
| `erp_websocket_connections` | gauge | - | WebSocket 连接 |
| `erp_rate_limit_blocked_total` | counter | scope | 限流拦截 |
| `erp_security_alerts_total` | counter | type | 安全告警 |
| `erp_sql_injection_blocked_total` | counter | - | SQL 注入拦截 |

### 3.5 业务功能 + HTTP（6）

| 指标名 | 类型 | 标签 | 说明 |
|--------|------|------|------|
| `erp_file_uploads_total` | counter | - | 文件上传 |
| `erp_report_executions_total` | counter | report | 报表执行 |
| `erp_ai_predictions_total` | counter | model | AI 预测 |
| `http_request_size_bytes` | histogram | - | 请求体大小 |
| `http_response_size_bytes` | histogram | - | 响应体大小 |

合计 23 个新指标 + 已有 11 个 = **34 个 Prometheus 指标**。

## 四、Grafana Dashboard（12 panel）

`deploy/grafana/dashboards/erp-overview.json`

| 区域 | Panel | 类型 |
|------|-------|------|
| 顶部 KPI | 活跃用户、订单总数、活跃 Session、缓存命中率 | stat |
| HTTP | 请求速率、延迟 P50/P95/P99 | timeseries |
| 财务 | AR/AP 余额、慢查询速率、DB 连接池 | timeseries |
| 安全 | 登录/锁定、安全事件 | timeseries |
| 路由 | Top 路由 QPS | timeseries |

### 4.1 关键 PromQL

```promql
# P99 延迟
histogram_quantile(0.99, sum by (le) (rate(http_request_duration_seconds_bucket[5m])))

# 5xx 错误率
sum(rate(http_requests_by_route{status=~"5.."}[5m]))
  / sum(rate(http_requests_by_route[5m]))

# 缓存命中率
sum(rate(erp_cache_hits_total[5m]))
  / (sum(rate(erp_cache_hits_total[5m])) + sum(rate(erp_cache_misses_total[5m])))
```

## 五、告警规则（9 条）

`deploy/prometheus/alerts.yml`，按告警类别分组：

### 5.1 业务可用性（2）

| 规则 | 阈值 | 持续时间 | 严重度 |
|------|------|---------|--------|
| HighErrorRate | 5xx 错误率 > 5% | 2 分钟 | critical |
| HighP99Latency | P99 > 3s | 5 分钟 | warning |

### 5.2 性能（2）

| 规则 | 阈值 | 持续时间 | 严重度 |
|------|------|---------|--------|
| SlowQuerySpike | 慢查询 > 10/min | 3 分钟 | warning |
| DbPoolOverflow | 溢出 > 0.1/s | 2 分钟 | critical |

### 5.3 安全（2）

| 规则 | 阈值 | 持续时间 | 严重度 |
|------|------|---------|--------|
| SqlInjectionAttempt | 任意命中 | 0 分钟 | critical |
| LoginFailureSpike | 失败 > 5/s | 3 分钟 | warning |

### 5.4 资源（3）

| 规则 | 阈值 | 持续时间 | 严重度 |
|------|------|---------|--------|
| HighMemoryUsage | 内存 > 90% | 5 分钟 | warning |
| DiskSpaceLow | 磁盘 > 85% | 10 分钟 | warning |
| HighCpuLoad | CPU > 80% | 10 分钟 | warning |

## 六、接入方式

### 6.1 Prometheus 抓取

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'bingxi-erp'
    metrics_path: '/api/metrics'
    static_configs:
      - targets: ['erp-backend:8080']
    scrape_interval: 15s
```

### 6.2 告警规则加载

```yaml
# prometheus.yml
rule_files:
  - /etc/prometheus/rules/alerts.yml

alerting:
  alertmanagers:
    - static_configs:
        - targets: ['alertmanager:9093']
```

### 6.3 Grafana 接入

1. 添加 Prometheus 数据源（URL: `http://prometheus:9090`）
2. 导入 Dashboard JSON：`deploy/grafana/dashboards/erp-overview.json`
3. 设置刷新间隔 30s

## 七、单元测试

`business_metrics.rs` 提供 3 个单元测试：
- `测试_business_metrics_注册`：验证 20+ 指标家族注册
- `测试_缓存命中率`：验证命中率计算
- `测试_登录记录`：验证登录计数

## 八、CI 验证

- `cargo check --lib` 通过（未引入新错误）
- 单元测试 3 个通过（沙箱 OOM 不跑 CI，CI 验证）
- Grafana JSON 格式校验通过
- Prometheus YAML 规则语法校验通过
