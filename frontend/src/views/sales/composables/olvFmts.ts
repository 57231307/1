/**
 * olvFmts.ts - 销售订单列表格式化工具
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 sales/views/OrderListView.vue）
 * 提供状态类型/标签/金额格式化等纯函数
 *
 * 状态映射不在本文件维护：词表与配色统一出自 utils/sales-status.ts
 * （它与后端 models/status/sales.rs 的 sales_order 常量一一对应），
 * 此前这里另存一份只覆盖 5 个状态的中文硬编码表，导致 draft/partial_shipped/
 * rejected 三个真实状态在列表里直接露出英文枚举，且英文界面下文案仍是中文。
 */
import { i18n } from '@/i18n';
import { salesStatusLabelKey, salesStatusTagType, type SalesTagType } from '@/utils/sales-status';

/** 获取销售订单状态 el-tag 类型 */
export const getStatusType = (status: string): SalesTagType => salesStatusTagType(status);

/** 获取销售订单状态文案（词表外的值原样显示，由单一映射源负责告警） */
export const getStatusText = (status: string): string => {
  const key = salesStatusLabelKey(status);
  return key ? i18n.global.t(key) : status;
};

/** 格式化金额（人民币 + 千分位） */
export const formatAmount = (value: number) => `¥${(value || 0).toLocaleString()}`;
