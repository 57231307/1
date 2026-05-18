# 面料管理系统前端架构文档

> **版本**: 2026.1.0  
> **技术栈**: Vue 3.4 + TypeScript 5.4 + Element Plus 2.6  
> **构建工具**: Vite 5.2  
> **状态管理**: Pinia 2.1

---

## 目录

1. [架构概述](#架构概述)
2. [项目结构](#项目结构)
3. [核心模块详解](#核心模块详解)
4. [开发规范](#开发规范)
5. [性能优化](#性能优化)
6. [部署说明](#部署说明)

---

## 架构概述

### 技术选型

| 技术 | 版本 | 用途 |
|------|------|------|
| Vue | 3.4 | 前端框架，提供响应式数据绑定和组件化开发 |
| TypeScript | 5.4 | 类型系统，提供静态类型检查 |
| Element Plus | 2.6 | UI组件库，提供企业级组件 |
| Vue Router | 4.3 | 路由管理，支持动态路由和路由守卫 |
| Pinia | 2.1 | 状态管理，Vue官方推荐 |
| Axios | 1.6 | HTTP客户端，支持拦截器 |
| Vite | 5.2 | 构建工具，支持热更新和快速构建 |

### 架构特点

1. **组件化设计**：页面拆分为独立组件，提高复用性
2. **类型安全**：全项目使用TypeScript，减少运行时错误
3. **状态集中管理**：使用Pinia管理全局状态
4. **权限控制**：路由守卫 + 指令级权限控制
5. **自动化导入**：使用unplugin-auto-import简化开发

---

## 项目结构

```
frontend/
├── package.json              # npm配置
├── vite.config.ts            # Vite构建配置
├── tsconfig.json             # TypeScript配置
├── index.html                # 入口HTML
├── env.d.ts                  # 类型声明
├── auto-imports.d.ts         # 自动导入声明
├── components.d.ts           # 组件声明
├── src/
│   ├── main.ts               # 应用入口
│   ├── App.vue               # 根组件
│   │
│   ├── api/                  # API层
│   │   ├── request.ts        # Axios封装
│   │   ├── auth.ts           # 认证API
│   │   ├── index.ts          # API导出
│   │   └── ...               # 其他API模块
│   │
│   ├── components/           # 通用组件
│   │   ├── Layout/           # 布局组件
│   │   │   └── MainLayout.vue
│   │   └── ...               # 其他组件
│   │
│   ├── views/                # 页面视图
│   │   ├── Login.vue         # 登录页
│   │   ├── Dashboard.vue     # 仪表盘
│   │   ├── 403.vue           # 无权访问
│   │   ├── 404.vue           # 页面不存在
│   │   ├── fabric/           # 面料管理
│   │   ├── inventory/        # 库存管理
│   │   ├── sales/            # 销售管理
│   │   └── ...               # 其他模块
│   │
│   ├── router/               # 路由配置
│   │   └── index.ts
│   │
│   ├── store/                # Pinia状态管理
│   │   ├── index.ts          # Store导出
│   │   ├── user.ts           # 用户状态
│   │   └── permission.ts     # 权限状态
│   │
│   ├── utils/                # 工具函数
│   │   └── storage.ts        # 本地存储
│   │
│   └── types/                # TypeScript类型
│       ├── api.ts            # API类型
│       └── models.ts         # 数据模型
│
└── public/                   # 静态资源
```

---

## 核心模块详解

### 1. API层 (src/api/)

#### request.ts - Axios封装

```typescript
// 功能说明：
// 1. 统一配置baseURL和超时时间
// 2. 请求拦截器：自动添加Authorization头
// 3. 响应拦截器：统一处理错误和Token过期
// 4. 适配后端ApiResponse格式
```

**关键特性**：
- 自动携带JWT Token
- 401自动跳转登录页
- 统一错误提示
- 支持环境变量配置

#### auth.ts - 认证API

```typescript
// 提供的接口：
// - login(data: LoginRequest): Promise<LoginResponse>
// - logout(): Promise<void>
// - refreshToken(refreshToken: string): Promise<{ token: string }>
// - getUserInfo(): Promise<UserInfo>
```

### 2. 状态管理 (src/store/)

#### user.ts - 用户状态

```typescript
// 管理内容：
// - token: 当前登录令牌
// - userInfo: 用户信息（包含权限）
//
// 提供方法：
// - login(): 登录并存储Token
// - logout(): 退出并清理状态
// - setUserInfo(): 更新用户信息
```

#### permission.ts - 权限状态

```typescript
// 管理内容：
// - permissions: 用户权限列表
//
// 提供方法：
// - setPermissions(): 设置权限
// - hasPermission(resource, action): 检查权限
// - hasAnyPermission(resource): 检查是否有某资源权限
// - userResources: 获取用户可访问资源列表
```

### 3. 路由系统 (src/router/)

#### 路由配置

```typescript
// 路由元信息：
// - title: 页面标题
// - requiresAuth: 是否需要认证
// - resource: 资源标识（用于权限检查）
// - action: 操作标识（用于权限检查）
```

#### 路由守卫

```typescript
// 守卫逻辑：
// 1. 登录页：已登录则跳首页
// 2. 需认证页：未登录则跳登录页
// 3. 权限检查：无权限则跳403
// 4. 动态设置页面标题
```

### 4. 布局组件 (src/components/Layout/)

#### MainLayout.vue

```typescript
// 布局结构：
// - 左侧：Logo + 导航菜单
// - 顶部：面包屑 + 用户信息
// - 主内容区：路由视图
//
// 功能特性：
// - 响应式侧边栏
// - 动态菜单高亮
// - 用户信息下拉菜单
// - 退出登录
```

---

## 开发规范

### 命名规范

| 类型 | 规范 | 示例 |
|------|------|------|
| 组件 | PascalCase | UserList.vue |
| 文件 | camelCase | userService.ts |
| 变量 | camelCase | userName |
| 常量 | UPPER_SNAKE_CASE | API_BASE_URL |
| 类型 | PascalCase | UserInfo |
| 接口 | PascalCase + I前缀 | IUserService |

### 代码组织

1. **单一职责**：每个组件/函数只做一件事
2. **组合式API**：使用`<script setup>`语法
3. **类型优先**：所有数据定义类型
4. **注释规范**：复杂逻辑需注释说明"为什么"

### 组件规范

```vue
<template>
  <!-- 模板内容 -->
</template>

<script setup lang="ts">
// 1. 导入（按类型分组）
import { ref, computed } from 'vue'
import type { UserInfo } from '@/types/api'

// 2. 类型定义
interface Props {
  userId: number
}

// 3. Props和Emits
const props = defineProps<Props>()
const emit = defineEmits<{
  update: [user: UserInfo]
}>()

// 4. 响应式数据
const user = ref<UserInfo | null>(null)
const loading = ref(false)

// 5. 计算属性
const userName = computed(() => user.value?.real_name || '')

// 6. 方法
async function fetchUser() {
  // 实现
}

// 7. 生命周期
onMounted(() => {
  fetchUser()
})
</script>

<style scoped>
/* 样式 */
</style>
```

---

## 性能优化

### 1. 构建优化

- **代码分割**：路由懒加载
- **Tree Shaking**：自动移除未使用代码
- **Gzip压缩**：生产环境启用
- **CDN加速**：静态资源可配置CDN

### 2. 运行时优化

- **v-memo**：大数据列表使用
- **shallowRef**：大数据对象使用浅响应
- **虚拟滚动**：长列表使用Element Plus的虚拟滚动
- **防抖节流**：搜索输入使用防抖

### 3. 网络优化

- **请求缓存**：GET请求合理缓存
- **请求合并**：并行请求使用Promise.all
- **懒加载**：图片和组件按需加载

---

## 部署说明

### 构建命令

```bash
# 开发环境
cd frontend
npm run dev

# 生产构建
npm run build

# 预览构建产物
npm run preview
```

### 环境变量

```bash
# .env.development
VITE_API_BASE=/api/v1/erp

# .env.production
VITE_API_BASE=/api/v1/erp
```

### 构建输出

构建产物位于 `frontend/dist/` 目录：
- `index.html` - 入口文件
- `static/` - 静态资源（JS/CSS/图片）

### Nginx配置

```nginx
server {
    listen 80;
    server_name localhost;
    root /var/www/bingxi/frontend/dist;
    index index.html;

    # 前端路由支持
    location / {
        try_files $uri $uri/ /index.html;
    }

    # API代理
    location /api/ {
        proxy_pass http://localhost:8082;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }

    # 静态资源缓存
    location /static/ {
        expires 1y;
        add_header Cache-Control "public, immutable";
    }
}
```

---

## 更新日志

### 2026.1.0 (当前版本)

- 前端从Yew/WASM迁移至Vue 3 + TypeScript
- 引入Element Plus组件库
- 使用Pinia替换原有状态管理
- 使用Vite替换Trunk构建工具

---

*本文档由前端团队维护*
