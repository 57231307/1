# 秉羲前端迁移至 Vue 实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 将前端从 Yew/WASM 替换为 Vue 3.4 + TypeScript + Element Plus，保留 Rust 后端

**架构：** Vue 3 SPA 前端 + Rust Axum 后端（添加 CORS），使用 Vite 构建，Pinia 状态管理，Axios HTTP 客户端

**技术栈：** Vue 3.4, TypeScript 5.x, Element Plus 2.x, Vite 5.x, Pinia 2.x, vue-router 4.x, Axios 1.x

---

## 文件清单

### 删除的文件
| 文件/目录 | 说明 |
|-----------|------|
| `frontend/src/` | 整个 Rust 源码目录 |
| `frontend/Cargo.toml` | Rust Cargo 配置 |
| `frontend/Trunk.toml` | Trunk 构建配置 |
| `frontend/index.html` | 旧版 HTML |
| `frontend/static/` | 旧版静态资源 |

### 新增的文件（Vue 项目）
| 文件 | 职责 |
|------|------|
| `frontend/package.json` | npm 包配置 |
| `frontend/vite.config.ts` | Vite 构建配置 |
| `frontend/tsconfig.json` | TypeScript 配置 |
| `frontend/index.html` | SPA 入口 HTML |
| `frontend/src/main.ts` | Vue 应用入口 |
| `frontend/src/App.vue` | 根组件 |
| `frontend/src/api/request.ts` | Axios 封装 |
| `frontend/src/router/index.ts` | 路由配置 |
| `frontend/src/store/user.ts` | 用户状态 |
| `frontend/src/store/permission.ts` | 权限状态 |
| `frontend/src/views/Login.vue` | 登录页 |
| `frontend/src/views/Dashboard.vue` | 仪表盘 |
| `frontend/src/components/Layout/` | 布局组件 |

### 修改的文件
| 文件 | 修改内容 |
|------|---------|
| `backend/src/routes/mod.rs` | 添加 CORS 层 |
| `backend/src/main.rs` | 添加 CORS 中间件 |

---

## 任务 1：删除旧前端代码

- [ ] **步骤 1：删除 Rust 前端源码**

```bash
cd /workspace
rm -rf frontend/src frontend/Cargo.toml frontend/Trunk.toml frontend/index.html frontend/static
```

- [ ] **步骤 2：验证删除**

```bash
ls frontend/
# 应该只剩下空的目录结构或 docs/
```

- [ ] **步骤 3：Commit**

```bash
cd /workspace
git add -A frontend/
git commit -m "chore: remove Yew/WASM frontend code"
```

---

## 任务 2：创建 Vue 项目基础结构

- [ ] **步骤 1：创建 package.json**

```json
{
  "name": "bingxi-frontend",
  "version": "2026.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vue-tsc -b && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "vue": "^3.4.0",
    "vue-router": "^4.3.0",
    "pinia": "^2.1.0",
    "element-plus": "^2.6.0",
    "axios": "^1.6.0",
    "@element-plus/icons-vue": "^2.3.0"
  },
  "devDependencies": {
    "@vitejs/plugin-vue": "^5.0.0",
    "vite": "^5.2.0",
    "vue-tsc": "^2.0.0",
    "typescript": "~5.4.0",
    "unplugin-auto-import": "^0.17.0",
    "unplugin-vue-components": "^0.26.0"
  }
}
```

- [ ] **步骤 2：创建 vite.config.ts**

```typescript
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import AutoImport from 'unplugin-auto-import/vite'
import Components from 'unplugin-vue-components/vite'
import { ElementPlusResolver } from 'unplugin-vue-components/resolvers'
import { resolve } from 'path'

export default defineConfig({
  plugins: [
    vue(),
    AutoImport({
      resolvers: [ElementPlusResolver()],
    }),
    Components({
      resolvers: [ElementPlusResolver()],
    }),
  ],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
  server: {
    port: 3000,
    proxy: {
      '/api': {
        target: 'http://localhost:8082',
        changeOrigin: true,
      },
    },
  },
  build: {
    outDir: 'dist',
    assetsDir: 'static',
    sourcemap: false,
  },
})
```

