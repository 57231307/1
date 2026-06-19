# 故障注入测试用例（P0-2 主备隔离）

> **测试目的**: 验证 P0-2 主备隔离模块在主调用不可用时能自动切换至备用，并在主调用恢复后自动回切。
> **测试时间**: 2026-06-16
> **测试策略**: 集成测试 + 关键路径故障注入
> **执行者**: 运维 / QA

---

## 前置条件

1. 部署 P0-2 TEST 测试版本：`docker-compose up -d`
2. PostgreSQL 主备 + Redis 主 + 应用均启动
3. 应用监听 `http://localhost:8080`
4. 监控面板：`http://localhost:8080/admin/failover`

---

## 场景 1：主库连接拒绝

**目的**: 验证主库不可达时自动切换到备库

**步骤**:

```bash
# 1. 启动应用
docker-compose up -d app

# 2. 验证主库连接正常
curl http://localhost:8080/api/v1/erp/admin/failover/status | jq

# 3. 注入故障：拒绝主库连接
docker exec app iptables -A OUTPUT -p tcp --dport 5432 -j DROP

# 4. 等待 5 次失败（约 15s）
sleep 20

# 5. 验证状态
curl http://localhost:8080/api/v1/erp/admin/failover/status | jq
# 预期：database.current_state = "backup"

# 6. 检查指标
curl http://localhost:8080/api/v1/erp/admin/failover/metrics | grep failover_switch
# 预期：failover_switch_total{function="database"} >= 1

# 7. 清理：恢复主库连接
docker exec app iptables -D OUTPUT -p tcp --dport 5432 -j DROP

# 8. 等待 30s 熔断超时 + 半开探测
sleep 35

# 9. 验证自动回切
curl http://localhost:8080/api/v1/erp/admin/failover/status | jq
# 预期：database.current_state = "primary"
```

**通过标准**:
- [ ] 主库被拒绝后，5 次失败内自动切到备库
- [ ] `failover_switch_total{function="database"}` ≥ 1
- [ ] 恢复连接后 30s 内自动回切到主库

---

## 场景 2：主库连接超时

**目的**: 验证主库响应慢时超时切换

**步骤**:

```bash
# 1. 启动应用
docker-compose up -d app

# 2. 注入网络延迟（5s）
docker exec app tc qdisc add dev eth0 root netem delay 5000ms

# 3. 触发数据库调用
curl http://localhost:8080/api/v1/erp/admin/failover/status

# 4. 验证状态
curl http://localhost:8080/api/v1/erp/admin/failover/status | jq
# 预期：database.current_state = "backup"

# 5. 清理：移除延迟
docker exec app tc qdisc del dev eth0 root

# 6. 等待回切
sleep 35
```

**通过标准**:
- [ ] 主库延迟 5s 时自动切到备库（不等待 5s）
- [ ] 主库恢复后自动回切

---

## 场景 3：Redis 不可用

**目的**: 验证 Redis 不可用时切到进程内 LRU

**步骤**:

```bash
# 1. 启动应用
docker-compose up -d app

# 2. 停止 Redis
docker stop redis-primary

# 3. 等待熔断
sleep 20

# 4. 验证状态
curl http://localhost:8080/api/v1/erp/admin/failover/status | jq
# 预期：cache.current_state = "backup"，backup_type = "lru"

# 5. 重启 Redis
docker start redis-primary

# 6. 等待回切
sleep 35
```

**通过标准**:
- [ ] Redis 停止后切到进程内 LRU
- [ ] 应用功能不中断（缓存读写仍可用）
- [ ] Redis 恢复后自动回切

---

## 场景 4：熔断后探测回切

**目的**: 验证半开状态探测后自动回切

**步骤**:

```bash
# 1. 启动应用
docker-compose up -d app

# 2. 手动触发切换
curl -X POST http://localhost:8080/api/v1/erp/admin/failover/test/switch \
  -H "Content-Type: application/json" \
  -d '{"function":"cache"}'

# 3. 验证状态已切换
curl http://localhost:8080/api/v1/erp/admin/failover/status | jq
# 预期：cache.current_state = "backup"，circuit_state = "open"

# 4. 等待 30s 熔断超时
sleep 30

# 5. 触发一次健康检查（模拟半开探测）
curl http://localhost:8080/api/v1/erp/admin/failover/health

# 6. 验证自动回切
curl http://localhost:8080/api/v1/erp/admin/failover/status | jq
# 预期：cache.current_state = "primary"，circuit_state = "closed"
```

