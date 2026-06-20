/**
 * invFmts.ts - 库存模块格式化工具
 * 任务编号: P14 批 2 I-3 第 8 批（拆分原 inventory/index.vue）
 * 提供数字千分位、CSV 转义等纯函数
 * 行为完全保持一致（仅结构重构）
 */

/** 千分位格式化数字 */
export const formatNumber = (num: number) => num.toLocaleString()

/** CSV 单元格转义 */
export const escapeCsvCell = (cell: any) => `"${cell}"`

/** 仓库名兜底：warehouse_name 优先，name 次之 */
export const getWarehouseLabel = (wh: any) => wh.warehouse_name || wh.name || ''
