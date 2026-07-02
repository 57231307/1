import { describe, it, expect, vi } from 'vitest'

// Mock @/router 以避免加载完整路由配置（含 MainLayout 等重组件）
vi.mock('@/router', () => ({
  default: { push: vi.fn() },
}))

// Mock @/api/auth 以避免循环依赖和副作用
vi.mock('@/api/auth', () => ({
  refreshToken: vi.fn(),
}))

import {
  SAFE_ERROR_MESSAGES,
  getSafeErrorMessage,
  shouldRetry,
} from '@/api/request'

// 测试 request 模块中的纯函数逻辑（从源文件真实导入）
describe('Request 工具函数测试', () => {
  describe('SAFE_ERROR_MESSAGES', () => {
    it('应该包含所有预定义状态码消息', () => {
      expect(SAFE_ERROR_MESSAGES[400]).toBe('请求参数错误')
      expect(SAFE_ERROR_MESSAGES[401]).toBe('未授权，请重新登录')
      expect(SAFE_ERROR_MESSAGES[403]).toBe('拒绝访问')
      expect(SAFE_ERROR_MESSAGES[404]).toBe('资源不存在')
      expect(SAFE_ERROR_MESSAGES[429]).toBe('请求过于频繁')
      expect(SAFE_ERROR_MESSAGES[500]).toBe('服务器内部错误')
      expect(SAFE_ERROR_MESSAGES[502]).toBe('网关错误')
      expect(SAFE_ERROR_MESSAGES[503]).toBe('服务暂时不可用')
    })
  })

  describe('getSafeErrorMessage', () => {
    it('应该返回对应状态码的错误消息', () => {
      expect(getSafeErrorMessage(400)).toBe('请求参数错误')
      expect(getSafeErrorMessage(401)).toBe('未授权，请重新登录')
      expect(getSafeErrorMessage(403)).toBe('拒绝访问')
      expect(getSafeErrorMessage(404)).toBe('资源不存在')
      expect(getSafeErrorMessage(429)).toBe('请求过于频繁')
      expect(getSafeErrorMessage(500)).toBe('服务器内部错误')
      expect(getSafeErrorMessage(502)).toBe('网关错误')
      expect(getSafeErrorMessage(503)).toBe('服务暂时不可用')
    })

    it('应该返回默认消息当状态码未知时', () => {
      expect(getSafeErrorMessage(418)).toBe('请求失败，请稍后重试')
      expect(getSafeErrorMessage(undefined)).toBe('请求失败，请稍后重试')
    })

    it('应该正确处理 401 状态码', () => {
      expect(getSafeErrorMessage(401)).toBe('未授权，请重新登录')
    })
  })

  describe('shouldRetry', () => {
    it('应该重试 502 错误', () => {
      expect(shouldRetry({ response: { status: 502 } })).toBe(true)
    })

    it('应该重试 503 错误', () => {
      expect(shouldRetry({ response: { status: 503 } })).toBe(true)
    })

    it('应该重试 504 错误', () => {
      expect(shouldRetry({ response: { status: 504 } })).toBe(true)
    })

    it('不应该重试 400 错误', () => {
      expect(shouldRetry({ response: { status: 400 } })).toBe(false)
    })

    it('不应该重试 401 错误', () => {
      expect(shouldRetry({ response: { status: 401 } })).toBe(false)
    })

    it('不应该重试 500 错误', () => {
      expect(shouldRetry({ response: { status: 500 } })).toBe(false)
    })

    it('应该重试 ECONNABORTED 错误', () => {
      expect(shouldRetry({ code: 'ECONNABORTED' })).toBe(true)
    })

    it('应该重试 NETWORK_ERROR 错误', () => {
      expect(shouldRetry({ code: 'NETWORK_ERROR' })).toBe(true)
    })

    it('应该重试无 response 的错误', () => {
      expect(shouldRetry({})).toBe(true)
    })
  })
})
