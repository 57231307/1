# P4-7 Chaos Test 与灾备演练方案

> 阶段：P4 Chaos 工程 + 灾备
> 日期：2026-06-17
> 适用版本：bingxi-erp 2026.522.2+

## 一、目标

通过主动故障注入验证系统在异常情况下的稳定性，
并建立**可重复的灾备演练机制**，确保 RTO / RPO 目标可达。

## 二、RTO / RPO 目标

| 灾难场景 | RTO（恢复时间） | RPO（数据丢失） |
|---------|----------------|----------------|
| 数据库主库宕机 | **4 小时** | **1 小时**（WAL 流复制） |
| 应用节点全部宕机 | **30 分钟** | 0（HPA 自动扩缩） |
| Redis 不可用 | **5 分钟** | 0（降级到内存缓存） |
| 机房网络中断 | **8 小时** | **30 分钟**（异地灾备） |

## 三、3 个 Chaos 用例

### 3.1 用例 1：数据库主库宕机（chaos-db-failover）

**目标**：验证主库连接断开时 5xx 错误率 < 5%，自动切换到备库。

**故障注入**：
```bash
# 1. 启动应用
docker-compose up -d app

# 2. 模拟主库不可达（关闭主库容器）
docker stop postgres-primary

# 3. 验证自动切换
curl http://localhost:8080/api/admin/failover/status | jq '.status'  # 应为 "standby"
```

**验收标准**：
- [ ] 备库接管时间 < 30s
- [ ] 5xx 错误率 < 5%（1 分钟窗口）
- [ ] 应用进程不崩溃
- [ ] 慢查询指标上升 < 2 倍

**恢复**：
```bash
docker start postgres-primary
# 验证主库自动回切
curl http://localhost:8080/api/admin/failover/status | jq '.status'  # 应为 "primary"
```

### 3.2 用例 2：Redis 不可用（chaos-redis-down）

**目标**：验证 Redis 故障时自动降级到内存缓存，业务不受阻塞。

**故障注入**：
```bash
# 1. 启动应用
docker-compose up -d app

# 2. 模拟 Redis 不可达
docker stop redis

# 3. 验证降级
curl http://localhost:8080/api/health/ready  # 应返回 200（不依赖 Redis）
```

**验收标准**：
- [ ] 应用启动后正常运行
- [ ] `/api/health/ready` 仍返回 200
- [ ] 业务请求成功率 > 95%
- [ ] 缓存层自动降级到 moka 内存缓存
- [ ] 慢查询速率不会显著上升

**恢复**：
```bash
docker start redis
# 缓存自动重建
```

### 3.3 用例 3：慢查询注入（chaos-slow-query）

**目标**：验证慢查询不会拖垮整个应用，慢查询限流生效。

**故障注入**：
```bash
# 1. 通过 API 注入慢查询
curl -X POST http://localhost:8080/api/admin/chaos/inject-slow-query \
  -H "Content-Type: application/json" \
  -d '{"duration_ms": 5000, "label": "test_inject"}'

# 2. 验证限流
curl http://localhost:8080/api/admin/chaos/slow-query-stats
```

**验收标准**：
- [ ] 单次慢查询超过 1s 时被限流中间件截断
- [ ] 其他业务请求不受影响
- [ ] `erp_slow_queries_total` 指标 +1
- [ ] 5xx 错误率不上升
- [ ] 应用不 OOM

**恢复**：
```bash
curl -X DELETE http://localhost:8080/api/admin/chaos/inject-slow-query/test_inject
```

## 四、Chaos 工具链

### 4.1 工具选型

| 工具 | 用途 | 部署位置 |
|------|------|---------|
| Chaos Mesh | K8s 故障注入（生产） | K8s 集群 |
| toxiproxy | TCP 代理故障注入（开发） | docker-compose |
| 自研 chaos admin API | 业务层故障注入 | 应用内置 |

### 4.2 启用 Chaos Mesh

```bash
# 安装 Chaos Mesh（仅在测试环境）
helm install chaos-mesh chaos-mesh/chaos-mesh \
  --namespace chaos-mesh --create-namespace
```

## 五、灾备方案

### 5.1 备份策略

