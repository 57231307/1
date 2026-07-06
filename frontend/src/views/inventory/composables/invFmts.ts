/**
 * invFmts.ts - 库存模块格式化工具
 * 任务编号: P14 批 2 I-3 第 8 批（拆分原 inventory/index.vue）
 * 提供数字千分位等纯函数
 */

/** 千分位格式化数字 */
export const formatNumber = (num: number) => num.toLocaleString()

/** 仓库名兜底：warehouse_name 优先，name 次之 */
export const getWarehouseLabel = (wh: any) => wh.warehouse_name || wh.name || ''
