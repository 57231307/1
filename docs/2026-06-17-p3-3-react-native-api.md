# P3-3 React Native 移动端 - API 文档

> **发布日期**：2026-06-17
> **任务编号**：P3 / P3-3

---

## 一、项目概述

P3-3 React Native 移动端关键路径 demo。

后端复用现有 Rust + axum API，无新增服务端代码。

## 二、ApiClient 接口

### 2.1 初始化

```typescript
import { ApiClient } from '@/components/ApiClient';
```

默认基础 URL：`https://api.bingxi-erp.com/api/v1/erp`

环境变量配置：`EXPO_PUBLIC_API_BASE_URL`

### 2.2 认证模块

#### `ApiClient.auth.login(data)`

登录。

**参数**：
```typescript
interface LoginRequest {
  username: string;
  password: string;
}
```

**返回**：
```typescript
interface LoginResponse {
  token: string;
  user: {
    id: number;
    username: string;
    tenant_id: number;
  };
}
```

**错误**：
- `Error('用户名或密码错误')` - 401
- `Error('网络连接失败')` - 网络错误

**示例**：
```typescript
try {
  const { token, user } = await ApiClient.auth.login({
    username: 'admin',
    password: '123456',
  });
  await useAuthStore.getState().login(token, user);
} catch (err) {
  Alert.alert('登录失败', err.message);
}
```

#### `ApiClient.auth.logout()`

登出。

**返回**：`Promise<void>`

### 2.3 库存模块（示例）

#### `ApiClient.inventory.list(params)`

库存列表。

**参数**：
```typescript
interface InventoryListParams {
  page?: number;  // 默认 1
  size?: number;  // 默认 20
}
```

**返回**：
```typescript
interface PaginatedResponse<InventoryItem> {
  items: InventoryItem[];
  total: number;
  page: number;
  size: number;
}
```

## 三、authStore API

### 3.1 状态

```typescript
interface AuthState {
  token: string | null;
  user: User | null;
  login: (token: string, user: User) => Promise<void>;
  logout: () => Promise<void>;
  hydrate: () => Promise<void>;
}
```

### 3.2 方法

#### `useAuthStore.login(token, user)`

登录，持久化到 AsyncStorage。

```typescript
await useAuthStore.getState().login('jwt-token', {
  id: 1,
  username: 'admin',
  tenant_id: 1,
});
```

#### `useAuthStore.logout()`

登出，清理 AsyncStorage。

```typescript
await useAuthStore.getState().logout();
```

#### `useAuthStore.hydrate()`

启动时从 AsyncStorage 恢复登录状态。

```typescript
// 在 App.tsx 的 useEffect 中
useEffect(() => {
  useAuthStore.getState().hydrate();
}, []);
```

### 3.3 在组件中使用

```typescript
import { useAuthStore } from '@/stores/authStore';

const MyComponent = () => {
  const token = useAuthStore((s) => s.token);
  const user = useAuthStore((s) => s.user);
  // ...
};
```

## 四、WebSocketClient API

### 4.1 初始化

```typescript
import { WebSocketClient } from '@/components/WebSocketClient';

const ws = new WebSocketClient(
  'wss://api.bingxi-erp.com/api/v1/erp/ws/notifications',
  'jwt-token',
);
ws.connect();
```

### 4.2 事件订阅

```typescript
ws.on('notification', (msg) => {
  console.log('收到通知:', msg.data);
});

ws.on('error', (msg) => {
  console.error('WebSocket 错误:', msg.message);
});
```

### 4.3 方法

| 方法 | 用途 |
|------|------|
| `connect()` | 建立连接 |
| `disconnect()` | 主动断开 |
| `send(msg)` | 发送消息 |
| `on(type, listener)` | 订阅事件 |
| `off(type, listener)` | 取消订阅 |
| `isConnected` | 状态查询 |

### 4.4 消息类型

| type | payload | 方向 |
|------|---------|------|
| `notification` | `{ data: NotificationPayload }` | S → C |
| `ping` | `{ timestamp: number }` | C → S |
| `pong` | `{ timestamp: number }` | S → C |
| `error` | `{ code, message }` | S → C |
| `mark_as_read` | `{ id: number }` | C → S |

## 五、Storage 工具

```typescript
import { Storage } from '@/utils/storage';

await Storage.setString('key', 'value');
const value = await Storage.getString('key');
await Storage.setJson('user', { name: 'admin' });
const user = await Storage.getJson('user');
await Storage.remove('key');
await Storage.clear();
```

## 六、validation 工具

```typescript
import { isValidUsername, isValidPassword, isValidEmail } from '@/utils/validation';

isValidUsername('admin');  // true
isValidPassword('123456'); // true
isValidEmail('a@b.com');   // true
```

## 七、TypeScript 类型

```typescript
// src/types/api.ts
export interface ApiResponse<T = unknown> {
  code: number;
  message: string;
  data: T;
}

export interface NotificationPayload {
  id: number;
  title: string;
  content: string;
  category: string;
  priority: number;
  created_at: string;
}

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  size: number;
}

export interface User {
  id: number;
  username: string;
  tenant_id: number;
  email?: string;
  roles?: string[];
}
```

## 八、测试

```bash
npm test
```

## 九、限制

- 仅做关键路径 demo，业务页面留 P4+
- 无离线架构（依赖网络）
- 无原生推送（需 P4+ 集成 APNs/FCM）
- AsyncStorage 未加密（生产用 keychain）
- 无生物识别（需 P4+）
