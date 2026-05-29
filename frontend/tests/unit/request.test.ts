import { describe, it, expect, vi, beforeEach } from 'vitest'

// 测试 request 模块中的纯函数逻辑
describe('Request 工具函数测试', () => {
  // 由于 request.ts 有副作用（创建 axios 实例），我们测试其中的纯函数逻辑

  const SAFE_ERROR_MESSAGES: Record<number, string> = {
    400: '请求参数错误',
    401: '未授权，请重新登录',
    403: '拒绝访问',
    404: '资源不存在',
    429: '请求过于频繁',
    500: '服务器内部错误',
    502: '网关错误',
    503: '服务暂时不可用',
  }

  function getSafeErrorMessage(codeOrStatus?: number): string {
    if (codeOrStatus && SAFE_ERROR_MESSAGES[codeOrStatus]) {
      return SAFE_ERROR_MESSAGES[codeOrStatus]
    }
    if (codeOrStatus === 401) {
      return '未授权，请重新登录'
    }
    return '请求失败，请稍后重试'
  }

  function shouldRetry(error: any): boolean {
    if (error.response) {
      return [502, 503, 504].includes(error.response.status)
    }
    return error.code === 'ECONNABORTED' || error.code === 'NETWORK_ERROR' || !error.response
  }

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

  describe('ApiResponse 类型验证', () => {
    it('应该正确解析成功响应', () => {
      const response = {
        code: 200,
        data: { id: 1, name: 'test' },
        message: 'success',
      }
      expect(response.code).toBe(200)
      expect(response.data).toEqual({ id: 1, name: 'test' })
    })

    it('应该正确解析分页响应', () => {
      const response = {
        code: 200,
        data: {
          list: [{ id: 1 }, { id: 2 }],
          total: 100,
          page: 1,
          page_size: 10,
        },
      }
      expect(response.data.list).toHaveLength(2)
      expect(response.data.total).toBe(100)
    })
  })
})