- [ ] **步骤 3：创建 tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "module": "ESNext",
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "isolatedModules": true,
    "moduleDetection": "force",
    "noEmit": true,
    "jsx": "preserve",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"]
    }
  },
  "include": ["src/**/*.ts", "src/**/*.tsx", "src/**/*.vue", "env.d.ts"]
}
```

- [ ] **步骤 4：创建 index.html**

```html
<!DOCTYPE html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <link rel="icon" type="image/svg+xml" href="/vite.svg" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>秉羲面料管理系统</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

- [ ] **步骤 5：创建 env.d.ts**

```typescript
/// <reference types="vite/client" />

declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  const component: DefineComponent<{}, {}, any>
  export default component
}
```

- [ ] **步骤 6：安装依赖**

```bash
cd /workspace/frontend
npm install
```

- [ ] **步骤 7：Commit**

```bash
cd /workspace
git add frontend/
git commit -m "feat: initialize Vue 3 + TypeScript + Element Plus project"
```

---

## 任务 3：创建项目目录结构和核心模块

- [ ] **步骤 1：创建目录结构**

```bash
cd /workspace/frontend
mkdir -p src/{api,components/Layout,views/{fabric,inventory,sales,purchase,finance,production,system},router,store,utils,types,assets}
```

- [ ] **步骤 2：创建 src/main.ts**

```typescript
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import App from './App.vue'
import router from './router'

const app = createApp(App)

app.use(createPinia())
app.use(router)
app.use(ElementPlus, { locale: zhCn })

app.mount('#app')
```

- [ ] **步骤 3：创建 src/App.vue**

```vue
<template>
  <router-view />
</template>

<script setup lang="ts">
</script>

<style>
body {
  margin: 0;
  padding: 0;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}
</style>
```

- [ ] **步骤 4：创建 src/types/api.ts**

```typescript
export interface ApiResponse<T> {
  success: boolean
  data?: T
  error?: string
  message?: string
}

export interface PageResponse<T> {
  items: T[]
  total: number
  page: number
  page_size: number
}

export interface LoginRequest {
  username: string
  password: string
}

export interface LoginResponse {
  token: string
  refresh_token: string
  user: UserInfo
}

export interface UserInfo {
  id: number
  username: string
  real_name: string
  email: string
  phone: string
  role: string
  permissions: UserPermission[]
}

export interface UserPermission {
  resource: string
  action: string
  resource_id?: number
}
```

- [ ] **步骤 5：创建 src/utils/storage.ts**

```typescript
const TOKEN_KEY = 'access_token'
const REFRESH_TOKEN_KEY = 'refresh_token'

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY)
}

export function setToken(token: string): void {
  localStorage.setItem(TOKEN_KEY, token)
}

export function removeToken(): void {
  localStorage.removeItem(TOKEN_KEY)
  localStorage.removeItem(REFRESH_TOKEN_KEY)
}

export function getRefreshToken(): string | null {
  return localStorage.getItem(REFRESH_TOKEN_KEY)
}

export function setRefreshToken(token: string): void {
  localStorage.setItem(REFRESH_TOKEN_KEY, token)
}
```

- [ ] **步骤 6：验证项目可运行**

```bash
cd /workspace/frontend
npm run build
```

- [ ] **步骤 7：Commit**

```bash
cd /workspace
git add frontend/src/
git commit -m "feat: create core modules and project structure"
```

---

## 任务 4：实现 API 封装和 Axios 配置

- [ ] **步骤 1：创建 src/api/request.ts**

