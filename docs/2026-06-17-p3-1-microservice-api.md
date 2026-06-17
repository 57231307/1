# P3-1 微服务拆分 - API 文档

> **发布日期**：2026-06-17
> **任务编号**：P3 / P3-1
> **协议**：gRPC（HTTP/2 + Protocol Buffers）
> **服务**：notifications
> **端口**：50056

---

## 一、protobuf 包

```proto
package notifications;
```

## 二、gRPC 服务定义

```proto
service NotificationService {
  rpc SendNotification(SendNotificationRequest) returns (SendNotificationResponse);
  rpc BatchSend(BatchSendRequest) returns (BatchSendResponse);
  rpc ListUserNotifications(ListRequest) returns (ListResponse);
  rpc MarkAsRead(MarkAsReadRequest) returns (MarkAsReadResponse);
}
```

## 三、RPC 方法详细说明

### 3.1 SendNotification

**功能**：发送单条通知

**请求字段**：

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| tenant_id | int64 | ✅ | 租户 ID（多租户隔离） |
| user_id | int64 | ✅ | 接收用户 ID |
| title | string | ✅ | 通知标题（≤255 字符） |
| content | string | ✅ | 通知内容 |
| category | string | ❌ | 分类，默认 "system"（order/inventory/production/system） |
| priority | int32 | ❌ | 优先级 1-10，默认 5 |

**响应字段**：

| 字段 | 类型 | 说明 |
|------|------|------|
| id | int64 | 通知 ID |
| status | string | "success" / "failed" |

**错误码**：

| gRPC Status | 触发条件 |
|-------------|----------|
| INVALID_ARGUMENT | tenant_id ≤ 0 / user_id ≤ 0 / 标题或内容为空 / 优先级越界 |
| INTERNAL | 数据库插入失败 |

**示例**：

```bash
grpcurl -plaintext -d '{
  "tenant_id": 1,
  "user_id": 100,
  "title": "订单已创建",
  "content": "您的订单 #123 已创建",
  "category": "order",
  "priority": 5
}' localhost:50056 notifications.NotificationService/SendNotification
```

```json
{
  "id": "42",
  "status": "success"
}
```

### 3.2 BatchSend

**功能**：批量发送通知

**请求字段**：

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| items | repeated SendNotificationRequest | ✅ | 通知列表 |

**响应字段**：

| 字段 | 类型 | 说明 |
|------|------|------|
| count | int32 | 成功发送数量 |
| status | string | "success" / "failed" |

**注意**：批量发送**不**保证原子性，单条失败不影响其他。

### 3.3 ListUserNotifications

**功能**：列出用户通知（分页）

**请求字段**：

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| tenant_id | int64 | ✅ | 租户 ID |
| user_id | int64 | ✅ | 用户 ID |
| limit | int32 | ❌ | 每页数量，默认 20，最大 100 |
| offset | int32 | ❌ | 偏移，默认 0 |

**响应字段**：

| 字段 | 类型 | 说明 |
|------|------|------|
| items | repeated NotificationItem | 通知列表（按 created_at DESC 排序）|
| total | int32 | 通知总数 |

**NotificationItem 字段**：

| 字段 | 类型 | 说明 |
|------|------|------|
| id | int64 | 通知 ID |
| tenant_id | int64 | 租户 ID |
| user_id | int64 | 用户 ID |
| title | string | 标题 |
| content | string | 内容 |
| category | string | 分类 |
| priority | int32 | 优先级 |
| is_read | bool | 是否已读 |
| created_at | string | ISO 8601 时间 |

### 3.4 MarkAsRead

**功能**：标记通知已读

**请求字段**：

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | int64 | ✅ | 通知 ID |
| tenant_id | int64 | ✅ | 租户 ID（双条件防跨租户） |

**响应字段**：

| 字段 | 类型 | 说明 |
|------|------|------|
| success | bool | true=成功，false=通知不存在或已读 |

## 四、多租户隔离保证

| 隔离点 | 实现 |
|--------|------|
| 输入校验 | `tenant_id <= 0` 拒绝 |
| SQL 查询 | `WHERE tenant_id = $1` 强制 |
| 标记已读 | `WHERE id = $1 AND tenant_id = $2` 双条件 |
| 索引 | `(tenant_id, user_id, created_at DESC)` 联合索引 |
| 数据隔离 | 独立 schema `notifications_db` |

## 五、性能指标

| 指标 | 目标值 |
|------|--------|
| 发送单条通知 P99 | < 50ms |
| 批量发送 100 条 | < 500ms |
| 列出 20 条 P99 | < 100ms |
| 标记已读 P99 | < 30ms |

## 六、限制与约束

- 单条通知 title ≤ 255 字符
- content 无硬限制（数据库 TEXT）
- 批量发送无上限（建议单批 ≤ 1000）
- priority 范围 1-10（1 最高）
- category 建议使用：order / inventory / production / system

## 七、后续演进

- P4+：服务间链路追踪（OpenTelemetry）
- P4+：熔断限流（Sentinel / Hystrix）
- P4+：事件驱动（Kafka / NATS）
- P4+：通知推送集成（WebSocket / Push）
