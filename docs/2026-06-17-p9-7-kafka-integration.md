# P9-7 Kafka 集成报告

> 创建日期：2026-06-17
> 范围：业务事件总线（Kafka + Redis 双轨）
> 3 核心 topic + Producer + Consumer

## 一、目标

P9-7 任务为冰溪 ERP 后端引入 **Apache Kafka** 作为跨服务事件总线，实现：

1. **3 个核心 topic**（按业务域拆分）
2. **Producer**：业务事件发布
3. **Consumer**：业务事件订阅
4. **与 Redis pub/sub 并存**（双轨制：进程内 Redis / 跨服务 Kafka）

## 二、Topic 设计

| Topic | 业务域 | 分区数 | 副本数 | 事件类型数 |
|-------|--------|--------|--------|------------|
| `erp.sales.events` | 销售 | 3 | 1 | 4 |
| `erp.purchase.events` | 采购 | 3 | 1 | 3 |
| `erp.inventory.events` | 库存 | 3 | 1 | 4 |

### 事件类型清单（共 11 种）

**销售（4）**：
- `SalesOrderCreated` — 销售订单创建
- `SalesOrderApproved` — 销售订单审批
- `SalesOrderShipped` — 销售订单发货
- `SalesPaymentReceived` — 销售收款

**采购（3）**：
- `PurchaseOrderCreated` — 采购订单创建
- `PurchaseOrderReceived` — 采购订单到货
- `PurchasePaymentMade` — 采购付款

**库存（4）**：
- `InventoryInbound` — 库存入库
- `InventoryOutbound` — 库存出库
- `InventoryTransfer` — 库存调拨
- `InventoryCount` — 库存盘点

## 三、模块设计

### 1. `backend/src/messaging/kafka.rs` — Kafka 核心

提供：
- `topics` — 3 个 topic 常量
- `EventType` — 11 种事件类型 + topic 映射 + 中文描述
- `BusinessEvent` — 业务事件结构（含 trace_id 关联）
- `KafkaMessage` — Kafka 消息结构（key/value/headers）
- `MessagingProvider` trait — 抽象 Redis/Kafka
- `KafkaProducer` — Producer 实现（mock + real）
- `KafkaConsumer` — Consumer 配置

### 2. `backend/src/messaging/bus.rs` — 事件总线

提供：
- `EventHandler` trait — 事件处理器
- `EventBus` — 业务层 API（publish + register）

### 3. `backend/src/messaging/mod.rs` — 模块导出

## 四、双轨制设计

| 维度 | Redis pub/sub | Kafka |
|------|---------------|-------|
| 范围 | 进程内 | 跨服务 |
| 持久化 | 无 | 7 天（可配） |
| 重放 | 不可 | 可（offset 复位） |
| 性能 | 极快 | 较快（毫秒级） |
| 适用 | 进程内通知 | 跨服务事件流 |

业务层通过 `MessagingProvider` trait 抽象，**同一套 API 可在 Redis/Kafka 间无缝切换**。

## 五、Kafka 消息结构

```rust
KafkaMessage {
    key: "<aggregate_id>",       // 例："SO-20260617-0001"
    value: Vec<u8>,              // JSON 序列化的 BusinessEvent
    headers: {
        "event_type": "销售订单创建",
        "tenant_id": "tenant-001",
        "event_id": "evt-xxxx",
    }
}
```

**关键设计**：
- `key = aggregate_id`：保证同一聚合的事件进入同一分区（保证顺序）
- `headers.tenant_id`：多租户隔离
- `headers.event_id`：幂等处理
- `trace_id`：与 OpenTelemetry trace 关联

## 六、单元测试覆盖

| 模块 | 测试数 |
|------|--------|
| `kafka::tests` | 13 |
| `bus::tests` | 2 |
| **合计** | **15 测试** |

覆盖：
- Topic 常量正确性
- EventType 与 topic 映射
- EventType 中文描述
- BusinessEvent 构造 + trace_id
- KafkaMessage 序列化（headers）
- Producer mock publish
- Consumer 配置 + offset 策略
- EventBus 注册

## 七、部署：`deploy/kafka/docker-compose.yml`

包含 4 个服务：

| 服务 | 端口 | 用途 |
|------|------|------|
| **Zookeeper** | 2181 | Kafka 协调服务 |
| **Kafka** | 9092 | Broker（PLAINTEXT） |
| **Kafka UI** | 8081 | Web 管理界面 |
| **Kafka Init** | — | 自动创建 3 个 topic |

启动：
```bash
cd deploy/kafka
docker-compose up -d

# 访问 Kafka UI
open http://localhost:8081

# 验证 topic
docker exec bx-kafka kafka-topics --list --bootstrap-server localhost:9092
```

## 八、启用 rdkafka（可选）

默认情况下，本模块提供 **mock 实现**（不引入重依赖）。

要启用真实 Kafka，添加：

```toml
# backend/Cargo.toml
rdkafka = { version = "0.36", features = ["cmake-build", "ssl-vendored"] }
```

然后在 `main.rs` 中替换 `KafkaProducer::mock()` 为 `KafkaProducer::real(brokers)`。

## 九、约束遵守

- ✅ **零硬编码**：broker 地址从环境变量读
- ✅ **多租户隔离**：消息 headers 携带 tenant_id
- ✅ **无破坏性变更**：messaging 模块为新增
- ✅ **可选依赖**：rdkafka 不在默认依赖中
- ✅ **资源属性标准化**：与 OpenTelemetry trace 关联

## 十、风险评估

| 风险 | 等级 | 缓解措施 |
|------|------|----------|
| Kafka 部署失败 | 中 | docker-compose 一键启动 |
| rdkafka 编译失败 | 中 | 默认不引入，提供 mock |
| 消息丢失 | 低 | acks=all + 重试机制（生产环境） |
| 顺序错乱 | 极低 | 按 aggregate_id 分区 |
| 重复消费 | 低 | consumer 端幂等（event_id） |
