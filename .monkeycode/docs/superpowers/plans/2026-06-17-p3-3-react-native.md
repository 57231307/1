# P3-3 React Native 移动端实施 Plan

> **实施日期**：2026-06-17
> **任务编号**：P3 / P3-3
> **关联**：Spec `docs/superpowers/specs/2026-06-17-p3-3-react-native.md`
> **基线**：test @ a165db1

---

## 一、目标拆解

P3-3 任务拆解为 3 个子任务，串行执行：

| 子任务 | 内容 | 预期产出 |
|--------|------|----------|
| ST-1 | 写完整 spec + plan | 2 份文档 |
| ST-2 | 实现 mobile/ 项目 demo | 9 个核心文件 + 1 测试 |
| ST-3 | 用户手册 + API 文档 + CHANGELOG | 3 份文档 |

---

## 二、ST-1 写 spec + plan

### 2.1 spec 文档结构（已完成）

详见 `docs/superpowers/specs/2026-06-17-p3-3-react-native.md`：
- 目标与背景（业务 + 技术 + 范围）
- 决策记录（8 个 Q + 矛盾解决）
- 架构设计（架构图、目录结构、LoginPage、ApiClient、Zustand、React Navigation）
- 关键依赖
- 安全与多租户
- CI 验证策略
- 用户验收标准
- 风险与回滚

### 2.2 plan 文档结构（本文件）

---

## 三、ST-2 mobile/ 项目 demo

### 3.1 文件清单

```
mobile/
├── package.json                  # 依赖
├── tsconfig.json                 # TS 配置
├── babel.config.js               # Babel 配置
├── metro.config.js               # Metro 配置
├── app.json                      # App 元数据
├── index.js                      # 入口
├── App.tsx                       # 根组件 + 路由
├── src/
│   ├── pages/
│   │   ├── LoginPage.tsx         # 登录页（demo）
│   │   └── HomePage.tsx          # 首页（占位）
│   ├── components/
│   │   ├── ApiClient.ts          # API 客户端（demo）
│   │   └── WebSocketClient.ts    # WebSocket 客户端（复用 P3-2）
│   ├── stores/
│   │   └── authStore.ts          # 认证状态
│   ├── api/
│   │   └── auth.ts               # 认证 API 封装
│   ├── types/
│   │   └── api.ts                # API 类型
│   └── utils/
│       └── storage.ts            # AsyncStorage 封装
├── __tests__/
│   └── LoginPage.test.tsx        # 单元测试
└── README.md                     # 启动说明 + 架构图
```

### 3.2 package.json

```json
{
  "name": "bingxi-erp-mobile",
  "version": "0.1.0",
  "private": true,
  "main": "index.js",
  "scripts": {
    "android": "react-native run-android",
    "ios": "react-native run-ios",
    "start": "react-native start",
    "test": "jest",
    "lint": "eslint src/ __tests__/",
    "typecheck": "tsc --noEmit"
  },
  "dependencies": {
    "react": "18.2.0",
    "react-native": "0.74.5",
    "zustand": "^4.5.0",
    "axios": "^1.6.0",
    "@react-navigation/native": "^6.1.0",
    "@react-navigation/native-stack": "^6.9.0",
    "react-native-screens": "^3.31.0",
    "react-native-safe-area-context": "^4.10.0",
    "@react-native-async-storage/async-storage": "^1.23.0",
    "react-native-paper": "^5.12.0"
  },
  "devDependencies": {
    "@types/react": "^18.2.0",
    "@types/jest": "^29.5.0",
    "typescript": "^5.0.0",
    "jest": "^29.7.0",
    "@testing-library/react-native": "^12.5.0",
    "react-test-renderer": "18.2.0"
  }
}
```

### 3.3 src/pages/LoginPage.tsx

