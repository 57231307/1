# P3-2 WebSocket 实时通信 - 用户手册

> **发布日期**：2026-06-17
> **任务编号**：P3 / P3-2
> **关联**：Spec + Plan

---

## 一、什么是 WebSocket 实时通信

冰溪 ERP 通知模块原采用**轮询**（前端每 30 秒 GET 一次 `/api/v1/erp/notifications`），存在：
- 延迟高（30 秒轮询周期）
- 浪费带宽
- 移动端体验差

P3-2 升级为 **WebSocket 实时推送**：
- **延迟降低到 < 1 秒**
- **节省带宽**（按需推送）
- **移动端友好**（可结合 Push Notification）
- **多端同步**

## 二、与轮询模式对比

| 指标 | 轮询模式 | WebSocket 实时推送 |
|------|----------|---------------------|
| 消息延迟 | 0-30 秒 | < 1 秒 |
| 带宽占用 | 持续高（每 30 秒请求） | 仅推送时 |
| 服务端负载 | 高（持续接收请求） | 低（仅推送时） |
| 移动端体验 | 差 | 好 |
| 锁屏支持 | 无 | 需配合 Push |

## 三、架构

```
┌─────────────────────────────────────────┐
│           Frontend (Vue 3)              │
│  ┌───────────────────────────────────┐  │
│  │    WebSocketClient (utils)        │  │
│  │  - 自动重连（指数退避）            │  │
│  │  - 心跳（30s ping）               │  │
│  │  - 事件分发（EventTarget）        │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
              │ ws://server/api/v1/erp/ws/notifications?token=jwt
              ▼
┌─────────────────────────────────────────┐
│         Backend (axum + tokio)          │
│  ┌───────────────────────────────────┐  │
│  │      WebSocket Handler            │  │
│  │  - JWT 验证                       │  │
│  │  - 连接管理（ConnectionManager）   │  │
│  │  - 消息广播（broadcast::Sender）  │  │
│  │  - 心跳处理                       │  │
│  └───────────────────────────────────┘  │
│              │                          │
│  ┌───────────────────────────────────┐  │
│  │   notification_service.send()     │  │
│  │   → ws_broadcaster.broadcast()    │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

## 四、连接管理

### 4.1 连接生命周期

1. 客户端发起 `new WebSocket(url + '?token=xxx')`
2. 服务端验证 JWT
3. 升级到 WebSocket 协议
4. 注册到 `ConnectionManager`（按 `(tenant_id, user_id)` 分组）
5. 进入消息循环
6. 断开时自动注销

### 4.2 多端登录

- 同一用户可在多个端（浏览器、平板、手机）同时连接
- 服务端为每个 `(tenant_id, user_id)` 维护一个 `broadcast::Sender`
- 多端都会收到推送

### 4.3 多租户隔离

- 消息按 `(tenant_id, user_id)` 双键路由
- 不同租户的用户**不会**收到跨租户消息

## 五、消息协议

### 5.1 服务端 → 客户端

#### 通知消息
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

#### 心跳响应
```json
{
  "type": "pong",
  "timestamp": 1718612400
}
```

#### 错误消息
```json
{
  "type": "error",
  "code": "INVALID_TOKEN",
  "message": "JWT 验证失败"
}
```

### 5.2 客户端 → 服务端

#### 心跳请求
```json
{
  "type": "ping",
  "timestamp": 1718612400
}
```

#### 标记已读
```json
{
  "type": "mark_as_read",
  "id": 42
}
```

## 六、前端使用示例

### 6.1 基本使用

```typescript
import { WebSocketClient } from '@/utils/websocket';

const ws = new WebSocketClient(
  '/api/v1/erp/ws/notifications',
  '1:100'  // tenant_id:user_id
);

ws.connect();

ws.addEventListener('notification', (event) => {
  console.log('收到通知:', event.detail.data);
  // 显示 toast、声音提示、更新 store
});

ws.addEventListener('reconnecting', (event) => {
  console.log(`第 ${event.detail.attempt} 次重连，${event.detail.delay}ms 后`);
});

ws.addEventListener('max_reconnect_failed', () => {
  console.error('重连失败，请刷新页面');
});
```

### 6.2 Vue 3 集成

```vue
<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref } from 'vue';
import { WebSocketClient, NotificationPayload } from '@/utils/websocket';

const notifications = ref<NotificationPayload[]>([]);
let ws: WebSocketClient | null = null;

onMounted(() => {
  const token = localStorage.getItem('token') || '';
  ws = new WebSocketClient('/api/v1/erp/ws/notifications', token);
  ws.connect();

  ws.addEventListener('notification', (event) => {
    notifications.value.unshift(event.detail.data);
  });
});

onBeforeUnmount(() => {
  ws?.disconnect();
});
</script>
```

## 七、性能与限制

| 指标 | 目标值 |
|------|--------|
| 单进程最大连接数 | 10,000 |
| 消息延迟 P99 | < 100ms |
| 心跳超时 | 60s（无 pong 即断开） |
| 广播吞吐 | 10,000 msg/s |
| 单消息大小 | < 64KB |

## 八、安全

- **JWT 鉴权**：URL query 携带 token（与浏览器 WebSocket API 一致）
- **多租户隔离**：消息按 `(tenant_id, user_id)` 双键路由
- **TLS 加密**：生产环境使用 `wss://`
- **token 泄露风险**：URL 可能记录在 server log，建议改用 `Sec-WebSocket-Protocol` 头（但浏览器 API 不支持）

## 九、故障排查

| 现象 | 原因 | 解决 |
|------|------|------|
| 连接立即断开 | JWT 无效 | 检查 token 格式（应为 `tenant_id:user_id`） |
| 心跳超时断开 | 60s 无 pong | 检查网络 + 调整 HEARTBEAT_INTERVAL |
| 重连失败 | 服务端崩溃 | 检查服务端日志 + 触发 `max_reconnect_failed` 事件 |
| 收不到推送 | 连接断开 | 监听 `close` + `reconnecting` 事件 |
| 跨租户串扰 | tenant_id 错误 | 检查 JWT 中的 tenant_id |

## 十、CI 验证

- 后端：`cd backend && cargo check --lib`（含 WebSocket 模块）
- 前端：`cd frontend && npx vue-tsc --noEmit`（检查 TS 类型）
- 沙箱限制：仅 `cargo check --lib` 验证编译
- CI 完整测试（GitHub Actions runner 内存充足）
