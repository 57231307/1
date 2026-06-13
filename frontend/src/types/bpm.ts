/**
 * BPM 工作流业务类型定义
 */

/**
 * 启动流程请求参数
 */
export interface StartProcessRequest {
  process_key: string
  business_key?: string
  variables?: Record<string, unknown>
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
  variables?: Record<string, unknown>
}

/**
 * 业务关系响应数据
 */
export interface BusinessRelationResponse {
  business_type: string
  business_id: number
  instance_id: string
  process_name: string
  status: string
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
  status: string
  current_activities: string[]
  variables: Record<string, unknown>
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
