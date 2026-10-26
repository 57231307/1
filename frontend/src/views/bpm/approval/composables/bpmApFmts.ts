/**
 * bpmApFmts.ts - BPM 审批格式化工具
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 bpm/approval.vue）
 * 提供优先级标签类型 / 逾期判断 / 时间格式化等纯函数
 *
 * 状态词表（任务/流程实例）的标签与 el-tag type 一律走
 * `@/constants/bpm-status` 单源（列表与详情共用同一枚举），
 * 本文件不再自持任何状态文案映射，也不再保留无调用方的硬编码中文映射
 * （原 PRIORITY_TEXT_MAP/NODE_TYPE_NAME_MAP 为死导出，视图实际使用 i18n 键）。
 */

/**
 * 优先级类型（el-tag type）
 */
const PRIORITY_TYPE_MAP: Record<string, string> = {
  high: 'danger',
  medium: 'warning',
  low: 'info',
};

/**
 * 获取优先级对应的 el-tag 类型
 */
export const getPriorityType = (priority: string): string => {
  return PRIORITY_TYPE_MAP[priority] || 'info';
};

/**
 * 判断是否逾期（截止时间 < 当前时间）
 */
export const isOverdue = (dueDate: string): boolean => {
  return new Date(dueDate) < new Date();
};

/**
 * ISO 时间串 → 本地可读时间；空值显示 '-'
 */
export const formatDateTime = (value?: string | null): string => {
  if (!value) return '-';
  const d = new Date(value);
  return Number.isNaN(d.getTime()) ? value : d.toLocaleString('zh-CN');
};

/**
 * 节点状态对应的 css 类名
 * 状态取值域 = bpm_task 词表（pending/completed/rejected/cancelled），
 * 审批链节点状态由 services/bpm_ops/instance.rs get_approval_chain 直接透传任务状态。
 */
const NODE_STATUS_CLASS_MAP: Record<string, string> = {
  pending: 'status-pending',
  completed: 'status-approved',
  rejected: 'status-rejected',
  cancelled: 'status-skipped',
};

/**
 * 获取审批链节点状态 css 类
 */
export const getNodeStatusClass = (status: string): string => {
  return NODE_STATUS_CLASS_MAP[status] || '';
};
