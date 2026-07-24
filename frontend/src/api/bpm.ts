import { request } from './request'
import type { ApiResponse } from '@/types/api-response'
import type {
  StartProcessRequest,
  StartProcessResponse,
  ApproveTaskRequest,
  BusinessRelationResponse,
  ProcessVisualizationResponse,
  InstanceDetailResponse,
  MonitorStatsResponse,
  MonitorPendingTasksParams,
  MonitorInstancesParams,
} from '@/types/bpm'

export interface BPMProcess {
  id: number
  process_key: string
  process_name: string
  description?: string
  version: number
  status: string
  created_at: string
}

export interface BPMTask {
  id: number
  task_id: string
  process_instance_id: string
  process_name: string
  task_name: string
  assignee?: string
  assignee_name?: string
  candidate_users?: string[]
  due_date?: string
  priority: string
  status: string
  created_at: string
  business_key?: string
  business_type?: string
}

export interface BPMInstance {
  id: number
  instance_id: string
  process_name: string
  start_user: string
  start_user_name: string
  start_time: string
  end_time?: string
  status: string
  current_activities?: string[]
}

export interface ApprovalChainItem {
  order: number
  approver_id: number
  approver_name: string
  status: string
  comment?: string
  approved_at?: string
}

// D14 Batch 5b：原 bpmApi.startProcess 转为风格 B 函数
export const startBpmProcess = (data: StartProcessRequest) =>
  request.post<ApiResponse<StartProcessResponse>>('/bpm/process/start', data)

// D14 Batch 5b：原 bpmApi.approveTask 转为风格 B 函数
export const approveBpmTask = (data: ApproveTaskRequest) =>
  request.post<ApiResponse<null>>('/bpm/tasks/approve', data)

// D14 Batch 5b：原 bpmApi.queryTasks 转为风格 B 函数
export const getBpmTaskList = (params?: {
  assignee?: number
  status?: string
  page?: number
  page_size?: number
}) => request.get<ApiResponse<{ list: BPMTask[]; total: number }>>('/bpm/tasks', { params })

// D14 Batch 5b：原 bpmApi.transferTask 转为风格 B 函数
export const transferBpmTask = (taskId: string, targetUserId: number, reason?: string) =>
  request.post<ApiResponse<null>>(`/bpm/tasks/${taskId}/transfer`, {
    target_user_id: targetUserId,
    reason,
  })

// D14 Batch 5b：原 bpmApi.urgeTask 转为风格 B 函数
export const urgeBpmTask = (taskId: string) =>
  request.post<ApiResponse<null>>(`/bpm/tasks/${taskId}/urge`)

// D14 Batch 5b：原 bpmApi.getBusinessRelation 转为风格 B 函数
export const getBpmBusinessRelation = (businessType: string, businessId: number) =>
  request.get<ApiResponse<BusinessRelationResponse>>('/bpm/business-relation', {
    params: { business_type: businessType, business_id: businessId },
  })

// D14 Batch 5b：原 bpmApi.getProcessVisualization 转为风格 B 函数
export const getBpmProcessVisualization = (instanceId: string) =>
  request.get<ApiResponse<ProcessVisualizationResponse>>(`/bpm/visualization/${instanceId}`)

// D14 Batch 5b：原 bpmApi.getApprovalChain 转为风格 B 函数
export const getBpmApprovalChain = (instanceId: string) =>
  request.get<ApiResponse<ApprovalChainItem[]>>(`/bpm/instances/${instanceId}/approval-chain`)

// D14 Batch 5b：原 bpmApi.getInstanceDetail 转为风格 B 函数
export const getBpmInstanceById = (instanceId: string) =>
  request.get<ApiResponse<InstanceDetailResponse>>(`/bpm/instances/${instanceId}/detail`)

// D14 Batch 5b：原 bpmApi.getMonitorStats 转为风格 B 函数
export const getBpmMonitorStats = () =>
  request.get<ApiResponse<MonitorStatsResponse>>('/bpm/monitor/stats')

// D14 Batch 5b：原 bpmApi.getPendingTasksForMonitor 转为风格 B 函数
export const getBpmPendingTaskList = (params?: MonitorPendingTasksParams) =>
  request.get<ApiResponse<{ list: BPMTask[]; total: number }>>('/bpm/monitor/pending-tasks', {
    params,
  })

// D14 Batch 5b：原 bpmApi.listInstancesForMonitor 转为风格 B 函数
export const getBpmInstanceListForMonitor = (params?: MonitorInstancesParams) =>
  request.get<ApiResponse<{ list: BPMInstance[]; total: number }>>('/bpm/monitor/instances', {
    params,
  })

// 批次 157d-3 新增：撤回流程实例（接收实例主键 id: number）
// D14 Batch 5b：原 bpmApi.cancelInstance 转为风格 B 函数
export const cancelBpmInstance = (instanceId: number, cancelReason?: string) =>
  request.post<ApiResponse<null>>(`/bpm/instances/${instanceId}/cancel`, {
    cancel_reason: cancelReason,
  })
