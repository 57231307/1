import { describe, it, expect } from 'vitest'
import { formatCurrency, formatDate, debounce } from '@/utils'

describe('工具函数测试', () => {
  describe('formatCurrency', () => {
    it('应该正确格式化金额', () => {
      expect(formatCurrency(1234.56)).toContain('1,234.56')
    })

    it('应该处理零值', () => {
      expect(formatCurrency(0)).toContain('0.00')
    })

    it('应该处理负数', () => {
      const result = formatCurrency(-100)
      expect(result).toContain('100.00')
      expect(result).toMatch(/-|负/) // 包含负号或负数标识
    })
  })

  describe('formatDate', () => {
    it('应该正确格式化日期', () => {
      const date = new Date('2026-05-16')
      const formatted = formatDate(date)
      expect(formatted).toContain('2026')
      expect(formatted).toContain('5')
      expect(formatted).toContain('16')
    })
  })

  describe('debounce', () => {
    it('应该延迟执行函数', async () => {
      let count = 0
      const fn = debounce(() => {
        count++
      }, 100)

      fn()
      fn()
      fn()

      expect(count).toBe(0)

      await new Promise((resolve) => setTimeout(resolve, 150))
      expect(count).toBe(1)
    })
  })
})
