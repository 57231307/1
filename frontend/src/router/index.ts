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
      {
        path: 'fabric',
        name: 'Fabric',
        component: () => import('@/views/fabric/index.vue'),
        meta: { title: '面料管理', requiresAuth: true, resource: 'fabric', action: 'view' },
      },
      {
        path: 'inventory',
        name: 'Inventory',
        component: () => import('@/views/inventory/index.vue'),
        meta: { title: '库存管理', requiresAuth: true, resource: 'inventory', action: 'view' },
      },
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

  if (to.name === 'Login') {
    if (token) {
      return { path: '/' }
    }
    return true
  }

  if (to.meta.requiresAuth && !token) {
    return { name: 'Login', query: { redirect: to.fullPath } }
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

  if (to.meta.title) {
    document.title = `${to.meta.title} - 秉羲面料管理`
  }

  return true
})

export default router
