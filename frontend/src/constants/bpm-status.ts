/**
 * bpm-status.ts - BPM 任务/流程实例状态词表的显示层单源
 *
 * 状态取值域与后端写入方逐字符同源：
 * - 任务状态：backend/src/models/status/bpm_crm_contract.rs `bpm_task`
 *   （小写 pending/completed/rejected/cancelled，写入点 bpm_ops/instance.rs 建单、
 *   bpm_ops/task.rs 推进）；
 * - 流程实例状态：同文件 `bpm_instance`（大写 PROCESSING/COMPLETED/TERMINATED/CANCELLED）。
 *
 * 列表（审批中心待办/已办）、详情（审批链对话框）、工作台（bpm/index.vue 待办/已办/
 * 监控/追溯文本）一律经本模块取标签与 el-tag 类型，禁止各视图再抄一份映射
 * （历史上 index.vue 与 approval 组件各持一套键路径，一处显码一处显名即漂移根因）。
 * 文案键统一取 `bpm.taskStatus.*` / `bpm.processStatus.*`，zh-CN 与 en-US 双侧同源。
 */
import { i18n } from '@/i18n';

/** 任务状态词表（与后端 bpm_task 写入方同源，小写） */
export const BPM_TASK_STATUSES = ['pending', 'completed', 'rejected', 'cancelled'] as const;
export type BpmTaskStatusLabelStatus = (typeof BPM_TASK_STATUSES)[number];

/** 流程实例状态词表（与后端 bpm_instance 写入方同源，大写） */
export const BPM_PROCESS_STATUSES = ['PROCESSING', 'COMPLETED', 'TERMINATED', 'CANCELLED'] as const;
export type BpmProcessStatusValue = (typeof BPM_PROCESS_STATUSES)[number];

/** el-tag type 联合字面量（与全站标签口径一致） */
export type BpmTagType = 'success' | 'warning' | 'info' | 'primary' | 'danger';

const TASK_STATUS_TAG_TYPE: Record<BpmTaskStatusLabelStatus, BpmTagType> = {
  pending: 'warning',
  completed: 'success',
  rejected: 'danger',
  cancelled: 'info',
};

const TASK_STATUS_LABEL_KEY: Record<BpmTaskStatusLabelStatus, string> = {
  pending: 'bpm.taskStatus.pending',
  completed: 'bpm.taskStatus.completed',
  rejected: 'bpm.taskStatus.rejected',
  cancelled: 'bpm.taskStatus.cancelled',
};

const PROCESS_STATUS_TAG_TYPE: Record<BpmProcessStatusValue, BpmTagType> = {
  PROCESSING: 'primary',
  COMPLETED: 'success',
  TERMINATED: 'danger',
  CANCELLED: 'info',
};

const PROCESS_STATUS_LABEL_KEY: Record<BpmProcessStatusValue, string> = {
  PROCESSING: 'bpm.processStatus.processing',
  COMPLETED: 'bpm.processStatus.completed',
  TERMINATED: 'bpm.processStatus.terminated',
  CANCELLED: 'bpm.processStatus.cancelled',
};

/** 任务状态 → el-tag type；词表外取值（脏数据）落 info，不猜语义 */
export const getTaskStatusType = (status?: string | null): BpmTagType => {
  if (!status) return 'info';
  return TASK_STATUS_TAG_TYPE[status as BpmTaskStatusLabelStatus] ?? 'info';
};

/**
 * 任务状态 → 用户可见文案（i18n 双侧同源）。
 * 词表外取值回退原文（如实显示脏数据，不兜底成某个合法状态的文案）。
 */
export const getTaskStatusLabel = (status?: string | null): string => {
  if (!status) return '';
  const key = TASK_STATUS_LABEL_KEY[status as BpmTaskStatusLabelStatus];
  return key ? i18n.global.t(key) : status;
};

/** 流程实例状态 → el-tag type */
export const getProcessStatusType = (status?: string | null): BpmTagType => {
  if (!status) return 'info';
  return PROCESS_STATUS_TAG_TYPE[status as BpmProcessStatusValue] ?? 'info';
};

/** 流程实例状态 → 用户可见文案（回退口径同上） */
export const getProcessStatusLabel = (status?: string | null): string => {
  if (!status) return '';
  const key = PROCESS_STATUS_LABEL_KEY[status as BpmProcessStatusValue];
  return key ? i18n.global.t(key) : status;
};
