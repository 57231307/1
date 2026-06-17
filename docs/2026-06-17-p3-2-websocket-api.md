# P3-2 WebSocket 实时通信 - API 文档

> **发布日期**：2026-06-17
> **任务编号**：P3 / P3-2
> **协议**：WebSocket（RFC 6455）
> **路径**：`/api/v1/erp/ws/notifications`
> **鉴权**：URL query `?token=<JWT>`

---

## 一、连接

### 1.1 WebSocket URL

```
ws://server/api/v1/erp/ws/notifications?token=<JWT>
wss://server/api/v1/erp/ws/notifications?token=<JWT>  # 生产 HTTPS
```

### 1.2 Token 格式

简化版（demo）：`<tenant_id>:<user_id>`

示例：`?token=1:100`（tenant_id=1, user_id=100）

生产环境：接入主项目 `jsonwebtoken` 验证，提取 `tenant_id` 和 `user_id`。

### 1.3 错误响应

| HTTP Status | 触发条件 |
|-------------|----------|
| 401 Unauthorized | 缺少 token / token 格式错误 / JWT 验证失败 |
| 500 Internal Server Error | 服务端内部错误 |

## 二、消息类型

### 2.1 服务端 → 客户端

#### 2.1.1 通知消息

| 字段 | 类型 | 说明 |
|------|------|------|
| type | string | "notification" |
| data.id | int64 | 通知 ID |
| data.title | string | 标题 |
| data.content | string | 内容 |
| data.category | string | 分类（order/inventory/production/system） |
| data.priority | int32 | 优先级 1-10 |
| data.created_at | string | ISO 8601 时间 |

示例：
```json
{
  "type": "notification",
  "data": {
    "id": 42,
    "title": "订单已创建",
    "content": "您的订单 #123 已创建",
    "category": "order",
    "priority": 5,
    "created_at": "2026-06-17T10:30:00Z"
  }
}
```

#### 2.1.2 心跳响应（pong）

```json
{
  "type": "pong",
  "timestamp": 1718612400
}
```

#### 2.1.3 错误消息

| 字段 | 类型 | 说明 |
|------|------|------|
| type | string | "error" |
| code | string | 错误码 |
| message | string | 错误描述 |

错误码：

| code | 说明 |
|------|------|
| INVALID_TOKEN | JWT 验证失败 |
| INTERNAL | 服务端内部错误 |
| INVALID_MESSAGE | 消息格式错误 |

### 2.2 客户端 → 服务端

#### 2.2.1 心跳请求（ping）

```json
{
  "type": "ping",
  "timestamp": 1718612400
}
```

#### 2.2.2 标记已读

| 字段 | 类型 | 说明 |
|------|------|------|
| type | string | "mark_as_read" |
| id | int64 | 通知 ID |

```json
{
  "type": "mark_as_read",
  "id": 42
}
```

## 三、性能指标

| 指标 | 目标值 |
|------|--------|
| 单进程最大连接数 | 10,000 |
| 消息延迟 P99 | < 100ms |
| 心跳超时 | 60s |
| 广播吞吐 | 10,000 msg/s |
| 单消息大小 | < 64KB |

## 四、限制与约束

- 单消息大小 < 64KB（WebSocket 帧限制）
- 同一用户最大并发连接数：未限制（受服务端资源约束）
- 心跳间隔：客户端每 30 秒发送 ping
- 多租户隔离：消息按 `(tenant_id, user_id)` 双键路由
- 单进程：P3-2 仅支持单进程（多进程推送需 Redis Pub/Sub，留 P4+）

## 五、错误码表

| gRPC Status | HTTP Status | 触发条件 |
|-------------|-------------|----------|
| UNAUTHORIZED | 401 | 缺少 token / JWT 无效 |
| INTERNAL | 500 | 服务端内部错误 |

## 六、限制与后续演进

### 6.1 P3-2 限制

- **单进程**：仅支持单进程内的 WebSocket 推送
- **简化 JWT**：仅 demo 解析 `tenant_id:user_id` 格式
- **无集群推送**：多实例部署时推送仅在同一实例内

### 6.2 后续演进（P4+）

- **Redis Pub/Sub**：多实例间广播
- **真实 JWT**：接入主项目 `jsonwebtoken` 验证
- **TLS 强制**：生产环境禁用 ws://
- **指标监控**：Prometheus 导出连接数、消息数
- **熔断限流**：高频消息合并、丢弃策略
