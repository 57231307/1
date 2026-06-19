import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
// Wave B-3：access_token / refresh_token 已迁出 localStorage，存于 httpOnly Cookie
// 这里只测试 csrf_token 的 Cookie 读取工具
import { getCsrfToken, loadCsrfToken, clearCsrfToken } from '@/utils/storage'

describe('Storage 工具函数测试（Wave B-3 Cookie 模式）', () => {
  // 设置测试用 document.cookie 模拟
  const setMockCookie = (cookieString: string) => {
    Object.defineProperty(document, 'cookie', {
      writable: true,
      configurable: true,
      value: cookieString,
    })
  }

  beforeEach(() => {
    setMockCookie('')
    vi.clearAllMocks()
  })

  afterEach(() => {
    setMockCookie('')
  })

  describe('getCsrfToken / loadCsrfToken', () => {
    it('应该在无 Cookie 时返回 null', () => {
      expect(getCsrfToken()).toBeNull()
      expect(loadCsrfToken()).toBeNull()
    })

    it('应该从 document.cookie 正确读取 csrf_token', () => {
      setMockCookie('csrf_token=abc-123-xyz; other=val')
      expect(getCsrfToken()).toBe('abc-123-xyz')
      expect(loadCsrfToken()).toBe('abc-123-xyz')
    })

    it('应该处理 URL 编码的 Cookie 值', () => {
      setMockCookie('csrf_token=hello%20world')
      expect(getCsrfToken()).toBe('hello world')
    })

    it('getCsrfToken 与 loadCsrfToken 行为一致', () => {
      setMockCookie('csrf_token=same-token')
      expect(getCsrfToken()).toBe(loadCsrfToken())
    })
  })

  describe('clearCsrfToken', () => {
    it('应该是无副作用的占位函数（后端负责清除）', () => {
      // clearCsrfToken 在前端是 no-op：后端通过 Set-Cookie + max-age=0 清除
      setMockCookie('csrf_token=existing')
      expect(() => clearCsrfToken()).not.toThrow()
    })
  })
})
