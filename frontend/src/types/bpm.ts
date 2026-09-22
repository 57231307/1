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
  running_instances: number;
  completed_instances: number;
  pending_tasks: number;
  overdue_tasks: number;
}

/**
 * 监控待办任务查询参数
 */
export interface MonitorPendingTasksParams {
  assignee?: number;
  status?: string;
  page?: number;
  page_size?: number;
}

/**
 * 监控实例列表查询参数
 */
export interface MonitorInstancesParams {
  status?: string;
  start_user?: number;
  page?: number;
  page_size?: number;
}
