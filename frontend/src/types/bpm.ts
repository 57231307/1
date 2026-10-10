/**
 * BPM 工作流业务类型定义
 */

/**
 * BPM 流程变量值类型
 * FE-P2-1 修复（批次 388 v13 复审）：原 Record<string, unknown> 过于宽泛，
 * 细化为具体的基础类型联合，避免 unknown 导致调用方需要类型断言
 */
export type BpmVariableValue = string | number | boolean | null;

/**
 * BPM 流程变量集合
 */
export interface BpmVariables {
  [key: string]: BpmVariableValue;
}

/**
 * 启动流程请求参数
 */
export interface StartProcessRequest {
  process_key: string;
  business_key?: string;
  variables?: BpmVariables;
}

/**
 * 启动流程响应数据
 */
export interface StartProcessResponse {
  instance_id: string;
  process_name: string;
  start_time: string;
}

/**
 * 审批任务请求参数
 */
export interface ApproveTaskRequest {
  task_id: string;
  comment?: string;
  variables?: BpmVariables;
}

/**
 * 监控统计数据
 */
export interface MonitorStatsResponse {
  total_instances: number;
  processing_instances: number;
  completed_instances: number;
  terminated_instances: number;
  total_tasks: number;
  pending_tasks: number;
  completed_tasks: number;
  rejected_tasks: number;
  /** status=pending 且 due_date 已过的任务数 */
  overdue_tasks: number;
  /** 已完成实例平均处理时长（分钟）；无样本时后端返回 null */
  avg_process_duration_minutes: number | null;
}

/**
 * 监控待办任务查询参数（唯一真相：handlers/bpm_handler.rs MonitorQuery{page,page_size,status}）。
 * 后端 pending-tasks 服务固定按小写 pending 过滤（bpm_ops/monitor.rs），status 传入不参与、
 * assignee 键后端不存在——两者被 Axum 静默丢弃，故不再声明（人员维度过滤为后端缺口，已登记串行清单）。
 */
export interface MonitorPendingTasksParams {
  page?: number;
  page_size?: number;
}

/**
 * 监控实例列表查询参数（唯一真相：MonitorQuery{page,page_size,status}）。
 * start_user 键后端不存在（发起人过滤为后端缺口，已登记串行清单），不声明。
 * status 取值为流程实例状态词表（大写 PROCESSING/COMPLETED/TERMINATED/CANCELLED）。
 */
export interface MonitorInstancesParams {
  status?: string;
  page?: number;
  page_size?: number;
}
