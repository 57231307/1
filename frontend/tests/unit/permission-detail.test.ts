import { describe, it, expect, vi } from 'vitest';

// src/router/index.ts 顶层 import MainLayout（重组件，会牵出整条应用依赖链）；
// 本测试只针对其中导出的纯权限判定函数，故把 MainLayout 桩掉，避免加载无关组件树。
vi.mock('@/components/Layout/MainLayout.vue', () => ({
  default: { name: 'MainLayoutStub', render: () => null },
}));

import { canAccessDetailPermission, hasRoutePermission } from '@/router';

// 后端事实基准（matches_permission 与 /auth/me 出参序列化）：
// - 角色级授权行（resource_id 为 NULL）覆盖该资源类型的全部实例，列表请求与按 ID 请求都放行；
//   记录级授权行只放行同一实例；resource_type == "*" 豁免 ID 维度判定。
// - 登录/me 把每行权限序列化为 "{resource}:{action}"，不携带 resource_id，
//   所以前端可见的权限码一律对应角色级授权，其 /{id} 可达性只取决于资源段与动作段。
// 资源段判据：相等，或任一侧为 "*"（admin 的 *:* 即此形态）。
// 销售订单的权限资源段是 orders：sales 只是模块前缀，派生名保留 URL 第四段原名。
describe('canAccessDetailPermission（/{id} 详情/编辑同源判定）', () => {
  it('管理员（*:*）对任意资源/动作的详情与编辑均可访问', () => {
    const admin = ['*:*'];
    expect(canAccessDetailPermission('orders', 'read', admin)).toBe(true);
    expect(canAccessDetailPermission('purchase-orders', 'update', admin)).toBe(true);
    expect(canAccessDetailPermission('vouchers', 'delete', admin)).toBe(true);
  });

  it('角色级授权码（resource_id 为 NULL 派生）覆盖该资源的 /{id} 详情与编辑', () => {
    const salesRole = ['orders:read', 'orders:create', 'orders:update', 'orders:delete'];
    expect(hasRoutePermission('orders:read', salesRole)).toBe(true);
    expect(canAccessDetailPermission('orders', 'read', salesRole)).toBe(true);
    expect(canAccessDetailPermission('orders', 'update', salesRole)).toBe(true);
  });

  it('资源段相等但动作未覆盖时不放行（只授 read 的角色打不开编辑入口）', () => {
    const readOnly = ['orders:read'];
    expect(canAccessDetailPermission('orders', 'read', readOnly)).toBe(true);
    expect(canAccessDetailPermission('orders', 'update', readOnly)).toBe(false);
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

  it('资源动作通配码（resource:*）覆盖该资源全部动作的详情/编辑', () => {
    const manager = ['orders:*', 'orders:read'];
    expect(hasRoutePermission('orders:read', manager)).toBe(true);
    expect(canAccessDetailPermission('orders', 'read', manager)).toBe(true);
    expect(canAccessDetailPermission('orders', 'delete', manager)).toBe(true);
  });

  it('资源段不参与通配时跨资源不放行（持 orders 授权打不开 purchase-orders 详情）', () => {
    const manager = ['orders:*'];
    expect(canAccessDetailPermission('purchase-orders', 'read', manager)).toBe(false);
    expect(canAccessDetailPermission('vouchers', 'read', manager)).toBe(false);
  });

  it('空权限用户：详情/编辑不可访问（与守卫"无权限不放行"一致，无兜底放行）', () => {
    expect(canAccessDetailPermission('orders', 'read', [])).toBe(false);
    expect(canAccessDetailPermission('orders', 'update', [])).toBe(false);
  });

  it('无冒号或缺段权限码不会误判为详情可访问', () => {
    expect(canAccessDetailPermission('orders', 'read', ['orders'])).toBe(false);
    expect(canAccessDetailPermission('orders', 'read', ['*:'])).toBe(false);
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
