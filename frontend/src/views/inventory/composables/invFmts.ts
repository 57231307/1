/**
 * invFmts.ts - 库存模块格式化工具
 * 任务编号: P14 批 2 I-3 第 8 批（拆分原 inventory/index.vue）
 * 提供数字千分位等纯函数
 */
import type { Warehouse } from '@/api/warehouse';
import { INVENTORY_STOCK_STATUS_LABEL_KEY } from '@/constants/inventory-stock-status';
import { logger } from '@/utils/logger';

/** 千分位格式化数字 */
export const formatNumber = (num: number) => num.toLocaleString();

/** 仓库名获取
 * v11 批次 160 P2-7 修复：参数类型从 any 改为 Warehouse（接口已保证 warehouse_name 必填） */
export const getWarehouseLabel = (wh: Pick<Warehouse, 'warehouse_name'>) => wh.warehouse_name;

/**
 * 台账状态 → 界面文案（列表、详情与打印共用，避免三处各写一份映射）
 *
 * 库里存的是后端主数据值（正常/报废/已删除）；取值域外的脏值告警后原样显示，
 * 不用「默认按正常」之类的兜底掩盖数据问题。translate 由调用方传入（组件内 useI18n 的 t）。
 */
export const getStockStatusLabel = (status: string, translate: (key: string) => string): string => {
  const key = INVENTORY_STOCK_STATUS_LABEL_KEY[status];
  if (!key) {
    logger.warn(`未知库存台账状态，后端主数据值清单需同步：${status}`);
    return status;
  }
  return translate(key);
};