**通过标准**:
- [ ] 手动切换后状态正确更新
- [ ] 30s 熔断后自动进入半开
- [ ] 探测成功后自动回切

---

## 场景 5：主备同时不可用

**目的**: 验证双不可用时正确返回错误

**步骤**:

```bash
# 1. 启动应用
docker-compose up -d app

# 2. 同时停止主备库
docker stop postgres-primary
docker stop postgres-backup

# 3. 触发调用
curl http://localhost:8080/api/v1/erp/admin/failover/health

# 4. 验证错误
# 预期：HTTP 500 + BothFailed 错误
# 验证指标
curl http://localhost:8080/api/v1/erp/admin/failover/metrics | grep failover_primary_failed
# 预期：failover_primary_failed_total 持续增加

# 5. 重启主备库
docker start postgres-primary
docker start postgres-backup

# 6. 等待恢复
sleep 35
```

**通过标准**:
- [ ] 双不可用时返回 BothFailed 错误
- [ ] 状态变为 both_down
- [ ] 恢复后自动回切

---

## 场景 6：高频切换（告警测试）

**目的**: 验证高频切换触发 P2 告警

**步骤**:

```bash
# 1. 启动应用
docker-compose up -d app

# 2. 反复触发切换（10 次）
for i in {1..10}; do
  curl -X POST http://localhost:8080/api/v1/erp/admin/failover/test/switch \
    -H "Content-Type: application/json" \
    -d '{"function":"database"}'
  sleep 60
done

# 3. 检查告警
# Prometheus 告警：FailoverSwitchFrequent (> 5/h)
```

**通过标准**:
- [ ] 切换次数 > 5/h 时触发 P2 告警
- [ ] 告警规则 `FailoverSwitchFrequent` 触发

---

## 场景 7：备用调用失败率（告警测试）

**目的**: 验证备用失败率 > 10% 触发 P1 告警

**步骤**:

```bash
# 1. 启动应用，但只配置主库（无备库）
# 模拟方法：先停备库
docker stop postgres-backup

# 2. 注入主库故障
docker exec app iptables -A OUTPUT -p tcp --dport 5432 -j DROP

# 3. 持续触发调用
for i in {1..20}; do
  curl http://localhost:8080/api/v1/erp/admin/failover/health
  sleep 2
done

# 4. 检查告警
# Prometheus 告警：FailoverBackupFailureRate
```

**通过标准**:
- [ ] 备用失败率 > 10% 时触发 P1 告警

---

## 场景 8：熔断器持续打开（告警测试）

**目的**: 验证熔断器打开 > 5min 触发 P1 告警

**步骤**:

```bash
# 1. 启动应用
docker-compose up -d app

# 2. 注入持续故障
docker exec app iptables -A OUTPUT -p tcp --dport 5432 -j DROP

# 3. 等待熔断
sleep 20

# 4. 持续 5 分钟
sleep 300

# 5. 检查告警
# Prometheus 告警：FailoverCircuitOpenLong
```

**通过标准**:
- [ ] 熔断器打开 > 5min 触发 P1 告警

---

## 场景 9：双不可用告警（P0）

**目的**: 验证主备同时不可用触发 P0 告警

**步骤**:

```bash
# 1. 启动应用
docker-compose up -d app

# 2. 同时停止主备
docker stop postgres-primary
docker stop postgres-backup
docker stop redis-primary

# 3. 等待 30s
sleep 30

# 4. 检查告警
# Prometheus 告警：FailoverBothDown
```

**通过标准**:
- [ ] 双不可用立即触发 P0 告警

---

## 验收清单

- [ ] 9 个故障注入场景全部通过
- [ ] 监控指标正确反映主备状态
- [ ] 告警规则按级别触发
- [ ] 切换延迟 < 100ms
- [ ] 回切延迟 < 30s

---

## 测试报告模板

```markdown
## 故障注入测试报告

**测试时间**: YYYY-MM-DD
**测试人员**: XXX
**测试版本**: P0-2 v1.0
**测试环境**: Docker Compose（PostgreSQL 主+备 + Redis 主 + App）

### 场景 1：主库连接拒绝
- 通过：是 / 否
- 切换时间：Xms
- 备注：XXX

### 场景 2：主库连接超时
- 通过：是 / 否
- 备注：XXX

...

### 总结
- 通过场景数：X/9
- 失败场景：XXX
- 改进建议：XXX
```

---

**版本**: v1.0
**最后更新**: 2026-06-16
