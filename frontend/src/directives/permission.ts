import type { Directive, DirectiveBinding } from 'vue';
import { useUserStore } from '@/store/user';
// 批次 22 v5 P0-3 阶段 A：复用 router 守卫的 hasRoutePermission，保持行为一致
import { hasRoutePermission, canAccessDetailPermission } from '@/router';

/** v-permission-detail 指令值：目标资源类型 + 动作（详情用 read / 编辑用 update） */
export interface PermissionDetailBinding {
  resource: string;
  action: string;
}

/**
 * 权限指令
 * 使用方式：
 * <el-button v-permission="'users:create'">创建用户</el-button>
 * <el-button v-permission="['users:create', 'users:update']">编辑用户</el-button>
 */
export const permission: Directive = {
  mounted(el: HTMLElement, binding: DirectiveBinding) {
    const { value } = binding;
    if (!value) return;

    const userStore = useUserStore();
    const user = userStore.userInfo;

    if (!user) {
      el.parentNode?.removeChild(el);
      return;
    }

    // P2 1-12 修复：删除 role_name === 'admin' 硬编码绕过，
    // 改为后端为 system 角色注入 *:* 通配权限，hasRoutePermission 自动处理通配符
    // 从用户信息中获取权限列表
    const permissions = user.permissions || [];
    // 批次 22 v5 P0-3 阶段 A 修复：复用 router 守卫的 hasRoutePermission
    // 与守卫行为一致：通配符 + read/view 等价，避免指令与守卫判断不一致
    let hasPermission = false;

    if (Array.isArray(value)) {
      hasPermission = value.some(perm => hasRoutePermission(perm, permissions));
    } else {
      hasPermission = hasRoutePermission(value, permissions);
    }

    if (!hasPermission) {
      el.parentNode?.removeChild(el);
    }
  },
};

/**
 * 详情/编辑入口可访问性指令
 * 后端对 resource_id=NULL 的权限行拒绝 `/{id}` 详情/编辑请求（故意严格策略），
 * 非管理员角色点了必然 403。本指令按与后端 canAccessDetailPermission 同源的条件，
 * 直接移除（隐藏）用户无权限访问的"详情/编辑"入口，而不是让按钮存在并抛错。
 * 使用方式：
 * <el-button v-permission-detail="{ resource: 'sales-orders', action: 'read' }">详情</el-button>
 */
export const permissionDetail: Directive = {
  mounted(el: HTMLElement, binding: DirectiveBinding<PermissionDetailBinding>) {
    const { value } = binding;
    if (!value || !value.resource || !value.action) return;

    const userStore = useUserStore();
    const permissions = userStore.userInfo?.permissions || [];
    if (!canAccessDetailPermission(value.resource, value.action, permissions)) {
      el.parentNode?.removeChild(el);
    }
  },
};
