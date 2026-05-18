# 面料管理系统前端架构迁移设计文档

> **日期**: 2026-05-13  
> **目标**: 将前端从 Yew/WASM 替换为 Vue 3.4 + Element Plus，保留 Rust 后端  
> **范围**: 前端完全重写，后端 API 适配

---

## 1. 架构概述

### 1.1 技术栈对比

| 层级 | 当前（删除） | 新架构 |
|------|-------------|--------|
| **前端框架** | Yew 0.21 + WASM | Vue 3.4 + TypeScript |
| **UI 组件库** | 手写 CSS | Element Plus |
| **路由** | yew-router | vue-router 4 |
| **状态管理** | 无（Yew Context） | Pinia |
| **HTTP 客户端** | gloo-net | Axios |
| **构建工具** | Trunk | Vite |
| **后端** | Rust + Axum（保留） | Rust + Axum（适配 API 格式）|

### 1.2 架构图

```
┌─────────────────────────────────────┐
│           Vue 3 前端                 │
│  Vue 3.4 + TypeScript + Vite        │
├─────────────────────────────────────┤
│  ┌──────────┐  ┌────────────────┐   │
│  │ Pinia    │  │ Vue Router     │   │
│  │ 状态管理  │  │ 路由守卫        │   │
│  └──────────┘  └────────────────┘   │
├─────────────────────────────────────┤
│  ┌──────────┐  ┌────────────────┐   │
│  │ Axios    │  │ Element Plus   │   │
│  │ API 封装  │  │ UI 组件库      │   │
│  └──────────┘  └────────────────┘   │
└─────────────────────────────────────┘
          │ HTTP + JSON
          ▼
┌─────────────────────────────────────┐
│           Rust 后端（保留）          │
│  Axum + JWT + MySQL                 │
├─────────────────────────────────────┤
│  ┌──────────┐  ┌────────────────┐   │
│  │ Auth     │  │ API Handlers   │   │
│  │ JWT      │  │ 业务逻辑        │   │
│  └──────────┘  └────────────────┘   │
└─────────────────────────────────────┘
```

---

## 2. 前端架构设计

### 2.1 目录结构

```
frontend/                  # 替换现有 frontend/
├── package.json
├── vite.config.ts
├── tsconfig.json
├── index.html
├── src/
│   ├── main.ts                    # 应用入口
│   ├── App.vue                    # 根组件
│   ├── api/                       # API 层
│   │   ├── request.ts             # Axios 封装
│   │   ├── auth.ts                # 认证 API
│   │   ├── fabric.ts              # 面料 API
│   │   ├── inventory.ts           # 库存 API
│   │   ├── sales.ts               # 销售 API
│   │   └── ...                    # 其他 API 模块
│   ├── components/                # 通用组件
│   │   ├── Layout/                # 布局组件
│   │   │   ├── Header.vue
│   │   │   ├── Sidebar.vue
│   │   │   └── Main.vue
│   │   ├── PermissionGuard.vue    # 权限守卫组件
│   │   ├── DataTable.vue          # 数据表格组件
│   │   └── ...
│   ├── views/                     # 页面视图
│   │   ├── Login.vue              # 登录页
│   │   ├── Dashboard.vue          # 仪表盘
│   │   ├── fabric/                # 面料管理
│   │   ├── inventory/             # 库存管理
│   │   ├── sales/                 # 销售管理
│   │   └── ...                    # 其他业务模块
│   ├── router/                    # 路由配置
│   │   └── index.ts
│   ├── store/                     # Pinia 状态管理
│   │   ├── user.ts                # 用户状态
│   │   ├── permission.ts          # 权限状态
│   │   └── app.ts                 # 应用状态
│   ├── utils/                     # 工具函数
│   │   ├── auth.ts                # 认证工具
│   │   ├── permissions.ts         # 权限工具
│   │   └── storage.ts             # 本地存储
│   └── types/                     # TypeScript 类型定义
│       ├── api.ts
│       └── models.ts
└── public/                        # 静态资源
```

### 2.2 核心模块设计

#### API 封装（Axios + 后端适配）

```typescript
// src/api/request.ts
import axios from 'axios'
import { ElMessage } from 'element-plus'
import { getToken, removeToken } from '@/utils/auth'

const service = axios.create({
  baseURL: import.meta.env.VITE_API_BASE || '/api/v1/erp',
  timeout: 15000,
})

// 请求拦截器
service.interceptors.request.use(
  (config) => {
    const token = getToken()
    if (token) {
      config.headers.Authorization = `Bearer ${token}`
    }
    return config
  },
  (error) => Promise.reject(error)
)

// 响应拦截器
service.interceptors.response.use(
  (response) => {
    // 适配后端 ApiResponse 格式
    const res = response.data
    if (res.success) {
      return res.data
    }
    ElMessage.error(res.error || res.message || '请求失败')
    return Promise.reject(new Error(res.error || res.message || '请求失败'))
  },
  (error) => {
    if (error.response?.status === 401) {
      removeToken()
      window.location.href = '/login'
    }
    ElMessage.error(error.message || '网络错误')
    return Promise.reject(error)
  }
)

export default service
```

#### 权限系统