```typescript
import axios, { type AxiosRequestConfig } from 'axios'
import { ElMessage } from 'element-plus'
import { getToken, removeToken } from '@/utils/storage'
import type { ApiResponse } from '@/types/api'

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
    const res = response.data as ApiResponse<any>
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

- [ ] **步骤 2：创建 src/api/auth.ts**

```typescript
import request from './request'
import type { LoginRequest, LoginResponse, UserInfo } from '@/types/api'

export function login(data: LoginRequest): Promise<LoginResponse> {
  return request.post('/auth/login', data)
}

export function logout(): Promise<void> {
  return request.post('/auth/logout')
}

export function refreshToken(refreshToken: string): Promise<{ token: string }> {
  return request.post('/auth/refresh', { refresh_token: refreshToken })
}

export function getUserInfo(): Promise<UserInfo> {
  return request.get('/auth/me')
}
```

- [ ] **步骤 3：创建 src/api/index.ts**

```typescript
export * from './auth'
// 后续添加其他 API 模块
```

- [ ] **步骤 4：Commit**

```bash
cd /workspace
git add frontend/src/api/
git commit -m "feat: implement Axios API wrapper with interceptors"
```

---

## 任务 5：实现状态管理（Pinia）

- [ ] **步骤 1：创建 src/store/user.ts**

```typescript
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { login as loginApi, logout as logoutApi } from '@/api/auth'
import { setToken, removeToken, getRefreshToken, setRefreshToken } from '@/utils/storage'
import type { UserInfo, LoginRequest } from '@/types/api'

export const useUserStore = defineStore('user', () => {
  const token = ref<string | null>(null)
  const userInfo = ref<UserInfo | null>(null)

  async function login(loginData: LoginRequest) {
    const res = await loginApi(loginData)
    token.value = res.token
    setToken(res.token)
    if (res.refresh_token) {
      setRefreshToken(res.refresh_token)
    }
    userInfo.value = res.user
    return res
  }

  async function logout() {
    try {
      await logoutApi()
    } finally {
      token.value = null
      userInfo.value = null
      removeToken()
    }
  }

  function setUserInfo(info: UserInfo) {
    userInfo.value = info
  }

  return { token, userInfo, login, logout, setUserInfo }
})
```

- [ ] **步骤 2：创建 src/store/permission.ts**

```typescript
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { UserPermission } from '@/types/api'

export const usePermissionStore = defineStore('permission', () => {
  const permissions = ref<UserPermission[]>([])

  function setPermissions(perms: UserPermission[]) {
    permissions.value = perms
  }

  function hasPermission(resource: string, action: string): boolean {
    return permissions.value.some(
      (p) => p.resource === resource && (p.action === action || p.action === '*')
    )
  }

  function hasAnyPermission(resource: string): boolean {
    return permissions.value.some((p) => p.resource === resource)
  }

  const userResources = computed(() => {
    return [...new Set(permissions.value.map((p) => p.resource))]
  })

  return { permissions, setPermissions, hasPermission, hasAnyPermission, userResources }
})
```

- [ ] **步骤 3：创建 src/store/index.ts**

```typescript
export { useUserStore } from './user'
export { usePermissionStore } from './permission'
```

- [ ] **步骤 4：Commit**

```bash
cd /workspace
git add frontend/src/store/
git commit -m "feat: implement Pinia stores for user and permissions"
```

---

## 任务 6：实现路由和路由守卫

- [ ] **步骤 1：创建 src/router/index.ts**

```typescript
import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router'
import { getToken } from '@/utils/storage'
import { usePermissionStore } from '@/store/permission'

