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

export const bpmApi = {
  startProcess: (data: StartProcessRequest) =>
    request.post<ApiResponse<StartProcessResponse>>('/bpm/process/start', data),

  approveTask: (data: ApproveTaskRequest) =>
    request.post<ApiResponse<null>>('/bpm/tasks/approve', data),

  queryTasks: (params?: {
    assignee?: number
    status?: string
    page?: number
    page_size?: number
  }) => request.get<ApiResponse<{ list: BPMTask[]; total: number }>>('/bpm/tasks', { params }),

  transferTask: (taskId: string, targetUserId: number, reason?: string) =>
    request.post<ApiResponse<null>>(`/bpm/tasks/${taskId}/transfer`, {
      target_user_id: targetUserId,
      reason,
    }),

  urgeTask: (taskId: string) => request.post<ApiResponse<null>>(`/bpm/tasks/${taskId}/urge`),

  getBusinessRelation: (businessType: string, businessId: number) =>
    request.get<ApiResponse<BusinessRelationResponse>>('/bpm/business-relation', {
      params: { business_type: businessType, business_id: businessId },
    }),

  getProcessVisualization: (instanceId: string) =>
    request.get<ApiResponse<ProcessVisualizationResponse>>(`/bpm/visualization/${instanceId}`),

  getApprovalChain: (instanceId: string) =>
    request.get<ApiResponse<ApprovalChainItem[]>>(`/bpm/instances/${instanceId}/approval-chain`),

  getInstanceDetail: (instanceId: string) =>
    request.get<ApiResponse<InstanceDetailResponse>>(`/bpm/instances/${instanceId}/detail`),

  getMonitorStats: () => request.get<ApiResponse<MonitorStatsResponse>>('/bpm/monitor/stats'),

  getPendingTasksForMonitor: (params?: MonitorPendingTasksParams) =>
    request.get<ApiResponse<{ list: BPMTask[]; total: number }>>('/bpm/monitor/pending-tasks', {
      params,
    }),

  listInstancesForMonitor: (params?: MonitorInstancesParams) =>
    request.get<ApiResponse<{ list: BPMInstance[]; total: number }>>('/bpm/monitor/instances', {
      params,
    }),

  // 批次 157d-3 新增：撤回流程实例（接收实例主键 id: number）
  cancelInstance: (instanceId: number, cancelReason?: string) =>
    request.post<ApiResponse<null>>(`/bpm/instances/${instanceId}/cancel`, {
      cancel_reason: cancelReason,
    }),
}
