import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { getToken, setToken, removeToken, getRefreshToken, setRefreshToken } from '@/utils/storage'

describe('Storage 工具函数测试', () => {
  beforeEach(() => {
    localStorage.clear()
    vi.clearAllMocks()
  })

  describe('getToken / setToken', () => {
    it('应该返回 null 当没有 token 时', () => {
      expect(getToken()).toBeNull()
    })

    it('应该正确设置和获取 token', () => {
      setToken('test-token-123')
      expect(getToken()).toBe('test-token-123')
    })

    it('应该覆盖已有的 token', () => {
      setToken('old-token')
      setToken('new-token')
      expect(getToken()).toBe('new-token')
    })
  })

  describe('removeToken', () => {
    it('应该移除 access_token', () => {
      setToken('test-token')
      removeToken()
      expect(getToken()).toBeNull()
    })

    it('应该同时移除 refresh_token', () => {
      setRefreshToken('refresh-token')
      removeToken()
      expect(getRefreshToken()).toBeNull()
    })
  })

  describe('getRefreshToken / setRefreshToken', () => {
    it('应该返回 null 当没有 refresh_token 时', () => {
      expect(getRefreshToken()).toBeNull()
    })

    it('应该正确设置和获取 refresh_token', () => {
      setRefreshToken('refresh-123')
      expect(getRefreshToken()).toBe('refresh-123')
    })
  })
})
