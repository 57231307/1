# 冰溪 ERP 移动端 (P3-3)

> P3-3 React Native 关键路径 demo
> 冰溪 ERP 移动端 - 登录页 + API 客户端

## 一、概述

这是 P3-3 React Native 移动端任务的**关键路径 demo**。

冰溪 ERP 当前仅有 Web 端，移动办公需求强烈。本 demo 验证：
- React Native + TypeScript 跨平台架构
- 与主项目 `backend/` 的 REST API 集成
- 复用 P3-2 WebSocket 实现实时通知
- Zustand 状态管理 + AsyncStorage 持久化

## 二、技术栈

| 维度 | 选型 | 版本 |
|------|------|------|
| 框架 | React Native | 0.74+ |
| 语言 | TypeScript | 5.0+ |
| 状态管理 | Zustand | 4.5+ |
| HTTP 客户端 | Axios | 1.6+ |
| 路由 | React Navigation | 6.x |
| UI 库 | React Native Paper | 5.x |
| 持久化 | AsyncStorage | 1.23+ |
| WebSocket | 原生 + 自研封装 | - |

## 三、目录结构

```
mobile/
├── package.json                  # 依赖管理
├── tsconfig.json                 # TypeScript 配置
├── babel.config.js               # Babel 配置
├── metro.config.js               # Metro 配置
├── app.json                      # App 元数据
├── index.js                      # 入口
├── App.tsx                       # 根组件 + 路由
├── src/
│   ├── pages/
│   │   ├── LoginPage.tsx         # 登录页（关键路径 demo）
│   │   └── HomePage.tsx          # 首页（占位）
│   ├── components/
│   │   ├── ApiClient.ts          # API 客户端（关键路径 demo）
│   │   └── WebSocketClient.ts    # WebSocket 客户端（复用 P3-2）
│   ├── stores/
│   │   └── authStore.ts          # 认证状态（Zustand）
│   ├── api/
│   │   └── auth.ts               # 认证 API 封装
│   ├── types/
│   │   └── api.ts                # API 类型
│   └── utils/
│       ├── storage.ts            # AsyncStorage 封装
│       └── validation.ts         # 输入校验
├── __tests__/
│   └── LoginPage.test.tsx        # 单元测试
└── README.md                     # 本文件
```

## 四、启动

### 4.1 前置依赖

- Node.js 18+
- npm 或 yarn
- iOS 开发：Xcode + CocoaPods
- Android 开发：Android Studio + Android SDK

### 4.2 安装依赖

```bash
cd mobile
npm install
# iOS 还需要：
cd ios && pod install && cd ..
```

### 4.3 启动开发服务器

```bash
npm start
```

### 4.4 运行 iOS

```bash
npm run ios
```

### 4.5 运行 Android

```bash
npm run android
```

## 五、关键路径 demo：登录流程

### 5.1 流程图

```
LoginPage
  ↓ 用户输入 username + password
ApiClient.auth.login()
  ↓ POST /api/v1/erp/auth/login
后端验证
  ↓ 返回 { token, user }
authStore.login(token, user)
  ↓ AsyncStorage 持久化
跳转 HomePage
```

### 5.2 代码要点

**LoginPage.tsx**（简化）：
```typescript
const handleLogin = async () => {
  setLoading(true);
  try {
    const response = await ApiClient.auth.login({ username, password });
    await login(response.token, response.user);
    // App.tsx 监听到 token 变化，自动跳转 HomePage
  } catch (err) {
    Alert.alert('登录失败', err.message);
  } finally {
    setLoading(false);
  }
};
```

**ApiClient.ts**（简化）：
```typescript
class ApiClientImpl {
  constructor() {
    this.axios = axios.create({ baseURL: API_BASE_URL });
    this.axios.interceptors.request.use(async (config) => {
      const token = await AsyncStorage.getItem('auth_token');
      if (token) config.headers.Authorization = `Bearer ${token}`;
      return config;
    });
  }

  auth = {
    login: (data) => this.axios.post('/auth/login', data).then((r) => r.data),
  };
}
```

## 六、架构图

```
┌─────────────────────────────────────────────┐
│           iOS / Android App                  │
│  ┌─────────────────────────────────────┐    │
│  │      React Native (TypeScript)       │    │
│  │  ┌───────────┐  ┌────────────┐      │    │
│  │  │ LoginPage │  │  HomePage  │      │    │
│  │  └─────┬─────┘  └──────┬─────┘      │    │
│  │        │               │            │    │
│  │  ┌─────▼───────────────▼─────┐      │    │
│  │  │    Zustand authStore      │      │    │
│  │  └─────┬─────────────────────┘      │    │
│  │        │                            │    │
│  │  ┌─────▼──────┐  ┌──────────────┐  │    │
│  │  │ ApiClient  │  │ WebSocketClient│  │    │
│  │  │  (Axios)   │  │  (P3-2 复用)  │  │    │
│  │  └─────┬──────┘  └──────┬───────┘  │    │
│  └────────┼────────────────┼──────────┘    │
└───────────┼────────────────┼────────────────┘
            │ HTTPS          │ WSS
            ▼                ▼
   ┌────────────────────────────────────┐
   │  现有 ERP 后端（Rust + axum）       │
   │  - /api/v1/erp/auth/login           │
   │  - /api/v1/erp/ws/notifications    │
   └────────────────────────────────────┘
```

## 七、安全

- **JWT 存储**：AsyncStorage（demo）；生产用 react-native-keychain 加密
- **HTTPS 强制**：生产禁用 HTTP
- **多租户隔离**：JWT 携带 tenant_id，所有请求自动注入
- **生物识别**：P4+ 集成 react-native-biometrics
- **证书锁定**：P4+ 集成

## 八、API 客户端

| 方法 | 路径 | 用途 |
|------|------|------|
| `ApiClient.auth.login(data)` | POST `/auth/login` | 登录 |
| `ApiClient.auth.logout()` | POST `/auth/logout` | 登出 |
| `ApiClient.inventory.list(params)` | GET `/inventory` | 库存列表 |

## 九、测试

```bash
npm test
# 或
npx jest
```

## 十、故障排查

| 现象 | 原因 | 解决 |
|------|------|------|
| npm install 失败 | 网络问题 | 切换 npm registry / 用 yarn |
| iOS 启动失败 | Pod 未安装 | `cd ios && pod install` |
| Android 启动失败 | SDK 缺失 | 配置 ANDROID_HOME + 装 SDK |
| 登录失败 | 后端地址错误 | 检查 `EXPO_PUBLIC_API_BASE_URL` |
| 401 Unauthorized | token 过期 | 重新登录 |

## 十一、CI 验证

- `npm ci` 装依赖
- `npx tsc --noEmit` 类型检查
- `npx jest` 跑测试
- `npx react-native build-android --mode release` 完整构建

## 十二、后续演进（P4+）

1. **业务页面**：Dashboard / Inventory / Sales / Production / Notifications
2. **离线架构**：SQLite 缓存 + 同步队列
3. **原生推送**：APNs（iOS）+ FCM（Android）
4. **生物识别**：TouchID / FaceID
5. **深链接**：通用链接 + 应用链接
6. **CI/CD**：Fastlane + TestFlight / Play Store
7. **国际化**：i18n 框架集成
8. **崩溃监控**：Sentry 集成
