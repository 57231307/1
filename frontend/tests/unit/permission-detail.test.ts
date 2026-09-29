import { describe, it, expect, vi } from 'vitest';

// src/router/index.ts 顶层 import MainLayout（重组件，会牵出整条应用依赖链）；
// 本测试只针对其中导出的纯权限判定函数，故把 MainLayout 桩掉，避免加载无关组件树。
vi.mock('@/components/Layout/MainLayout.vue', () => ({
  default: { name: 'MainLayoutStub', render: () => null },
}));

import { canAccessDetailPermission, hasRoutePermission } from '@/router';

// 后端事实基准（backend/src/middleware/permission.rs + handlers/auth_handler.rs:176）：
// - 登录/me 把每行权限序列化为 "{resource}:{action}"，丢弃 resource_id；
// - matches_permission 对 resource_id=Some(id) 的 /{id} 详情/编辑请求，普通 NULL 行被拒，
//   仅 resource_type=="*"（admin 注入的 *:*）放行。
// 因此：非管理员权限码资源段永不为 "*" → 详情/编辑一律 false；管理员 "*" 通配 → true。
describe('canAccessDetailPermission（/{id} 详情/编辑同源判定）', () => {
  it('管理员（*:*）对任意资源/动作的详情与编辑均可访问', () => {
    const admin = ['*:*'];
    expect(canAccessDetailPermission('sales-orders', 'read', admin)).toBe(true);
    expect(canAccessDetailPermission('purchase-orders', 'update', admin)).toBe(true);
    expect(canAccessDetailPermission('vouchers', 'delete', admin)).toBe(true);
  });

  it('非管理员持有 resource_id=NULL 派生的普通权限码时，详情/编辑均不可访问（点了必然 403）', () => {
    const salesRole = [
      'sales-orders:read',
      'sales-orders:create',
      'sales-orders:update',
      'sales-orders:delete',
    ];
    // 关键：列表级 hasRoutePermission 为 true，但 /{id} 详情必须为 false（严格策略）
    expect(hasRoutePermission('sales-orders:read', salesRole)).toBe(true);
    expect(canAccessDetailPermission('sales-orders', 'read', salesRole)).toBe(false);
    expect(canAccessDetailPermission('sales-orders', 'update', salesRole)).toBe(false);
  });

  it('资源段为 * 但动作受限的通配码：仅覆盖对应动作的详情', () => {
    const readStar = ['*:read'];
    expect(canAccessDetailPermission('customers', 'read', readStar)).toBe(true);
    // read 通配不覆盖 update（详情编辑）
    expect(canAccessDetailPermission('customers', 'update', readStar)).toBe(false);
  });

  it('动作等价：view 权限码可放行 read 详情、edit 权限码可放行 update 编辑（与后端 action 命名兼容一致）', () => {
    expect(canAccessDetailPermission('customers', 'read', ['*:view'])).toBe(true);
    expect(canAccessDetailPermission('customers', 'update', ['*:edit'])).toBe(true);
    expect(canAccessDetailPermission('customers', 'edit', ['*:update'])).toBe(true);
  });

  it('资源级动作通配（resource:*，源自 NULL 行）不放行 /{id} 详情——它只能放行列表', () => {
    const manager = ['sales-orders:*', 'sales-orders:read'];
    // hasRoutePermission 宽松匹配会放行
    expect(hasRoutePermission('sales-orders:read', manager)).toBe(true);
    // 但 /{id} 详情：资源段是 "sales-orders" 不是 "*"，后端 matches_permission 对 Some(id) 拒绝
    expect(canAccessDetailPermission('sales-orders', 'read', manager)).toBe(false);
  });

  it('空权限用户：详情/编辑不可访问（与守卫"无权限不放行"一致，无兜底放行）', () => {
    expect(canAccessDetailPermission('sales-orders', 'read', [])).toBe(false);
    expect(canAccessDetailPermission('sales-orders', 'update', [])).toBe(false);
  });

  it('无冒号或缺段权限码不会误判为详情可访问', () => {
    expect(canAccessDetailPermission('sales-orders', 'read', ['salesorders'])).toBe(false);
    expect(canAccessDetailPermission('sales-orders', 'read', ['*:'])).toBe(false);
  });
});

describe('hasRoutePermission（回归：重构为共享 actionEquivalent 后行为不变）', () => {
  it('*:* 超级通配放行任意权限码', () => {
    expect(hasRoutePermission('users:delete', ['*:*'])).toBe(true);
  });
  it('资源:* 通配放行该资源任意动作', () => {
    expect(hasRoutePermission('sales-orders:approve', ['sales-orders:*'])).toBe(true);
  });
  it('read/view、update/edit 等价', () => {
    expect(hasRoutePermission('sales-orders:view', ['sales-orders:read'])).toBe(true);
    expect(hasRoutePermission('sales-orders:edit', ['sales-orders:update'])).toBe(true);
  });
  it('资源不匹配或动作不匹配返回 false', () => {
    expect(hasRoutePermission('users:read', ['roles:read'])).toBe(false);
    expect(hasRoutePermission('users:delete', ['users:read'])).toBe(false);
  });
  it('数组入参任一命中即通过', () => {
    expect(hasRoutePermission(['users:read', 'roles:read'], ['roles:read'])).toBe(true);
  });
  it('required 为空时放行（路由无权限要求）', () => {
    expect(hasRoutePermission(undefined, [])).toBe(true);
  });
});
