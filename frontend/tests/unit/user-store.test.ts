import { describe, it, expect, vi, beforeEach } from 'vitest'

// Mock the API modules before imports
vi.mock('@/api/auth', () => ({
  login: vi.fn(),
  logout: vi.fn(),
  refreshToken: vi.fn(),
}))

// Wave B-3：access_token / refresh_token 已迁出 localStorage（存于 httpOnly Cookie）
// 这里仅 mock 仍然存在的 csrf_token Cookie 工具
vi.mock('@/utils/storage', () => ({
  getCsrfToken: vi.fn().mockReturnValue(null),
  loadCsrfToken: vi.fn().mockReturnValue(null),
  clearCsrfToken: vi.fn(),
}))

// Use real Pinia for store tests
vi.mock('pinia', async (importOriginal) => {
  const actual = await importOriginal<typeof import('pinia')>()
  return actual
})

import { setActivePinia, createPinia } from 'pinia'
import { useUserStore } from '@/store/user'
import * as authApi from '@/api/auth'

describe('User Store 测试（Wave B-3 Cookie 模式）', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('应该有正确的初始状态', () => {
    const store = useUserStore()
    expect(store.token).toBeNull()
    expect(store.userInfo).toBeNull()
  })

  it('login 应该调用 API 并设置 userInfo（不再操作 localStorage）', async () => {
    const mockResponse = {
      data: {
        user: { id: 1, username: 'admin', role: 'admin' },
        permissions: [],
      },
    }
    vi.mocked(authApi.login).mockResolvedValue(mockResponse as any)

    const store = useUserStore()
    const result = await store.login({ username: 'admin', password: 'password' })

    expect(authApi.login).toHaveBeenCalledWith({ username: 'admin', password: 'password' })
    expect(store.userInfo).toEqual({ id: 1, username: 'admin', role: 'admin' })
    // 凭据由后端 Cookie 管理，前端不再写入 localStorage
    expect(localStorage.getItem('access_token')).toBeNull()
    expect(localStorage.getItem('refresh_token')).toBeNull()
  })

  it('logout 应该调用 API 并清除状态（不再操作 localStorage）', async () => {
    vi.mocked(authApi.logout).mockResolvedValue(undefined as any)

    const store = useUserStore()
    store.userInfo = { id: 1, username: 'admin', role: 'admin' } as any

    await store.logout()

    expect(authApi.logout).toHaveBeenCalled()
    // 后端通过 Set-Cookie + max-age=0 清除所有登录态 Cookie
    expect(store.token).toBeNull()
    expect(store.userInfo).toBeNull()
  })

  it('logout 应该在 API 失败时仍然清除状态', async () => {
    vi.mocked(authApi.logout).mockImplementation(() => {
      return Promise.reject(new Error('Network error'))
    })

    const store = useUserStore()
    store.userInfo = { id: 1, username: 'admin', role: 'admin' } as any

    // The store uses try/finally, so state should be cleared even on error
    // But the error will propagate, so we need to catch it
    await expect(store.logout()).rejects.toThrow('Network error')

    expect(store.token).toBeNull()
    expect(store.userInfo).toBeNull()
  })

  it('setUserInfo 应该更新用户信息', () => {
    const store = useUserStore()
    const userInfo = { id: 1, username: 'test', role: 'user' } as any

    store.setUserInfo(userInfo)
    expect(store.userInfo).toEqual(userInfo)
  })
})