const routes: RouteRecordRaw[] = [
  {
    path: '/login',
    name: 'Login',
    component: () => import('@/views/Login.vue'),
    meta: { title: '登录' },
  },
  {
    path: '/',
    component: () => import('@/components/Layout/MainLayout.vue'),
    redirect: '/dashboard',
    children: [
      {
        path: 'dashboard',
        name: 'Dashboard',
        component: () => import('@/views/Dashboard.vue'),
        meta: { title: '仪表盘', requiresAuth: true },
      },
      // 面料管理
      {
        path: 'fabric',
        name: 'Fabric',
        component: () => import('@/views/fabric/index.vue'),
        meta: { title: '面料管理', requiresAuth: true, resource: 'fabric', action: 'view' },
      },
      // 库存管理
      {
        path: 'inventory',
        name: 'Inventory',
        component: () => import('@/views/inventory/index.vue'),
        meta: { title: '库存管理', requiresAuth: true, resource: 'inventory', action: 'view' },
      },
      // 销售管理
      {
        path: 'sales',
        name: 'Sales',
        component: () => import('@/views/sales/index.vue'),
        meta: { title: '销售管理', requiresAuth: true, resource: 'sales', action: 'view' },
      },
    ],
  },
  {
    path: '/403',
    name: 'Forbidden',
    component: () => import('@/views/403.vue'),
    meta: { title: '无权访问' },
  },
  {
    path: '/:pathMatch(.*)*',
    name: 'NotFound',
    component: () => import('@/views/404.vue'),
    meta: { title: '页面不存在' },
  },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

router.beforeEach(async (to) => {
  const token = getToken()
  const permissionStore = usePermissionStore()

  // 登录页直接放行
  if (to.name === 'Login') {
    if (token) {
      return { path: '/' }
    }
    return true
  }

  // 需要认证的页面
  if (to.meta.requiresAuth && !token) {
    return { name: 'Login', query: { redirect: to.fullPath } }
  }

  // 权限检查
  if (token && to.meta.resource) {
    const hasPerm = permissionStore.hasPermission(
      to.meta.resource as string,
      to.meta.action as string
    )
    if (!hasPerm) {
      return { path: '/403' }
    }
  }

  // 设置页面标题
  if (to.meta.title) {
    document.title = `${to.meta.title} - 秉羲面料管理`
  }

  return true
})

export default router
```

- [ ] **步骤 2：创建 src/views/403.vue**

```vue
<template>
  <div class="error-page">
    <el-result icon="warning" title="403" sub-title="您没有权限访问此页面">
      <template #extra>
        <el-button type="primary" @click="$router.push('/')">返回首页</el-button>
      </template>
    </el-result>
  </div>
</template>
```

- [ ] **步骤 3：创建 src/views/404.vue**

```vue
<template>
  <div class="error-page">
    <el-result icon="warning" title="404" sub-title="页面不存在">
      <template #extra>
        <el-button type="primary" @click="$router.push('/')">返回首页</el-button>
      </template>
    </el-result>
  </div>
</template>
```

- [ ] **步骤 4：Commit**

```bash
cd /workspace
git add frontend/src/router/ frontend/src/views/403.vue frontend/src/views/404.vue
git commit -m "feat: implement router with auth and permission guards"
```

---

## 任务 7：实现布局组件

- [ ] **步骤 1：创建 src/components/Layout/MainLayout.vue**

```vue
<template>
  <el-container class="main-layout">
    <el-aside width="220px" class="aside">
      <div class="logo">
        <h2>秉羲面料管理</h2>
      </div>
      <el-menu
        :default-active="activeMenu"
        class="menu"
        background-color="#304156"
        text-color="#bfcbd9"
        active-text-color="#409eff"
        router
      >
        <el-menu-item index="/dashboard">
          <el-icon><HomeFilled /></el-icon>
          <span>仪表盘</span>
        </el-menu-item>
        
        <el-sub-menu index="fabric">
          <template #title>
            <el-icon><Goods /></el-icon>
            <span>面料管理</span>
          </template>
          <el-menu-item index="/fabric">面料列表</el-menu-item>
        </el-sub-menu>

        <el-sub-menu index="inventory">
          <template #title>
            <el-icon><Box /></el-icon>
            <span>库存管理</span>
          </template>
          <el-menu-item index="/inventory">库存列表</el-menu-item>
        </el-sub-menu>

        <el-sub-menu index="sales">
          <template #title>
            <el-icon><ShoppingCart /></el-icon>
            <span>销售管理</span>
          </template>
          <el-menu-item index="/sales">销售订单</el-menu-item>
        </el-sub-menu>
      </el-menu>
    </el-aside>

    <el-container>
      <el-header class="header">
        <div class="header-left">
          <el-breadcrumb separator="/">
            <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
            <el-breadcrumb-item>{{ currentTitle }}</el-breadcrumb-item>
          </el-breadcrumb>
        </div>
        <div class="header-right">
          <el-dropdown>
            <span class="user-info">
              {{ userStore.userInfo?.real_name || '用户' }}
              <el-icon><ArrowDown /></el-icon>
            </span>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item @click="$router.push('/system/profile')">个人信息</el-dropdown-item>
                <el-dropdown-item divided @click="handleLogout">退出登录</el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
      </el-header>

      <el-main class="main-content">
        <router-view />
      </el-main>
    </el-container>
  </el-container>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { HomeFilled, Goods, Box, ShoppingCart, ArrowDown } from '@element-plus/icons-vue'
import { useUserStore } from '@/store'

const route = useRoute()
const router = useRouter()
const userStore = useUserStore()

const activeMenu = computed(() => route.path)
const currentTitle = computed(() => (route.meta.title as string) || '')

async function handleLogout() {
  await userStore.logout()
  router.push('/login')
}
</script>

<style scoped>
.main-layout {
  height: 100vh;
}
.aside {
  background-color: #304156;
}
.logo {
  height: 60px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: #263445;
}
.logo h2 {
  color: #fff;
  font-size: 18px;
  margin: 0;
}
.menu {
  border-right: none;
}
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: #fff;
  box-shadow: 0 1px 4px rgba(0,21,41,.08);
}
.header-right .user-info {
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 4px;
}
.main-content {
  background: #f0f2f5;
  padding: 20px;
}
</style>
```

- [ ] **步骤 2：Commit**

```bash
cd /workspace
git add frontend/src/components/Layout/
git commit -m "feat: implement main layout with sidebar and header"
```

---

## 任务 8：实现登录页面

- [ ] **步骤 1：创建 src/views/Login.vue**

```vue
<template>
  <div class="login-container">
    <div class="login-card">
      <h2 class="login-title">秉羲面料管理系统</h2>
      <el-form ref="formRef" :model="loginForm" :rules="rules" @submit.prevent="handleLogin">
        <el-form-item prop="username">
          <el-input
            v-model="loginForm.username"
            placeholder="用户名"
            prefix-icon="User"
            size="large"
          />
        </el-form-item>
        <el-form-item prop="password">
          <el-input
            v-model="loginForm.password"
            type="password"
            placeholder="密码"
            prefix-icon="Lock"
            size="large"
            show-password
            @keyup.enter="handleLogin"
          />
        </el-form-item>
        <el-form-item>
          <el-button
            type="primary"
            size="large"
            style="width: 100%"
            :loading="loading"
            @click="handleLogin"
          >
            登录
          </el-button>
        </el-form-item>
      </el-form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { ElMessage } from 'element-plus'
