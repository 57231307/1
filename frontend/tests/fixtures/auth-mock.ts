/**
 * 认证/安全域测试 mock 数据夹具（V15 P2 B06-P2-4 修复）
 * 规则 6：测试 mock 数据禁止硬编码在测试用例中，统一抽取到 fixtures。
 * 使用 createXxxMock(overrides?) 工厂函数模式，便于通过 overrides 灵活定制。
 */
import type { ApiResponse } from '@/types/api';
import type { LockStatus } from '@/api/security';

/** 创建账号锁定状态 mock（默认未锁定，可通过 overrides 覆盖） */
export function createLockStatusMock(overrides: Partial<LockStatus> = {}): LockStatus {
  return {
    user_id: 1,
    username: 'admin',
    is_locked: false,
    failed_attempts: 0,
    locked_until: null,
    max_attempts: 5,
    ...overrides,
  };
}

/** 创建锁定状态 API 响应 mock（包裹 ApiResponse.data 结构） */
export function createLockStatusResponseMock(
  overrides: Partial<LockStatus> = {}
): Partial<ApiResponse<LockStatus>> {
  return {
    data: createLockStatusMock(overrides),
  };
}

/** 创建已锁定账号状态 mock（锁定中，3 次失败，5 分钟后解锁） */
export function createLockedStatusMock(overrides: Partial<LockStatus> = {}): LockStatus {
  const lockedUntil = new Date(Date.now() + 5 * 60 * 1000).toISOString();
  return createLockStatusMock({
    is_locked: true,
    failed_attempts: 5,
    locked_until: lockedUntil,
    ...overrides,
  });
}

/** 路由对象 mock（用于 useRoute 返回值，可通过 overrides 覆盖） */
export function createRouteMock(overrides: Partial<RouteMock> = {}): RouteMock {
  return {
    path: '/login',
    query: {},
    params: {},
    meta: {},
    ...overrides,
  };
}

/** 路由 mock 类型（对齐 vue-router RouteLocationNormalized 关键字段） */
export interface RouteMock {
  path: string;
  query: Record<string, string>;
  params: Record<string, string>;
  meta: Record<string, unknown>;
}

/** 创建带 redirect 参数的路由 mock（用于登录后跳转测试） */
export function createRouteWithRedirectMock(
  redirect: string,
  overrides: Partial<RouteMock> = {}
): RouteMock {
  return createRouteMock({
    query: { redirect },
    ...overrides,
  });
}
