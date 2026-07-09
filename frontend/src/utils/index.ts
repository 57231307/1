/**
 * 通用格式化与防抖工具函数
 */

/**
 * 格式化金额为人民币货币字符串
 * @param amount 金额数值
 * @returns 格式化后的货币字符串（如 "￥1,234.56"）
 */
export function formatCurrency(amount: number): string {
  return new Intl.NumberFormat('zh-CN', {
    style: 'currency',
    currency: 'CNY',
    minimumFractionDigits: 2,
  }).format(amount)
}

/**
 * 格式化日期为中文 locale 字符串
 * @param date 日期对象
 * @returns 格式化后的日期字符串
 */
export function formatDate(date: Date): string {
  return date.toLocaleDateString('zh-CN')
}

/**
 * 创建防抖函数：在延迟时间内多次调用仅执行最后一次
 * @param fn 需要防抖的函数
 * @param delay 延迟毫秒数
 * @returns 防抖后的函数
 */
export function debounce<T extends (...args: unknown[]) => unknown>(
  fn: T,
  delay: number
): (...args: Parameters<T>) => void {
  let timer: ReturnType<typeof setTimeout> | null = null
  return (...args: Parameters<T>) => {
    if (timer) clearTimeout(timer)
    timer = setTimeout(() => fn(...args), delay)
  }
}

/**
 * 深拷贝工具函数
 * FE-P2-5 修复（v12 前端复审）：统一深拷贝实现，替换 9 个文件中的 JSON.parse(JSON.stringify(...)) 重复代码
 * 优先使用 structuredClone（支持 Date/Map/Set/循环引用），回退到 JSON 方式
 * @param value 需要深拷贝的值
 * @returns 深拷贝后的值
 */
export function deepClone<T>(value: T): T {
  if (typeof structuredClone === 'function') {
    try {
      return structuredClone(value)
    } catch {
      // structuredClone 不支持函数/Symbol 等，回退到 JSON
    }
  }
  return JSON.parse(JSON.stringify(value))
}

export * from './storage'
export * from './export'
export * from './print'
export * from './lazy-loader'
