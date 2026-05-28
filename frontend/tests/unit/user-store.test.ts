import { describe, it, expect, vi, beforeEach } from 'vitest'

// Mock the API modules before imports
vi.mock('@/api/auth', () => ({
  login: vi.fn(),
  logout: vi.fn(),
  refreshToken: vi.fn(),
}))

vi.mock('@/utils/storage', () => ({
  getToken: vi.fn().mockReturnValue(null),
  setToken: vi.fn(),
  removeToken: vi.fn(),
  getRefreshToken: vi.fn().mockReturnValue(null),
  setRefreshToken: vi.fn(),
}))

// Use real Pinia for store tests
vi.mock('pinia', async (importOriginal) => {
  const actual = await importOriginal<typeof import('pinia')>()
  return actual
})

import { setActivePinia, createPinia } from 'pinia'
import { useUserStore } from '@/store/user'
import * as authApi from '@/api/auth'
import * as storage from '@/utils/storage'

describe('User Store 测试', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('应该有正确的初始状态', () => {
    const store = useUserStore()
    expect(store.token).toBeNull()
    expect(store.userInfo).toBeNull()
  })

  it('login 应该调用 API 并设置 token', async () => {
    const mockResponse = {
      data: {
        token: 'test-token-123',
        refresh_token: 'refresh-123',
        user: { id: 1, username: 'admin', role: 'admin' },
      },
    }
    vi.mocked(authApi.login).mockResolvedValue(mockResponse as any)

    const store = useUserStore()
    const result = await store.login({ username: 'admin', password: 'password' })

    expect(authApi.login).toHaveBeenCalledWith({ username: 'admin', password: 'password' })
    expect(storage.setToken).toHaveBeenCalledWith('test-token-123')
    expect(storage.setRefreshToken).toHaveBeenCalledWith('refresh-123')
    expect(store.token).toBe('test-token-123')
    expect(store.userInfo).toEqual({ id: 1, username: 'admin', role: 'admin' })
  })

  it('logout 应该调用 API 并清除状态', async () => {
    vi.mocked(authApi.logout).mockResolvedValue(undefined as any)

    const store = useUserStore()
    store.token = 'existing-token'
    store.userInfo = { id: 1, username: 'admin', role: 'admin' } as any

    await store.logout()

    expect(authApi.logout).toHaveBeenCalled()
    expect(storage.removeToken).toHaveBeenCalled()
    expect(store.token).toBeNull()
    expect(store.userInfo).toBeNull()
  })

  it('logout 应该在 API 失败时仍然清除状态', async () => {
    vi.mocked(authApi.logout).mockImplementation(() => {
      return Promise.reject(new Error('Network error'))
    })

    const store = useUserStore()
    store.token = 'existing-token'
    store.userInfo = { id: 1, username: 'admin', role: 'admin' } as any

    // The store uses try/finally, so state should be cleared even on error
    // But the error will propagate, so we need to catch it
    await expect(store.logout()).rejects.toThrow('Network error')

    expect(storage.removeToken).toHaveBeenCalled()
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
