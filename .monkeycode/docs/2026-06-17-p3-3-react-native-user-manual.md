# P3-3 React Native 移动端 - 用户手册

> **发布日期**：2026-06-17
> **任务编号**：P3 / P3-3

---

## 一、为什么需要移动端

冰溪 ERP 当前仅有 Web 端，业务痛点：
- **销售外出**：客户拜访时无法查询库存 / 下单
- **生产巡检**：车间无法实时查看工单进度
- **仓库盘点**：PDA 设备需要原生 App
- **响应延迟**：P3-2 WebSocket 在移动端浏览器体验差

P3-3 React Native 移动端解决上述问题。

## 二、技术选型

### 2.1 React Native vs Flutter vs Native

| 维度 | React Native | Flutter | Native (Swift/Kotlin) |
|------|--------------|---------|------------------------|
| 跨平台 | ✅ iOS + Android | ✅ iOS + Android | ❌ 双倍开发 |
| 性能 | 中（接近原生） | 中 | ✅ 最佳 |
| 学习曲线 | 低（前端友好） | 中（Dart） | 高（Swift/Kotlin） |
| 生态 | ✅ 成熟 | 中 | ✅ 最丰富 |
| 团队技能 | ✅ Vue/React 友好 | 中 | 需双技能 |

**决策**：React Native + TypeScript（团队有 Vue 经验，跨平台效率高）

### 2.2 状态管理：Zustand vs Redux Toolkit

| 维度 | Zustand | Redux Toolkit |
|------|---------|---------------|
| 包大小 | 1KB | 10KB |
| TS 友好 | ✅ 优秀 | 中 |
| 学习曲线 | 低 | 中 |
| DevTools | ✅ | ✅ |

**决策**：Zustand（更轻量、TS 类型推断更好）

### 2.3 UI 库：React Native Paper vs NativeBase

**决策**：React Native Paper（Material Design 3，社区活跃）

## 三、目录结构

详见 `mobile/README.md`。

## 四、本地启动

### 4.1 前置依赖

- Node.js 18+
- npm 9+
- iOS：Xcode 15+ + CocoaPods
- Android：Android Studio + Android SDK 34

### 4.2 启动步骤

```bash
cd mobile

# 1. 装依赖
npm install

# 2. iOS 额外步骤
cd ios && pod install && cd ..

# 3. 启动 Metro
npm start

# 4. 新终端运行 iOS
npm run ios

# 5. 或运行 Android
npm run android
```

## 五、登录页使用

### 5.1 流程

1. App 启动 → 检测 AsyncStorage 中的 token
2. 有 token → 跳转到 HomePage
3. 无 token → 显示 LoginPage
4. 用户输入用户名 + 密码 → 提交
5. 成功后跳转 HomePage
6. 失败时显示错误提示

### 5.2 代码位置

- `mobile/src/pages/LoginPage.tsx`：登录页 UI
- `mobile/src/components/ApiClient.ts`：API 客户端
- `mobile/src/stores/authStore.ts`：认证状态

## 六、API 客户端

### 6.1 设计原则

- **统一拦截器**：自动注入 JWT、统一错误处理
- **业务模块**：按 `auth` / `inventory` 等 namespace 组织
- **类型安全**：TypeScript 严格类型
- **环境变量**：`EXPO_PUBLIC_API_BASE_URL` 配置后端地址

### 6.2 使用示例

```typescript
import { ApiClient } from '@/components/ApiClient';

// 登录
const response = await ApiClient.auth.login({
  username: 'admin',
  password: '123456',
});
console.log(response.token);

// 库存列表
const inventory = await ApiClient.inventory.list({ page: 1, size: 20 });
```

## 七、状态管理

### 7.1 Zustand store 示例

```typescript
import { create } from 'zustand';

interface CounterState {
  count: number;
  increment: () => void;
  decrement: () => void;
}

export const useCounterStore = create<CounterState>((set) => ({
  count: 0,
  increment: () => set((s) => ({ count: s.count + 1 })),
  decrement: () => set((s) => ({ count: s.count - 1 })),
}));
```

### 7.2 在 React 组件中使用

```typescript
import { useCounterStore } from '@/stores/counterStore';

const Counter: React.FC = () => {
  const { count, increment } = useCounterStore();
  return (
    <View>
      <Text>{count}</Text>
      <Button title="+" onPress={increment} />
    </View>
  );
};
```

## 八、安全

### 8.1 当前实现

- **JWT 存储**：AsyncStorage（未加密）
- **HTTPS 强制**：生产环境推荐
- **多租户隔离**：JWT 携带 `tenant_id`

### 8.2 后续增强（P4+）

- **敏感信息加密**：react-native-keychain
- **证书锁定**：react-native-ssl-pinning
- **生物识别**：react-native-biometrics
- **越狱检测**：react-native-jailbreak-detector

## 九、API 客户端设计

### 9.1 模块组织

```
ApiClient
├── auth
│   ├── login(data)
│   └── logout()
├── inventory
│   └── list(params)
├── sales (后续)
├── production (后续)
└── notifications (后续，集成 P3-2 WebSocket)
```

### 9.2 拦截器

```typescript
// 请求拦截器：自动注入 JWT
this.axios.interceptors.request.use(async (config) => {
  const token = await AsyncStorage.getItem('auth_token');
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

// 响应拦截器：统一错误处理
this.axios.interceptors.response.use(
  (response) => response.data,
  (error) => Promise.reject(this.normalizeError(error)),
);
```

## 十、后续演进

P3-3 仅做关键路径 demo。后续 P4+ 阶段：

1. **业务页面扩展**：Dashboard / Inventory / Sales / Production / Notifications
2. **离线架构**：SQLite + 同步队列
3. **原生推送**：APNs（iOS）+ FCM（Android）
4. **生物识别**：TouchID / FaceID
5. **深链接**：通用链接
6. **CI/CD**：Fastlane + TestFlight / Play Store
7. **国际化**：i18n 框架
8. **崩溃监控**：Sentry

详见 `docs/superpowers/specs/2026-06-17-p3-3-react-native.md`。
