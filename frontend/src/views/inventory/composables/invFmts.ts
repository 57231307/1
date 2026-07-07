/**
 * invFmts.ts - 库存模块格式化工具
 * 任务编号: P14 批 2 I-3 第 8 批（拆分原 inventory/index.vue）
 * 提供数字千分位等纯函数
 */
import type { Warehouse } from '@/api/warehouse'

/** 千分位格式化数字 */
export const formatNumber = (num: number) => num.toLocaleString()

/** 仓库名获取
 * v11 批次 160 P2-7 修复：参数类型从 any 改为 Warehouse（接口已保证 warehouse_name 必填） */
export const getWarehouseLabel = (wh: Pick<Warehouse, 'warehouse_name'>) => wh.warehouse_name
