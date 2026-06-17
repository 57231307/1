# P3-3 React Native 移动端设计 Spec

> **设计日期**：2026-06-17
> **任务编号**：P3 / P3-3
> **关联**：P3-2 WebSocket（移动端推送技术基础）
> **设计基线**：test @ a165db1（含 P3-2 WebSocket）

---

## 一、目标与背景

### 1.1 业务目标

当前冰溪 ERP 仅有**桌面 Web 端**，用户痛点：
- **移动办公缺失**：销售外出、生产巡检、仓库盘点无法使用
- **响应不及时**：离开电脑无法接收通知（虽然 P3-2 实现了 WebSocket，但移动端浏览器体验差）
- **离线能力弱**：Web 端断网即瘫痪

升级为**移动 App** 后：
- **iOS + Android 双端覆盖**（React Native 跨平台）
- **原生推送**（APNs / FCM）
- **离线缓存**（AsyncStorage + SQLite）
- **生物识别**（TouchID / FaceID）

### 1.2 技术目标

- **1 个**完整的 React Native 移动端项目（`mobile/`）
- **1 个**登录页面 demo（LoginPage）
- **1 个** API 客户端组件（ApiClient）
- **1 个**单元测试
- **完整 spec + plan** 描述未来 5+ 页面 + 离线架构 + 推送集成
- 复用主项目 `backend/` 的 REST API（无需后端改造）

### 1.3 范围

**包含**：
1. 完整 React Native 移动端设计 spec（本文件）
2. 完整实施 plan（`docs/superpowers/plans/2026-06-17-p3-3-react-native.md`）
3. 关键路径 demo：登录页 + API 客户端
   - `mobile/` 目录（独立 React Native 项目）
   - `package.json` + `tsconfig.json` + `babel.config.js`
   - 1 个页面：`LoginPage.tsx`
   - 1 个组件：`ApiClient.ts`（封装 REST API）
   - 1 个测试：`LoginPage.test.tsx`
   - `README.md`（启动说明 + 架构图）
4. 主项目 0 改动（移动端独立项目）

**不包含**（P4+ 后续阶段）：
- 完整 5+ 业务页面（库存、销售、生产、工艺、通知）
- 离线架构（SQLite + 同步队列）
- 原生推送集成（APNs / FCM）
- 生物识别（TouchID / FaceID）
- CI/CD（Fastlane + TestFlight / Play Store）
- App Store / Google Play 上架

---

## 二、决策记录（Q1-Q8 + 矛盾解决）

### 2.1 8 个澄清问题

| 编号 | 问题 | 决策 |
|------|------|------|
| Q1 | 移动端框架 | React Native 0.74+（TypeScript） |
| Q2 | 状态管理 | Zustand（轻量、TS 友好） |
| Q3 | 网络请求 | Axios（拦截器完善） |
| Q4 | 导航 | React Navigation 6（Stack + Tab） |
| Q5 | 持久化 | AsyncStorage（轻量 KV） |
| Q6 | UI 库 | React Native Paper（Material Design） |
| Q7 | 离线架构 | 暂不实现（P4+ 引入 SQLite） |
| Q8 | 是否合到 main | 不合到 main（仅合到 test） |

### 2.2 矛盾解决

**矛盾 1**：原生体验 vs 跨平台效率
- **决策**：React Native + TypeScript（共享 80% 代码）
- **理由**：iOS + Android 双端覆盖，迭代效率高；性能关键模块后续可换原生

**矛盾 2**：沙箱限制 vs 完整 RN 环境
- **决策**：仅做登录页 demo + 完整 spec，**不**真跑 RN CLI 启动（沙箱无 Android SDK / iOS toolchain）
- **理由**：保留架构完整性，CI 负责完整构建

**矛盾 3**：状态管理选型
- **决策**：Zustand 而非 Redux Toolkit
- **理由**：Zustand 更轻量（1KB vs 10KB），TS 类型推断更好，社区活跃

---

## 三、架构设计

### 3.1 整体架构图