import type { FormInstance } from 'element-plus'
import { useUserStore, usePermissionStore } from '@/store'

const router = useRouter()
const route = useRoute()
const userStore = useUserStore()
const permissionStore = usePermissionStore()

const formRef = ref<FormInstance>()
const loading = ref(false)

const loginForm = reactive({
  username: '',
  password: '',
})

const rules = {
  username: [{ required: true, message: '请输入用户名', trigger: 'blur' }],
  password: [{ required: true, message: '请输入密码', trigger: 'blur' }],
}

async function handleLogin() {
  const form = formRef.value
  if (!form) return
  
  await form.validate(async (valid) => {
    if (!valid) return
    
    loading.value = true
    try {
      await userStore.login(loginForm)
      
      // 从用户信息中获取权限
      if (userStore.userInfo?.permissions) {
        permissionStore.setPermissions(userStore.userInfo.permissions)
      }
      
      ElMessage.success('登录成功')
      
      const redirect = (route.query.redirect as string) || '/'
      router.push(redirect)
    } catch (error: any) {
      ElMessage.error(error.message || '登录失败')
    } finally {
      loading.value = false
    }
  })
}
</script>

<style scoped>
.login-container {
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}
.login-card {
  width: 400px;
  padding: 40px;
  background: #fff;
  border-radius: 8px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.1);
}
.login-title {
  text-align: center;
  margin-bottom: 30px;
  color: #303133;
}
</style>
```

- [ ] **步骤 2：创建 src/views/Dashboard.vue**

```vue
<template>
  <div class="dashboard">
    <el-row :gutter="20">
      <el-col :span="6">
        <el-card shadow="hover">
          <template #header>面料总数</template>
          <div class="stat-value">{{ stats.fabricCount }}</div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <template #header>库存总量</template>
          <div class="stat-value">{{ stats.inventoryTotal }}</div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <template #header>本月订单</template>
          <div class="stat-value">{{ stats.monthOrders }}</div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <template #header>客户总数</template>
          <div class="stat-value">{{ stats.customerCount }}</div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'

