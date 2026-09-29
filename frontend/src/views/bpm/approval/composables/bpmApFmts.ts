/**
 * bpmApFmts.ts - BPM 审批格式化工具
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 bpm/approval.vue）
 * 提供优先级映射 / 节点状态 / 节点类型 / 逾期判断等纯函数
 * 行为完全保持一致（仅结构重构）
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
 * 优先级文本
 */
const PRIORITY_TEXT_MAP: Record<string, string> = {
  high: '高',
  medium: '中',
  low: '低',
};

/**
 * 获取优先级对应的 el-tag 类型
 */
export const getPriorityType = (priority: string): string => {
  return PRIORITY_TYPE_MAP[priority] || 'info';
};

/**
 * 获取优先级显示文本
 */
export const getPriorityText = (priority: string): string => {
  return PRIORITY_TEXT_MAP[priority] || priority;
};

/**
 * 判断是否逾期（截止时间 < 当前时间）
 */
export const isOverdue = (dueDate: string): boolean => {
  return new Date(dueDate) < new Date();
};

/**
 * 任务状态词表（与后端 models/status/bpm_crm_contract.rs 的 bpm_task 逐项一致：
 * 小写 pending / completed / rejected / cancelled，写入点见 services/bpm_ops/task.rs）
 */
export const TASK_STATUSES = ['pending', 'completed', 'rejected', 'cancelled'] as const;

/**
 * 任务状态 → el-tag type
 */
const TASK_STATUS_TYPE_MAP: Record<string, string> = {
  pending: 'warning',
  completed: 'success',
  rejected: 'danger',
  cancelled: 'info',
};

/** 任务状态对应的 el-tag 类型 */
export const getTaskStatusType = (status: string): string => {
  return TASK_STATUS_TYPE_MAP[status] || 'info';
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

/**
 * 节点类型中文名
 * 取值域来自流程定义 config.nodes[].type（services/bpm_service.rs 与
 * services/bpm_ops/task.rs 判定用的 start_event / end_event / user_task / condition）
 */
const NODE_TYPE_NAME_MAP: Record<string, string> = {
  start_event: '开始',
  end_event: '结束',
  user_task: '审批',
  condition: '条件',
  notify: '通知',
};

/**
 * 获取审批链节点类型中文名
 */
export const getNodeTypeName = (type: string): string => {
  return NODE_TYPE_NAME_MAP[type] || type;
};