```
┌──────────────────────────────────────────────────────────────┐
│                    iOS / Android App                          │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                React Native (TypeScript)                │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │  │
│  │  │   LoginPage  │  │ InventoryPage│  │   HomePage   │  │  │
│  │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  │  │
│  │         │                 │                 │          │  │
│  │  ┌──────▼─────────────────▼─────────────────▼───────┐  │  │
│  │  │         Zustand Stores (auth / inventory)        │  │  │
│  │  └──────┬───────────────────────────────────────────┘  │  │
│  │         │                                              │  │
│  │  ┌──────▼──────────┐  ┌─────────────┐                 │  │
│  │  │   ApiClient     │  │  WebSocket  │  (P3-2 复用)    │  │
│  │  │  (Axios)        │  │   Client    │                 │  │
│  │  └──────┬──────────┘  └──────┬──────┘                 │  │
│  └─────────┼────────────────────┼────────────────────────┘  │
└────────────┼────────────────────┼────────────────────────────┘
             │ HTTPS              │ WSS
             ▼                    ▼
    ┌────────────────────────────────────┐
    │   现有 ERP 后端（rust + axum）      │
    │   - /api/v1/erp/auth/login         │
    │   - /api/v1/erp/inventory          │
    │   - /api/v1/erp/ws/notifications   │
    └────────────────────────────────────┘
```

### 3.2 目录结构

```
mobile/
├── package.json              # 依赖管理
├── tsconfig.json             # TypeScript 配置
├── babel.config.js           # Babel 配置
├── metro.config.js           # Metro 配置
├── app.json                  # App 元数据
├── index.js                  # 入口
├── App.tsx                   # 根组件
├── src/
│   ├── pages/
│   │   ├── LoginPage.tsx     # 登录页（demo）
│   │   └── HomePage.tsx      # 首页（占位）
│   ├── components/
│   │   ├── ApiClient.ts      # API 客户端（demo）
│   │   └── WebSocketClient.ts # WebSocket 客户端（复用 P3-2）
│   ├── stores/
│   │   ├── authStore.ts      # 认证状态
│   │   └── inventoryStore.ts # 库存状态
│   ├── api/
│   │   ├── auth.ts           # 认证 API
│   │   └── inventory.ts      # 库存 API
│   ├── types/
│   │   └── api.ts            # API 类型
│   └── utils/
│       ├── storage.ts        # AsyncStorage 封装
│       └── validation.ts     # 输入校验
├── __tests__/
│   └── LoginPage.test.tsx    # 单元测试
└── README.md                 # 启动说明 + 架构图
```

### 3.3 关键路径：LoginPage + ApiClient

#### 3.3.1 LoginPage 设计

```typescript
// src/pages/LoginPage.tsx

export const LoginPage: React.FC = () => {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleLogin = async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await ApiClient.auth.login({ username, password });
      await authStore.login(response.token);
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <View>
      <TextInput value={username} onChangeText={setUsername} />
      <TextInput value={password} onChangeText={setPassword} secureTextEntry />
      <Button title="登录" onPress={handleLogin} loading={loading} />
      {error && <Text style={styles.error}>{error}</Text>}
    </View>
  );
};
```

#### 3.3.2 ApiClient 设计

```typescript
// src/components/ApiClient.ts

class ApiClientImpl {
  private axios: AxiosInstance;

  constructor(baseURL: string) {
    this.axios = axios.create({ baseURL, timeout: 10000 });
    this.axios.interceptors.request.use(async (config) => {
      const token = await authStore.getToken();
      if (token) config.headers.Authorization = `Bearer ${token}`;
      return config;
    });
    this.axios.interceptors.response.use(
      (response) => response.data,
      (error) => Promise.reject(this.normalizeError(error)),
    );
  }

  auth = {
    login: (data: LoginRequest) =>
      this.axios.post<LoginResponse>('/auth/login', data),
  };

  inventory = {
    list: (params: InventoryListParams) =>
      this.axios.get<PaginatedResponse<InventoryItem>>('/inventory', { params }),
  };
}

export const ApiClient = new ApiClientImpl(
  process.env.EXPO_PUBLIC_API_BASE_URL || 'https://api.bingxi-erp.com/api/v1/erp',
);
```

### 3.4 状态管理（Zustand）

