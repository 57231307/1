/**
 * diFmts.ts - 数据导入格式化工具
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 data-import/index.vue）
 * 包含模块映射、任务状态映射、状态类型映射等格式化函数
 * 行为完全保持一致（仅结构重构）
 */

/**
 * 模块枚举到中文标签
 */
export const MODULE_MAP: Record<string, string> = {
  customer: '客户',
  supplier: '供应商',
  product: '产品',
  inventory: '库存',
  sales: '销售',
  purchase: '采购',
  finance: '财务',
}

/**
 * 导入任务状态到中文标签
 */
export const TASK_STATUS_MAP: Record<string, string> = {
  pending: '待处理',
  processing: '处理中',
  completed: '已完成',
  failed: '失败',
}

/**
 * 导入任务状态到 el-tag 类型
 */
export const TASK_STATUS_TYPE_MAP: Record<string, string> = {
  pending: 'info',
  processing: 'warning',
  completed: 'success',
  failed: 'danger',
}