const stats = ref({
  fabricCount: 0,
  inventoryTotal: 0,
  monthOrders: 0,
  customerCount: 0,
})

onMounted(() => {
  // 后续对接真实 API
  stats.value = {
    fabricCount: 156,
    inventoryTotal: 12500,
    monthOrders: 89,
    customerCount: 45,
  }
})
</script>

<style scoped>
.dashboard {
  padding: 20px;
}
.stat-value {
  font-size: 32px;
  font-weight: bold;
  color: #409eff;
  text-align: center;
}
</style>
```

- [ ] **步骤 3：创建占位页面**

```bash
cd /workspace/frontend
mkdir -p src/views/{fabric,inventory,sales}
echo '<template><div>面料管理</div></template>' > src/views/fabric/index.vue
echo '<template><div>库存管理</div></template>' > src/views/inventory/index.vue
echo '<template><div>销售管理</div></template>' > src/views/sales/index.vue
```

- [ ] **步骤 4：Commit**

```bash
cd /workspace
git add frontend/src/views/
git commit -m "feat: implement login page and dashboard with placeholder pages"
```

---

## 任务 9：添加后端 CORS 支持

- [ ] **步骤 1：修改 backend/src/routes/mod.rs**

在文件开头添加导入：
```rust
use tower_http::cors::CorsLayer;
```

在 `create_router` 函数末尾添加 CORS 层：

```rust
pub fn create_router(state: AppState) -> Router {
    // ... 现有路由配置 ...
    
    // 添加 CORS 层
    router.layer(
        CorsLayer::new()
            .allow_origin(tower_http::cors::Any)
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::PUT,
                axum::http::Method::DELETE,
                axum::http::Method::OPTIONS,
            ])
            .allow_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::header::AUTHORIZATION,
                axum::http::header::ACCEPT,
            ])
            .allow_credentials(true),
    )
}
```

- [ ] **步骤 2：编译验证**

```bash
cd /workspace/backend
cargo check
```

- [ ] **步骤 3：Commit**

```bash
cd /workspace
git add backend/src/routes/mod.rs
git commit -m "feat: add CORS support for Vue frontend"
```

---

## 自检清单

- [ ] 规格覆盖度：所有任务覆盖了设计文档中的需求
- [ ] 占位符扫描：无 "TODO"、"待定"
- [ ] 类型一致性：所有 TypeScript 类型在 `types/api.ts` 中统一定义
- [ ] 文件路径：所有路径使用精确路径
- [ ] 编译验证：每个任务后都有验证步骤

---

## 执行选项

**计划已完成并保存到 `/workspace/frontend/docs/superpowers/plans/2026-05-13-vue-migration.md`。**