```typescript
// src/store/permission.ts
import { defineStore } from 'pinia'

export const usePermissionStore = defineStore('permission', {
  state: () => ({
    permissions: [] as UserPermission[],
    roles: [] as string[],
  }),
  actions: {
    loadPermissions(token: string) {
      // 从后端获取权限
      return fetchPermissions().then((perms) => {
        this.permissions = perms
        return perms
      })
    },
    hasPermission(resource: string, action: string): boolean {
      return this.permissions.some(
        (p) =>
          p.resource === resource &&
          (p.action === action || p.action === '*')
      )
    },
  },
})
```

#### 路由守卫

```typescript
// src/router/index.ts
import { createRouter, createWebHistory } from 'vue-router'
import { getToken } from '@/utils/auth'
import { usePermissionStore } from '@/store/permission'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/login', component: () => import('@/views/Login.vue') },
    {
      path: '/',
      component: () => import('@/components/Layout/Main.vue'),
      children: [
        {
          path: 'dashboard',
          component: () => import('@/views/Dashboard.vue'),
          meta: { requiresAuth: true, resource: 'dashboard', action: 'view' },
        },
        // ... 其他路由
      ],
    },
  ],
})

router.beforeEach(async (to) => {
  const token = getToken()
  const permissionStore = usePermissionStore()

  if (to.meta.requiresAuth && !token) {
    return { name: 'Login' }
  }

  if (token && to.meta.resource) {
    const hasPerm = permissionStore.hasPermission(
      to.meta.resource as string,
      to.meta.action as string
    )
    if (!hasPerm) {
      return { path: '/403' }
    }
  }

  return true
})

export default router
```

---

## 3. 后端 API 适配

### 3.1 需要调整的后端 API

| API 端点 | 当前格式 | 适配方案 |
|---------|---------|---------|
| 认证 | JWT (已实现) | 保持不变，添加 CORS |
| 分页 | 自定义 | 改为 Element Plus 兼容格式 |
| 错误响应 | ApiResponse<T> | 保持，前端适配 |
| 文件上传 | multipart | 添加标准响应格式 |
| CORS | 可能未配置 | 必须添加 CORS 中间件 |

### 3.2 CORS 配置（必须添加）

在 `backend/src/main.rs` 中添加 CORS 层：

```rust
use tower_http::cors::{CorsLayer, Any};

// 在 router 配置中添加
.layer(CorsLayer::permissive())
```

---

## 4. 迁移范围

### 4.1 需要迁移的页面（80+）

按功能模块分组：

| 模块 | 页面数 | 优先级 |
|------|--------|--------|
| 认证/登录 | 2 | P0 |
| 仪表盘 | 1 | P0 |
| 面料管理 | 10 | P1 |
| 库存管理 | 8 | P1 |
| 销售管理 | 12 | P1 |
| 采购管理 | 8 | P2 |
| 财务管理 | 10 | P2 |
| 生产管理 | 8 | P2 |
| 系统设置 | 6 | P3 |
| 其他 | 15 | P3 |

### 4.2 需要保留的后端功能

- [x] JWT 认证
- [x] 用户管理
- [x] 权限系统
- [x] 所有业务 API
- [x] 数据库连接
- [x] 日志记录

### 4.3 需要删除的前端文件

- [ ] 整个 `frontend/src/` (Rust 代码)
- [ ] `frontend/Cargo.toml`
- [ ] `frontend/Trunk.toml`
- [ ] `frontend/index.html` (旧版)

---

## 5. 实施计划

| 阶段 | 任务 | 工作量 | 依赖 |
|------|------|--------|------|
| **P0** | 搭建 Vue 3 项目框架 | 2 天 | 无 |
| **P0** | 实现 API 封装 + Axios | 2 天 | 无 |
| **P0** | 实现登录/认证 | 1 天 | API 封装 |
| **P0** | 实现布局框架 | 2 天 | 无 |
| **P1** | 迁移面料管理模块 | 3 天 | P0 |
| **P1** | 迁移库存管理模块 | 2 天 | P0 |
| **P1** | 迁移销售管理模块 | 3 天 | P0 |
| **P2** | 迁移其他业务模块 | 5 天 | P0 |
| **P2** | 实现权限系统 | 2 天 | P0 |
| **P3** | 系统设置页面 | 2 天 | P0 |
| **P3** | 测试和优化 | 3 天 | 全部 |

**总工作量：约 4 周**

---

## 6. 成功标准

- [ ] 前端使用 Vue 3.4 + TypeScript + Element Plus
- [ ] 所有 80+ 页面功能完整迁移
- [ ] 权限系统正常工作
- [ ] 后端 API 保持兼容
- [ ] Rust 前端代码完全删除
- [ ] 构建产物体积 < 2MB（gzip）

---

## 7. 风险评估

| 风险 | 可能性 | 影响 | 缓解措施 |
|------|--------|------|---------|
| 后端 API 格式不兼容 | 中 | 高 | 编写适配层 |
| 权限系统迁移遗漏 | 中 | 高 | 逐项对照现有权限逻辑 |
| 页面功能缺失 | 高 | 中 | 每个页面迁移后进行功能对比测试 |
| CORS 配置问题 | 低 | 高 | 提前配置并测试 |