```typescript
// src/stores/authStore.ts

import { create } from 'zustand';
import AsyncStorage from '@react-native-async-storage/async-storage';

interface AuthState {
  token: string | null;
  user: User | null;
  login: (token: string, user: User) => Promise<void>;
  logout: () => Promise<void>;
}

export const useAuthStore = create<AuthState>((set) => ({
  token: null,
  user: null,
  login: async (token, user) => {
    await AsyncStorage.setItem('token', token);
    set({ token, user });
  },
  logout: async () => {
    await AsyncStorage.removeItem('token');
    set({ token: null, user: null });
  },
}));
```

### 3.5 路由（React Navigation）

```typescript
// App.tsx

const Stack = createNativeStackNavigator();

export default function App() {
  const isLoggedIn = useAuthStore((s) => !!s.token);

  return (
    <NavigationContainer>
      <Stack.Navigator>
        {isLoggedIn ? (
          <Stack.Screen name="Home" component={HomePage} />
        ) : (
          <Stack.Screen name="Login" component={LoginPage} />
        )}
      </Stack.Navigator>
    </NavigationContainer>
  );
}
```

### 3.6 关键依赖

| 包 | 版本 | 用途 |
|----|------|------|
| react | 18.2+ | 核心 |
| react-native | 0.74+ | 跨平台 |
| typescript | 5.0+ | 类型系统 |
| zustand | 4.5+ | 状态管理 |
| axios | 1.6+ | HTTP 客户端 |
| @react-navigation/native | 6.x | 路由 |
| @react-native-async-storage/async-storage | 1.23+ | KV 持久化 |
| react-native-paper | 5.x | UI 组件 |
| @testing-library/react-native | 12.x | 单元测试 |

---

## 四、安全与多租户

- **JWT 存储**：AsyncStorage 加密（react-native-keychain 集成，P4+）
- **HTTPS 强制**：生产禁用 HTTP
- **证书锁定**：P4+ 集成
- **多租户**：JWT 携带 `tenant_id`，所有 API 请求自动注入
- **生物识别**：P4+ 集成 react-native-biometrics

---

## 五、CI 验证策略

- **沙箱限制**：RN 项目无法在沙箱构建（无 Android SDK / iOS toolchain）
- **CI 验证**（GitHub Actions）：
  - `npm ci` 安装依赖
  - `npx tsc --noEmit` 类型检查
  - `npx jest` 跑单元测试
  - `npx react-native build-android`（仅 release）

---

## 六、用户验收标准

| 编号 | 验收项 | 验证方法 |
|------|--------|----------|
| AC-1 | spec + plan 完整 | 文档存在 + 含本文件全部章节 |
| AC-2 | `mobile/` 目录完整 | package.json + tsconfig.json + src/ 存在 |
| AC-3 | 1 个页面 + 1 个 API 客户端 | LoginPage.tsx + ApiClient.ts 存在 |
| AC-4 | 1 个测试 | `__tests__/LoginPage.test.tsx` |
| AC-5 | 类型检查通过 | `npx tsc --noEmit`（CI 跑） |
| AC-6 | 不破坏 P0/P1/P2/P3-1/P3-2 | 主项目 `cargo check --lib` 通过 |
| AC-7 | README 完整 | 启动说明 + 架构图 + 后续演进 |

---

## 七、风险与回滚

### 7.1 风险

| 风险 | 等级 | 缓解 |
|------|------|------|
| 沙箱无 RN 环境 | 高 | 仅做源码 + spec，CI 跑完整构建 |
| 依赖版本冲突 | 中 | 锁定 package.json + 使用 npm ci |
| 主项目兼容性 | 低 | mobile/ 是独立项目，不影响主项目 |
| TypeScript 编译错误 | 中 | 严格 tsconfig + 测试 |

### 7.2 回滚

- 删除 `mobile/` 目录
- 不影响主项目 `backend/` 与 `frontend/`

---

## 八、关联

- Plan：`docs/superpowers/plans/2026-06-17-p3-3-react-native.md`
- 用户手册：`docs/2026-06-17-p3-3-react-native-user-manual.md`
- API 文档：`docs/2026-06-17-p3-3-react-native-api.md`
- CHANGELOG：`CHANGELOG.md`
- MEMORY：`MEMORY.md`
