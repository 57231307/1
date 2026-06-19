# P4-7 Chaos Test 用例（生产可执行）

> 配合 `2026-06-17-p4-7-disaster-recovery.md` 使用
> 本文件为自动化 Chaos 测试脚本（Chaos Mesh / toxiproxy）

## 一、Chaos Mesh 用例

### 1.1 数据库主库宕机

```yaml
# chaos-db-failover.yaml
apiVersion: chaos-mesh.org/v1alpha1
kind: PodChaos
metadata:
  name: db-primary-fail
  namespace: chaos-mesh
spec:
  action: pod-failure
  mode: one
  selector:
    namespaces:
      - erp
    labelSelectors:
      app: postgres
      role: primary
  duration: "5m"
  scheduler:
    cron: "@every 4h"
```

### 1.2 Redis 不可用

```yaml
# chaos-redis-down.yaml
apiVersion: chaos-mesh.org/v1alpha1
kind: PodChaos
metadata:
  name: redis-kill
  namespace: chaos-mesh
spec:
  action: pod-kill
  mode: one
  selector:
    namespaces:
      - erp
    labelSelectors:
      app: redis
  duration: "3m"
  scheduler:
    cron: "@every 6h"
```

### 1.3 网络延迟注入

```yaml
# chaos-network-delay.yaml
apiVersion: chaos-mesh.org/v1alpha1
kind: NetworkChaos
metadata:
  name: network-delay
  namespace: chaos-mesh
spec:
  action: delay
  mode: all
  selector:
    namespaces:
      - erp
    labelSelectors:
      app: erp-backend
  delay:
    latency: "500ms"
    correlation: "100"
    jitter: "50ms"
  duration: "10m"
  direction: to
  target:
    selector:
      namespaces:
        - erp
      labelSelectors:
        app: postgres
```

## 二、toxiproxy 用例（开发环境）

### 2.1 启动 toxiproxy

```yaml
# docker-compose.yml (追加)
services:
  toxiproxy:
    image: ghcr.io/shopify/toxiproxy:2.9.0
    ports:
      - "8474:8474"  # 管理端口
      - "25432:25432"  # PG 代理
    volumes:
      - ./toxiproxy.json:/config/toxiproxy.json
```

### 2.2 toxiproxy.json 配置

```json
[
  {
    "name": "postgres",
    "listen": "0.0.0.0:25432",
    "upstream": "postgres:5432",
    "enabled": true
  },
  {
    "name": "redis",
    "listen": "0.0.0.0:26379",
    "upstream": "redis:6379",
    "enabled": true
  }
]
```

### 2.3 故障注入 CLI

```bash
# 数据库延迟
curl -X POST http://localhost:8474/proxies/postgres/toxics \
  -H "Content-Type: application/json" \
  -d '{"type":"latency","attributes":{"latency":1000}}'

# Redis 断连
curl -X POST http://localhost:8474/proxies/redis/toxics \
  -H "Content-Type: application/json" \
  -d '{"type":"timeout","attributes":{"timeout":0}}'

# 清除故障
curl -X DELETE http://localhost:8474/proxies/postgres/toxics/latency
```

## 三、应用内置 chaos admin API

`/api/admin/chaos/*`（仅在 `CHAOS_ENABLED=true` 时启用）

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/admin/chaos/inject-slow-query` | POST | 注入指定耗时的慢查询 |
| `/api/admin/chaos/slow-query-stats` | GET | 查询注入历史 |
| `/api/admin/chaos/inject-error` | POST | 注入指定状态码的错误 |
| `/api/admin/chaos/clear` | DELETE | 清除所有故障 |

## 四、CI 集成

`.github/workflows/chaos-test.yml`：

```yaml
name: Chaos Test
on:
  schedule:
    - cron: '0 2 * * 0'  # 每周日凌晨
  workflow_dispatch:  # 手动触发

jobs:
  chaos:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Start services
        run: docker-compose up -d
      - name: Run chaos scenarios
        run: ./scripts/chaos-test.sh
      - name: Collect results
        run: ./scripts/chaos-report.sh
```

## 五、混沌工程原则

1. **从小爆炸开始**：先在测试环境验证，再到预发，最后到生产
2. **定义稳态**：明确"系统正常"的指标（如错误率 < 0.1%）
3. **最小爆炸半径**：每次只注入一个故障
4. **持续运行**：把混沌测试纳入 CI/CD
5. **记录一切**：每次故障注入的结果、发现、改进都要留档
