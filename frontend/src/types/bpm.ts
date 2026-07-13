/**
 * BPM 工作流业务类型定义
 */

/**
 * BPM 流程变量值类型
 * FE-P2-1 修复（批次 388 v13 复审）：原 Record<string, unknown> 过于宽泛，
 * 细化为具体的基础类型联合，避免 unknown 导致调用方需要类型断言
 */
export type BpmVariableValue = string | number | boolean | null

/**
 * BPM 流程变量集合
 */
export interface BpmVariables {
  [key: string]: BpmVariableValue
}

/**
 * BPM 流程实例状态
 * FE-P2-1 修复（批次 388 v13 复审）：原 status: string 过于宽泛，收窄为字面量联合类型
 */
export type BpmProcessStatus = 'running' | 'completed' | 'terminated' | 'cancelled' | 'suspended'

/**
 * 启动流程请求参数
 */
export interface StartProcessRequest {
  process_key: string
  business_key?: string
  variables?: BpmVariables
}

/**
 * 启动流程响应数据
 */
export interface StartProcessResponse {
  instance_id: string
  process_name: string
  start_time: string
}

/**
 * 审批任务请求参数
 */
export interface ApproveTaskRequest {
  task_id: string
  comment?: string
  variables?: BpmVariables
}

/**
 * 业务关系响应数据
 */
export interface BusinessRelationResponse {
  business_type: string
  business_id: number
  instance_id: string
  process_name: string
  status: BpmProcessStatus
}

/**
 * 流程可视化响应数据
 */
export interface ProcessVisualizationResponse {
  instance_id: string
  process_name: string
  current_activity: string
  activity_history: string[]
  diagram_url?: string
}

/**
 * 流程实例详情响应数据
 */
export interface InstanceDetailResponse {
  instance_id: string
  process_name: string
  start_user: string
  start_time: string
  end_time?: string
  status: BpmProcessStatus
  current_activities: string[]
  variables: BpmVariables
}

/**
 * 监控统计数据
 */
export interface MonitorStatsResponse {
  total_instances: number
  running_instances: number
  completed_instances: number
  pending_tasks: number
  overdue_tasks: number
}

/**
 * 监控待办任务查询参数
 */
export interface MonitorPendingTasksParams {
  assignee?: number
  status?: string
  page?: number
  page_size?: number
}

/**
 * 监控实例列表查询参数
 */
export interface MonitorInstancesParams {
  status?: string
  start_user?: number
  page?: number
  page_size?: number
}
