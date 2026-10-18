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
 * 详情/编辑（`/{id}`）入口可访问性指令
 * 入参 resource 为后端派生的权威资源段、action 为详情 read 或编辑 update，交给
 * router 导出的 canAccessDetailPermission 判定（与后端 matches_permission 同源）：
 * 角色级授权码（如 orders:read）覆盖该资源全部实例，含按 ID 的详情与编辑请求；
 * 判定为假时移除该元素，避免留下点了必然 403 的入口。
 * 使用方式：
 * <el-button v-permission-detail="{ resource: 'orders', action: 'read' }">详情</el-button>
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
