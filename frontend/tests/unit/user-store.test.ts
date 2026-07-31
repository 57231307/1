import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock the API modules before imports
vi.mock('@/api/auth', () => ({
  login: vi.fn(),
  logout: vi.fn(),
  refreshToken: vi.fn(),
}));

// Wave B-3：access_token / refresh_token 已迁出 localStorage（存于 httpOnly Cookie）
// 这里仅 mock 仍然存在的 csrf_token Cookie 工具
vi.mock('@/utils/storage', () => ({
  getCsrfToken: vi.fn().mockReturnValue(null),
  loadCsrfToken: vi.fn().mockReturnValue(null),
  clearCsrfToken: vi.fn(),
}));

// Use real Pinia for store tests
vi.mock('pinia', async importOriginal => {
  const actual = await importOriginal<typeof import('pinia')>();
  return actual;
});

import { setActivePinia, createPinia } from 'pinia';
import { useUserStore } from '@/store/user';
import * as authApi from '@/api/auth';
import type { LoginResponse, UserInfo } from '@/types/api';
// V15 P2 B06-P2-3 修复（规则 6）：内联 mock 数据抽取到 fixtures 工厂函数
import { createLoginResponseMock, createUserInfoMock } from '../fixtures';

describe('User Store 测试（Wave B-3 Cookie 模式）', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
  });

  it('应该有正确的初始状态', () => {
    const store = useUserStore();
    expect(store.token).toBeNull();
    expect(store.userInfo).toBeNull();
  });

  it('login 应该调用 API 并设置 userInfo（不再操作 localStorage）', async () => {
    // V15 P2 B06-P2-3 修复（规则 6）：使用 fixtures 工厂函数替代内联 mock
    const mockResponse = createLoginResponseMock({
      user: createUserInfoMock({ id: 1, username: 'admin' } as Partial<UserInfo>),
    }) as LoginResponse;
    // 测试期望 user 包含 role 字段（业务逻辑测试用，UserInfo 类型不含 role）
    (mockResponse.user as Record<string, unknown>).role = 'admin';
    vi.mocked(authApi.login).mockResolvedValue(mockResponse);

    const store = useUserStore();
    const result = await store.login({ username: 'admin', password: 'password' });

    expect(authApi.login).toHaveBeenCalledWith({ username: 'admin', password: 'password' });
    // FE-P-2/FE-P-3 修复：userStore.login() 将 LoginResponse.permissions 合并到 userInfo
    expect(store.userInfo).toEqual({
      id: 1,
      username: 'admin',
      role: 'admin',
      permissions: [],
    });
    // 凭据由后端 Cookie 管理，前端不再写入 localStorage
    expect(localStorage.getItem('access_token')).toBeNull();
    expect(localStorage.getItem('refresh_token')).toBeNull();
  });

  it('logout 应该调用 API 并清除状态（不再操作 localStorage）', async () => {
    vi.mocked(authApi.logout).mockResolvedValue(undefined);

    const store = useUserStore();
    // V15 P2 B06-P2-3 修复（规则 6）：使用 fixtures 工厂函数替代内联 mock
    const userInfo = createUserInfoMock({ id: 1, username: 'admin' });
    (userInfo as Record<string, unknown>).role = 'admin';
    store.userInfo = userInfo;

    await store.logout();

    expect(authApi.logout).toHaveBeenCalled();
    // 后端通过 Set-Cookie + max-age=0 清除所有登录态 Cookie
    expect(store.token).toBeNull();
    expect(store.userInfo).toBeNull();
  });

  it('logout 应该在 API 失败时仍然清除状态', async () => {
    vi.mocked(authApi.logout).mockImplementation(() => {
      return Promise.reject(new Error('Network error'));
    });

    const store = useUserStore();
    // V15 P2 B06-P2-3 修复（规则 6）：使用 fixtures 工厂函数替代内联 mock
    const userInfo = createUserInfoMock({ id: 1, username: 'admin' });
    (userInfo as Record<string, unknown>).role = 'admin';
    store.userInfo = userInfo;

    // store 使用 try/finally，即使 API 失败也会清除状态
    await expect(store.logout()).rejects.toThrow('Network error');

    expect(store.token).toBeNull();
    expect(store.userInfo).toBeNull();
  });

  it('setUserInfo 应该更新用户信息', () => {
    const store = useUserStore();
    // V15 P2 B06-P2-3 修复（规则 6）：使用 fixtures 工厂函数替代内联 mock
    const userInfo = createUserInfoMock({ id: 1, username: 'test' });
    (userInfo as Record<string, unknown>).role = 'user';

    store.setUserInfo(userInfo);
    expect(store.userInfo).toEqual(userInfo);
  });
});
