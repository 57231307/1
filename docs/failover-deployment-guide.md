# 冰溪 ERP P0-2 主备隔离部署指南

> **版本**: v1.0
> **更新时间**: 2026-06-16
> **适用范围**: 冰溪 ERP P0-2 主备隔离 TEST 测试版本
> **目标环境**: Docker / Docker Compose

---

## 1. 概述

P0-2 主备隔离模块为冰溪 ERP 核心功能（数据库 / 缓存）提供自动故障转移能力。本指南介绍如何部署和测试该模块。

### 1.1 核心特性

- **数据库主备隔离**：PostgreSQL 主库 + 备库自动切换
- **缓存主备隔离**：Redis 主缓存 + 进程内 moka LRU 备用
- **统一抽象接口**：`FailoverCall` trait，P1/P2 阶段可复用
- **熔断器保护**：连续 5 次失败自动熔断，30s 后半开探测
- **自动回切**：主调用恢复后自动回切（< 30s）
- **完整监控**：5 个 Prometheus 指标 + 4 条告警规则
- **故障注入测试**：9 个 chaos test 场景

### 1.2 范围

- ✅ **P0**：数据库 + 缓存（本次实现）
- ⏳ **P1**：MQ + 存储 + 短信 + 邮件
- ⏳ **P2**：搜索引擎 + OCR

---

## 2. 环境要求

### 2.1 硬件

| 资源 | 最低 | 推荐 |
|------|------|------|
| CPU | 2 核 | 4 核 |
| 内存 | 4 GB | 8 GB |
| 磁盘 | 20 GB | 50 GB |

### 2.2 软件

| 软件 | 版本 | 说明 |
|------|------|------|
| Docker | 20.10+ | 容器运行时 |
| Docker Compose | 2.0+ | 容器编排 |
| PostgreSQL | 16+ | 主备数据库 |
| Redis | 7+ | 主缓存 |

### 2.3 网络

- 应用监听 `:8080`
- PostgreSQL 主库：`5432`
- PostgreSQL 备库：`5432`
- Redis：`6379`
- Prometheus：`9090`（可选）
- Grafana：`3000`（可选）

---

## 3. 部署步骤

### 3.1 获取部署包

```bash
# 从仓库获取 dist/test-version-P0-2/
cd /workspace/dist/test-version-P0-2/
```

### 3.2 配置环境变量

```bash
# 复制配置模板
cp config/failover.toml.example config/failover.toml

# 编辑配置（可选）
vim config/failover.toml
```

环境变量（必须设置）：

```bash
# 主库
export POSTGRES_PRIMARY_PASSWORD=your_secure_password_here
export DATABASE_URL_PRIMARY=postgresql://user:your_secure_password_here@postgres-primary:5432/bingxi

# 备库
export POSTGRES_BACKUP_PASSWORD=your_secure_password_here
export DATABASE_URL_BACKUP=postgresql://user:your_secure_password_here@postgres-backup:5432/bingxi

# Redis
export REDIS_URL=redis://redis-primary:6379

# JWT 密钥
export JWT_SECRET=$(openssl rand -hex 32)
```

### 3.3 一键启动

```bash
# 启动所有服务
./start.sh

# 或手动启动
docker-compose up -d

# 查看启动状态
docker-compose ps
```

### 3.4 验证部署

```bash
# 1. 检查应用健康
curl http://localhost:8080/health

# 2. 检查主备状态
curl http://localhost:8080/api/v1/erp/admin/failover/status

# 3. 检查 Prometheus 指标
curl http://localhost:8080/api/v1/erp/admin/failover/metrics
```

预期输出（status）：

```json
{
  "statuses": [
    {
      "function_name": "database",
      "current_state": "primary",
      "circuit_state": "closed",
      "consecutive_failures": 0,
      "total_primary_calls": 0,
      "total_backup_calls": 0,
      "total_switches": 0
    },
    {
      "function_name": "cache",
      "current_state": "primary",
      "circuit_state": "closed",
      "consecutive_failures": 0,
      "total_primary_calls": 0,
      "total_backup_calls": 0,
      "total_switches": 0
    }
  ]
}
```

---

## 4. 配置文件说明

### 4.1 config/failover.toml

```toml
[database]
# 主库 URL
primary_url = "postgresql://user:password@postgres-primary:5432/bingxi"
# 备库 URL
backup_url = "postgresql://user:password@postgres-backup:5432/bingxi"
# 主调用超时（毫秒）
primary_timeout_ms = 3000
# 备用调用超时（毫秒）
backup_timeout_ms = 5000
# 熔断阈值（连续失败次数）
circuit_breaker_threshold = 5
# 熔断时长（秒）
circuit_breaker_duration_s = 30

[cache]
# Redis 主缓存 URL
primary_url = "redis://redis-primary:6379"
# 进程内 LRU 最大条目数
backup_max_entries = 10000
# 主调用超时（毫秒）
primary_timeout_ms = 1000
# 备用调用超时（毫秒，0 表示无超时）
backup_timeout_ms = 0

[monitoring]
# 是否启用指标
metrics_enabled = true
# 日志级别
log_level = "info"
```

