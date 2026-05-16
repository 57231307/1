import { describe, it, expect } from 'vitest'

// 示例工具函数
function formatCurrency(amount: number): string {
  return new Intl.NumberFormat('zh-CN', {
    style: 'currency',
    currency: 'CNY',
    minimumFractionDigits: 2,
  }).format(amount)
}

function formatDate(date: Date): string {
  return date.toLocaleDateString('zh-CN')
}

function debounce<T extends (...args: any[]) => any>(
  fn: T,
  delay: number
): (...args: Parameters<T>) => void {
  let timer: ReturnType<typeof setTimeout> | null = null
  return (...args: Parameters<T>) => {
    if (timer) clearTimeout(timer)
    timer = setTimeout(() => fn(...args), delay)
  }
}

describe('工具函数测试', () => {
  describe('formatCurrency', () => {
    it('应该正确格式化金额', () => {
      expect(formatCurrency(1234.56)).toContain('1,234.56')
    })

    it('应该处理零值', () => {
      expect(formatCurrency(0)).toContain('0.00')
    })

    it('应该处理负数', () => {
      expect(formatCurrency(-100)).toContain('-100.00')
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