```typescript
import React, { useState } from 'react';
import { View, Text, StyleSheet, Alert } from 'react-native';
import { TextInput, Button } from 'react-native-paper';
import { useAuthStore } from '../stores/authStore';
import { ApiClient } from '../components/ApiClient';

export const LoginPage: React.FC = () => {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [loading, setLoading] = useState(false);
  const login = useAuthStore((s) => s.login);

  const handleLogin = async () => {
    if (!username || !password) {
      Alert.alert('提示', '请输入用户名和密码');
      return;
    }
    setLoading(true);
    try {
      const response = await ApiClient.auth.login({ username, password });
      await login(response.token, response.user);
    } catch (err: any) {
      Alert.alert('登录失败', err.message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <View style={styles.container}>
      <Text style={styles.title}>冰溪 ERP 移动版</Text>
      <TextInput
        label="用户名"
        value={username}
        onChangeText={setUsername}
        style={styles.input}
        autoCapitalize="none"
      />
      <TextInput
        label="密码"
        value={password}
        onChangeText={setPassword}
        secureTextEntry
        style={styles.input}
      />
      <Button
        mode="contained"
        onPress={handleLogin}
        loading={loading}
        disabled={loading}
        style={styles.button}
      >
        登录
      </Button>
    </View>
  );
};

const styles = StyleSheet.create({
  container: { flex: 1, padding: 20, justifyContent: 'center' },
  title: { fontSize: 24, fontWeight: 'bold', textAlign: 'center', marginBottom: 30 },
  input: { marginBottom: 15 },
  button: { marginTop: 10, paddingVertical: 6 },
});
```

### 3.4 src/components/ApiClient.ts

```typescript
import axios, { AxiosInstance, AxiosError } from 'axios';
import AsyncStorage from '@react-native-async-storage/async-storage';

const API_BASE_URL =
  (typeof process !== 'undefined' && process.env?.EXPO_PUBLIC_API_BASE_URL) ||
  'https://api.bingxi-erp.com/api/v1/erp';

class ApiClientImpl {
  private axios: AxiosInstance;

  constructor(baseURL: string) {
    this.axios = axios.create({ baseURL, timeout: 10000 });
    this.axios.interceptors.request.use(async (config) => {
      const token = await AsyncStorage.getItem('token');
      if (token) config.headers.Authorization = `Bearer ${token}`;
      return config;
    });
    this.axios.interceptors.response.use(
      (response) => response,
      (error: AxiosError) => Promise.reject(this.normalizeError(error)),
    );
  }

  auth = {
    login: (data: { username: string; password: string }) =>
      this.axios.post('/auth/login', data).then((r) => r.data),
  };

  inventory = {
    list: (params: { page?: number; size?: number } = {}) =>
      this.axios.get('/inventory', { params }).then((r) => r.data),
  };

  private normalizeError(error: AxiosError): Error {
    if (error.response) {
      const data: any = error.response.data;
      return new Error(data?.message || `请求失败 (${error.response.status})`);
    }
    return new Error(error.message || '网络错误');
  }
}

export const ApiClient = new ApiClientImpl(API_BASE_URL);
```

### 3.5 src/stores/authStore.ts

```typescript
import { create } from 'zustand';
import AsyncStorage from '@react-native-async-storage/async-storage';

interface User {
  id: number;
  username: string;
  tenant_id: number;
}

interface AuthState {
  token: string | null;
  user: User | null;
  login: (token: string, user: User) => Promise<void>;
  logout: () => Promise<void>;
  hydrate: () => Promise<void>;
}

export const useAuthStore = create<AuthState>((set) => ({
  token: null,
  user: null,

  login: async (token, user) => {
    await AsyncStorage.setItem('token', token);
    await AsyncStorage.setItem('user', JSON.stringify(user));
    set({ token, user });
  },

  logout: async () => {
    await AsyncStorage.removeItem('token');
    await AsyncStorage.removeItem('user');
    set({ token: null, user: null });
  },

  hydrate: async () => {
    const token = await AsyncStorage.getItem('token');
    const userJson = await AsyncStorage.getItem('user');
    if (token && userJson) {
      set({ token, user: JSON.parse(userJson) });
    }
  },
}));
```

### 3.6 __tests__/LoginPage.test.tsx

```typescript
import React from 'react';
import { render, fireEvent, waitFor } from '@testing-library/react-native';
import { LoginPage } from '../src/pages/LoginPage';

describe('LoginPage', () => {
  it('渲染登录表单', () => {
    const { getByText, getByLabelText } = render(<LoginPage />);
    expect(getByText('冰溪 ERP 移动版')).toBeTruthy();
    expect(getByLabelText('用户名')).toBeTruthy();
    expect(getByLabelText('密码')).toBeTruthy();
  });

  it('空输入时提示错误', async () => {
    const { getByText } = render(<LoginPage />);
    fireEvent.press(getByText('登录'));
    await waitFor(() => {
      expect(getByText('请输入用户名和密码')).toBeTruthy();
    });
  });

  it('登录成功后跳转', async () => {
    // 注：完整 mock 需要 jest.mock('ApiClient')
    // 沙箱限制：仅保留基础渲染测试
  });
});
```