### 4.2 docker-compose.yml

包含 4 个服务：

1. `postgres-primary`：主库
2. `postgres-backup`：备库
3. `redis-primary`：Redis 主缓存
4. `app`：冰溪 ERP 后端

### 4.3 启动脚本

`start.sh` 自动完成：

1. 复制配置模板
2. 检查 Docker / Docker Compose
3. 构建镜像
4. 启动容器
5. 健康检查
6. 输出访问地址

---

## 5. 监控接入

### 5.1 Prometheus 抓取配置

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'bingxi-failover'
    scrape_interval: 10s
    static_configs:
      - targets: ['app:8080']
    metrics_path: '/api/v1/erp/admin/failover/metrics'
```

### 5.2 5 个核心指标

| 指标名 | 类型 | 说明 |
|--------|------|------|
| `failover_primary_total` | Counter | 主调用总次数 |
| `failover_primary_failed_total` | Counter | 主调用失败总次数 |
| `failover_backup_total` | Counter | 备用调用总次数 |
| `failover_switch_total` | Counter | 切换总次数 |
| `failover_circuit_state` | Gauge | 熔断器状态（0/1/2） |

### 5.3 4 条告警规则

| 规则 | 阈值 | 级别 |
|------|------|------|
| FailoverSwitchFrequent | > 5/h | P2 |
| FailoverBackupFailureRate | > 10% | P1 |
| FailoverCircuitOpenLong | > 5min | P1 |
| FailoverBothDown | 任意时长 | P0 |

### 5.4 Grafana Dashboard

导入 `monitoring-dashboard.json`：

```bash
# 在 Grafana 界面
# 1. Dashboards -> Import
# 2. 上传 monitoring-dashboard.json
# 3. 选择 Prometheus 数据源
# 4. 导入
```

---

## 6. 故障注入测试

详见 [`../chaos-test-scenarios.md`](../chaos-test-scenarios.md)

### 6.1 快速测试

```bash
# 场景 1：主库连接拒绝
docker exec app iptables -A OUTPUT -p tcp --dport 5432 -j DROP
sleep 20
curl http://localhost:8080/api/v1/erp/admin/failover/status
docker exec app iptables -D OUTPUT -p tcp --dport 5432 -j DROP
```

### 6.2 场景 2：Redis 不可用

```bash
docker stop redis-primary
sleep 20
curl http://localhost:8080/api/v1/erp/admin/failover/status
docker start redis-primary
```

---

## 7. API 端点

### 7.1 状态查询

```http
GET /api/v1/erp/admin/failover/status
```

返回所有主备状态和最近 20 条事件。

### 7.2 Prometheus 指标

```http
GET /api/v1/erp/admin/failover/metrics
```

返回 Prometheus 文本格式。

### 7.3 手动切换

```http
POST /api/v1/erp/admin/failover/test/switch
Content-Type: application/json

{
  "function": "database"  // 或 "cache"
}
```

### 7.4 健康检查

```http
GET /api/v1/erp/admin/failover/health
```

返回当前主备状态摘要。

---

## 8. 故障排查

### 8.1 应用无法启动

```bash
# 查看应用日志
docker-compose logs app

# 检查数据库连接
docker-compose exec app env | grep DATABASE_URL
```

### 8.2 切换不生效

```bash
# 检查熔断器状态
curl http://localhost:8080/api/v1/erp/admin/failover/status | jq

# 检查 Prometheus 指标
curl http://localhost:8080/api/v1/erp/admin/failover/metrics | grep circuit_state
```

### 8.3 监控指标缺失

```bash
# 验证指标端点
curl http://localhost:8080/api/v1/erp/admin/failover/metrics

# 检查 Prometheus 抓取
curl http://prometheus:9090/api/v1/targets
```

---

## 9. 升级与回滚

### 9.1 升级

```bash
# 1. 拉取最新镜像
docker-compose pull

# 2. 重启服务
docker-compose up -d

# 3. 验证
curl http://localhost:8080/health
```

### 9.2 回滚

```bash
# 1. 停止当前版本
docker-compose down

# 2. 切换到上一版本
git checkout v0.X

# 3. 重启
docker-compose up -d
```

---

## 10. 验收清单

- [ ] 4 个 Docker 服务全部启动
- [ ] `GET /api/v1/erp/admin/failover/status` 返回 200
- [ ] 5 个 Prometheus 指标可抓取
- [ ] Grafana dashboard 导入成功
- [ ] 9 个故障注入场景全部通过
- [ ] 告警规则按级别触发
- [ ] 切换延迟 < 100ms
- [ ] 回切延迟 < 30s

---

## 11. 联系与支持

- **文档**：`/workspace/docs/`
- **设计报告**：`/workspace/docs/superpowers/reports/2026-06-16-failover-design.md`
- **设计 spec**：`/workspace/docs/superpowers/specs/2026-06-16-failover-isolation-design.md`
- **实施 plan**：`/workspace/docs/superpowers/plans/2026-06-16-failover-isolation-plan.md`

---

**版本**: v1.0
**最后更新**: 2026-06-16
**维护者**: 冰溪 ERP 团队
