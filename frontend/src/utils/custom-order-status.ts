import { logger } from '@/utils/logger';

/**
 * 定制订单状态（custom_orders.status）单一映射源。
 *
 * 取值与后端权威模块 models/status/sales.rs::custom_order::ALL 逐字符一致
 * （全小写下划线，11 态）：状态机 10 个工艺态
 * draft→lab_dip→quotation→yarn_purchasing→dyeing→finishing→delivery→after_sales→completed
 * （+ cancelled 任意非终态可达，utils/process_state_machine.rs）
 * 加变更挂起态 change_pending（services/custom_order_crud_service.rs
 * submit_change_request 金额变化超阈值写入，approve_change 回 draft）。
 * DB 约束 chk_custom_order_status（migration m0064）与本词表由契约测
 * contract_wave5_custom_order_status_unity_test.rs 双向锁定。
 *
 * 此前前端手写表只有 8 token（缺 lab_dip/quotation/change_pending），且
 * list.vue 用 `|| status` 兜底把未知 token 直接显裸英文——补全后禁止兜底，
 * 词表外取值（脏数据/漂移）必须显式抛错暴露。
 */

export const CUSTOM_ORDER_STATUSES = [
  'draft',
  'lab_dip',
  'quotation',
  'yarn_purchasing',
  'dyeing',
  'finishing',
  'delivery',
  'after_sales',
  'change_pending',
  'completed',
  'cancelled',
] as const;

export type CustomOrderStatus = (typeof CUSTOM_ORDER_STATUSES)[number];

/** el-tag 配色类型（与 element-plus Tag 的 type 属性对齐） */
export type CustomOrderTagType = 'success' | 'warning' | 'info' | 'primary' | 'danger';

/** i18n 文案键（locales 的 customOrders.status.*，键名 camelCase 与既有条目同源） */
export const CUSTOM_ORDER_STATUS_LABEL_KEYS: Record<CustomOrderStatus, string> = {
  draft: 'customOrders.status.draft',
  lab_dip: 'customOrders.status.labDip',
  quotation: 'customOrders.status.quotation',
  yarn_purchasing: 'customOrders.status.yarnPurchasing',
  dyeing: 'customOrders.status.dyeing',
  finishing: 'customOrders.status.finishing',
  delivery: 'customOrders.status.delivery',
  after_sales: 'customOrders.status.afterSales',
  change_pending: 'customOrders.status.changePending',
  completed: 'customOrders.status.completed',
  cancelled: 'customOrders.status.cancelled',
};

/** Element Plus el-tag 类型 */
export const CUSTOM_ORDER_STATUS_TAG_TYPES: Record<CustomOrderStatus, CustomOrderTagType> = {
  draft: 'info',
  // 打样/报价是生产前置里程碑，与 yarn_purchasing 同级 primary
  lab_dip: 'primary',
  quotation: 'primary',
  yarn_purchasing: 'primary',
  dyeing: 'warning',
  finishing: 'warning',
  delivery: 'success',
  after_sales: 'danger',
  change_pending: 'warning',
  completed: 'success',
  cancelled: 'info',
};

/**
 * 把后端返回的状态归一到已知枚举。
 * 状态缺失（订单尚未加载）返回 undefined，由调用方渲染空标签；
 * 取值在词表外（拼错的大小写、后端已删除的状态、漂移 token）不被静默放过：
 * 记错误日志并抛错，禁止调用方用 `|| status` 把裸枚举回显给用户。
 */
export function normalizeCustomOrderStatus(
  status: string | undefined
): CustomOrderStatus | undefined {
  if (status === undefined) return undefined;
  if ((CUSTOM_ORDER_STATUSES as readonly string[]).includes(status)) {
    return status as CustomOrderStatus;
  }
  const message =
    `未识别的定制订单状态「${status}」，` +
    '合法取值见后端 models/status/sales.rs 的 custom_order 模块（11 态小写）';
  logger.error(message, { status });
  throw new Error(message);
}

export function customOrderStatusLabelKey(status: string | undefined): string | undefined {
  const normalized = normalizeCustomOrderStatus(status);
  return normalized ? CUSTOM_ORDER_STATUS_LABEL_KEYS[normalized] : undefined;
}

export function customOrderStatusTagType(status: string | undefined): CustomOrderTagType {
  const normalized = normalizeCustomOrderStatus(status);
  return normalized ? CUSTOM_ORDER_STATUS_TAG_TYPES[normalized] : 'info';
}
