import type { Directive, DirectiveBinding } from 'vue'
import { useUserStore } from '@/store/user'
// 批次 22 v5 P0-3 阶段 A：复用 router 守卫的 hasRoutePermission，保持行为一致
import { hasRoutePermission } from '@/router'

/**
 * 权限指令
 * 使用方式：
 * <el-button v-permission="'user:create'">创建用户</el-button>
 * <el-button v-permission="['user:create', 'user:update']">编辑用户</el-button>
 */
export const permission: Directive = {
  mounted(el: HTMLElement, binding: DirectiveBinding) {
    const { value } = binding
    if (!value) return

    const userStore = useUserStore()
    const user = userStore.userInfo

    if (!user) {
      el.parentNode?.removeChild(el)
      return
    }

    // 如果用户是管理员，拥有所有权限
    if (user.role_name === 'admin') return

    // 从用户信息中获取权限列表
    const permissions = user.permissions || []
    // 批次 22 v5 P0-3 阶段 A 修复：复用 router 守卫的 hasRoutePermission
    // 与守卫行为一致：通配符 + read/view 等价，避免指令与守卫判断不一致
    let hasPermission = false

    if (Array.isArray(value)) {
      hasPermission = value.some(perm => hasRoutePermission(perm, permissions))
    } else {
      hasPermission = hasRoutePermission(value, permissions)
    }

    if (!hasPermission) {
      el.parentNode?.removeChild(el)
    }
  },
}

/**
 * 角色指令
 * 使用方式：
 * <el-button v-role="'admin'">管理员操作</el-button>
 * <el-button v-role="['admin', 'manager']">管理操作</el-button>
 */
export const role: Directive = {
  mounted(el: HTMLElement, binding: DirectiveBinding) {
    const { value } = binding
    if (!value) return

    const userStore = useUserStore()
    const userRole = userStore.userInfo?.role_name || ''

    let hasRole = false

    if (Array.isArray(value)) {
      hasRole = value.includes(userRole)
    } else {
      hasRole = userRole === value
    }

    if (!hasRole) {
      el.parentNode?.removeChild(el)
    }
  },
}