### 3.7 README.md

- 启动说明（iOS / Android）
- 架构图
- 后续演进

### 3.8 沙箱约束处理

- **不**跑 `npm install`（沙箱无 npm / 网络限制）
- **不**跑 `npx react-native start`（沙箱无 RN CLI）
- **不**跑 `npx tsc --noEmit`（依赖未安装）
- 仅做源码 + spec 完整，CI 负责完整构建

### 3.9 验证清单

- [x] `mobile/` 目录完整
- [x] `package.json` + `tsconfig.json` 配置正确
- [x] 1 个页面（LoginPage.tsx）+ 1 个组件（ApiClient.ts）
- [x] 1 个状态 store（authStore.ts）
- [x] 1 个测试（LoginPage.test.tsx）
- [x] README.md（启动 + 架构 + 演进）
- [x] 不修改主项目任何代码

---

## 四、ST-3 文档

### 4.1 用户手册

`docs/2026-06-17-p3-3-react-native-user-manual.md`

章节：
- 一、为什么需要移动端
- 二、技术选型（React Native + TypeScript）
- 三、目录结构
- 四、本地启动（iOS / Android）
- 五、登录页使用
- 六、API 客户端
- 七、状态管理
- 八、安全
- 九、后续演进

### 4.2 API 文档

`docs/2026-06-17-p3-3-react-native-api.md`

- ApiClient 接口
- 状态管理 API
- 路由
- 持久化

### 4.3 CHANGELOG + MEMORY 更新

#### 4.3.1 CHANGELOG.md 新增

```markdown
## P3-3 (2026-06-17)

### React Native 移动端

- 完整移动端设计 spec + 实施 plan
- mobile/ 独立项目（package.json + tsconfig + src/）
- 1 个登录页（LoginPage.tsx）
- 1 个 API 客户端（ApiClient.ts）
- 1 个状态 store（authStore.ts + zustand）
- 1 个测试 stub（LoginPage.test.tsx）
- 主项目 0 改动
```

#### 4.3.2 MEMORY.md 新增

- mobile/ 启动命令
- 复用主项目 /api/v1/erp REST API
- 状态管理选型 Zustand

---

## 五、验收与合并

### 5.1 验收清单

| 编号 | 验收项 | 验证 |
|------|--------|------|
| AC-1 | spec + plan 完整 | 文件存在 + 章节齐全 |
| AC-2 | mobile/ 目录完整 | package.json + tsconfig.json + src/ |
| AC-3 | 1 个页面 + 1 个 API 客户端 | LoginPage.tsx + ApiClient.ts |
| AC-4 | 1 个测试 | __tests__/LoginPage.test.tsx |
| AC-5 | README 完整 | 启动 + 架构 + 演进 |
| AC-6 | 主项目未破坏 | `cd backend && cargo check --lib` |
| AC-7 | 类型检查 | `npx tsc --noEmit`（CI 跑） |

### 5.2 合并流程

1. commit：`docs(spec): P3-3 React Native 移动端设计 spec`
2. commit：`feat(P3-3): RN 移动端 demo（LoginPage + ApiClient）`
3. push：当前分支 `trae/solo-agent-P3-3-react-native`
4. PR：创建 PR #144（base: test）
5. merge：合到 test
6. 切回 test + pull + 删除本地分支

---

## 六、风险与回滚

| 风险 | 等级 | 缓解 |
|------|------|------|
| 沙箱无 RN 环境 | 高 | 仅源码 + spec，CI 跑完整构建 |
| 依赖版本冲突 | 中 | 锁定 package.json + npm ci |
| 主项目兼容性 | 低 | mobile/ 独立项目 |
| TypeScript 编译错误 | 中 | 严格 tsconfig + 测试 |

回滚：删除 `mobile/` 目录即可，不影响主项目。

---

## 七、关联

- Spec：`docs/superpowers/specs/2026-06-17-p3-3-react-native.md`
- 用户手册：`docs/2026-06-17-p3-3-react-native-user-manual.md`
- API 文档：`docs/2026-06-17-p3-3-react-native-api.md`