| 备份类型 | 频率 | 保留 | 工具 |
|---------|------|------|------|
| 全量备份 | 每日 02:00 | 30 天 | `pg_dump` |
| 增量备份 | 每小时 | 7 天 | WAL 流复制 |
| 配置文件 | 每次变更 | 永久 | Git |
| 镜像备份 | 每次发布 | 90 天 | 阿里云 ACR |

### 5.2 备份脚本

`/opt/bingxi/backup/backup.sh`：

```bash
#!/bin/bash
set -e

BACKUP_DIR=/data/backup/$(date +%Y%m%d_%H%M%S)
mkdir -p "$BACKUP_DIR"

# 1. PostgreSQL 全量备份
pg_dump -Fc -h postgres-primary -U erp bingxi_erp \
  > "$BACKUP_DIR/db.dump"

# 2. 备份上传到 OSS
ossutil cp "$BACKUP_DIR/db.dump" \
  oss://bingxi-backup/db/$(date +%Y%m%d)/

# 3. 清理 30 天前的本地备份
find /data/backup -mtime +30 -delete

echo "Backup completed: $BACKUP_DIR"
```

### 5.3 恢复演练 Checklist

每季度执行一次：

- [ ] 启动灾备环境（异地机房）
- [ ] 从 OSS 下载最新备份
- [ ] 执行恢复：`pg_restore -d bingxi_erp db.dump`
- [ ] 启动应用，验证健康检查
- [ ] 验证关键业务接口（登录/订单/库存）
- [ ] 验证数据完整性（最近 1 小时订单数）
- [ ] 记录 RTO / RPO 实际值
- [ ] 优化备份/恢复流程

### 5.4 异地灾备架构

```text
┌─────────────────────┐         ┌─────────────────────┐
│  主站（杭州）         │         │  灾备（上海）         │
│  ┌──────────────┐  │  WAL    │  ┌──────────────┐  │
│  │ PostgreSQL   │──│──流复制─▶│  │ PostgreSQL   │  │
│  │ Primary      │  │         │  │ Standby      │  │
│  └──────────────┘  │         │  └──────────────┘  │
│  ┌──────────────┐  │  OSS    │  ┌──────────────┐  │
│  │ Application  │  │  备份   │  │ Application  │  │
│  │ Replica x 2  │  │────────▶│  │ 灾备（待命）   │  │
│  └──────────────┘  │         │  └──────────────┘  │
└─────────────────────┘         └─────────────────────┘
```

## 六、监控灾备健康

### 6.1 关键指标

| 指标 | 健康阈值 | 告警 |
|------|---------|------|
| 主备同步延迟 | < 60s | > 5 分钟 critical |
| 备份成功 | 每日 1 次 | 缺失 critical |
| OSS 备份大小 | 500MB - 5GB | < 100MB warning |
| 灾备环境心跳 | < 5 分钟 | > 10 分钟 warning |

### 6.2 Prometheus 告警

```yaml
- alert: BackupMissing
  expr: time() - bingxi_last_backup_timestamp > 86400  # 24h
  for: 0m
  labels:
    severity: critical
  annotations:
    summary: "数据库备份缺失超过 24 小时"
```

## 七、Runbook

### 7.1 主库宕机

```bash
# 1. 确认主库状态
kubectl get pods -n erp -l app=postgres

# 2. 触发主备切换（自动或手动）
curl -X POST http://promote-api/erp/promote

# 3. 验证应用连接
curl http://localhost:8080/api/health/ready

# 4. 通知值班（PagerDuty）
```

### 7.2 数据损坏恢复

```bash
# 1. 停止应用（避免脏写）
kubectl scale deployment erp --replicas=0

# 2. 从 OSS 下载备份
ossutil get oss://bingxi-backup/db/20260617/db.dump

# 3. 恢复
pg_restore -h postgres-primary -U erp -d bingxi_erp --clean db.dump

# 4. 启动应用
kubectl scale deployment erp --replicas=2
```

## 八、季度演练计划

| 季度 | 演练内容 | 责任人 |
|------|---------|--------|
| Q1 | 主库故障切换 | DBA |
| Q2 | 异地灾备恢复 | SRE |
| Q3 | Redis 故障降级 | 应用 |
| Q4 | 完整灾备切换 | 全员 |

每次演练输出报告：
- 实际 RTO / RPO
- 发现的问题
- 改进措施
