/**
 * prdFmts.ts - 生产管理格式化工具
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 production/index.vue）
 * 提供状态文本格式化等纯函数
 * 行为完全保持一致（仅结构重构）
 */
import { PRODUCTION_ORDER_STATUS } from '@/api/production'

/**
 * 获取生产订单状态中文标签
 */
export const getStatusLabel = (status: string): string => {
  return (
    PRODUCTION_ORDER_STATUS[status as keyof typeof PRODUCTION_ORDER_STATUS]?.label || status
  )
}
