import { request } from './request'
import type { ApiResponse } from './request'

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
  startProcess: (data: { process_key: string; business_key?: string; variables?: any }) =>
    request.post<ApiResponse<any>>('/bpm/process/start', data),

  approveTask: (data: { task_id: string; comment?: string; variables?: any }) =>
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
    request.get<ApiResponse<any>>('/bpm/business-relation', {
      params: { business_type: businessType, business_id: businessId },
    }),

  getProcessVisualization: (instanceId: string) =>
    request.get<ApiResponse<any>>(`/bpm/visualization/${instanceId}`),

  getApprovalChain: (instanceId: string) =>
    request.get<ApiResponse<ApprovalChainItem[]>>(`/bpm/instances/${instanceId}/approval-chain`),

  getInstanceDetail: (instanceId: string) =>
    request.get<ApiResponse<any>>(`/bpm/instances/${instanceId}/detail`),

  getMonitorStats: () => request.get<ApiResponse<any>>('/bpm/monitor/stats'),

  getPendingTasksForMonitor: (params?: any) =>
    request.get<ApiResponse<{ list: BPMTask[]; total: number }>>('/bpm/monitor/pending-tasks', {
      params,
    }),

  listInstancesForMonitor: (params?: any) =>
    request.get<ApiResponse<{ list: BPMInstance[]; total: number }>>('/bpm/monitor/instances', {
      params,
    }),
}
