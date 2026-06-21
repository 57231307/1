import { request } from './request'
import type { ApiResponse, PageResult } from '@/types/api'

export interface ProcessDefinition {
  id: number
  process_key: string
  process_name: string
  description?: string
  version: number
  status: 'draft' | 'active' | 'suspended' | 'deprecated'
  category?: string
  form_schema?: any
  nodes?: ProcessNode[]
  created_at: string
  updated_at: string
  created_by?: string
}

export interface ProcessNode {
  id: string
  type: 'start' | 'end' | 'approval' | 'condition' | 'notify'
  name: string
  assignee_type?: 'user' | 'role' | 'department' | 'dynamic'
  assignee_value?: string | number
  condition?: string
  next_nodes?: string[]
}

export interface ProcessVersion {
  id: number
  process_definition_id: number
  version: number
  status: 'draft' | 'active' | 'deprecated'
  change_log?: string
  created_at: string
  created_by?: string
}

export interface ProcessTemplate {
  id: number
  template_key: string
  template_name: string
  description?: string
  category: string
  icon?: string
  process_definition?: ProcessDefinition
  usage_count: number
  created_at: string
}

export interface ApprovalAction {
  task_id: string
  action: 'approve' | 'reject' | 'transfer' | 'delegate'
  comment?: string
  target_user_id?: number
  variables?: any
}

export interface ApprovalTask {
  id: number
  task_id: string
  process_instance_id: string
  process_name: string
  task_name: string
  assignee_name?: string
  start_user_name?: string
  created_at: string
  due_date?: string
  priority: 'low' | 'medium' | 'high'
  status: 'pending' | 'completed' | 'cancelled'
  business_key?: string
  result?: 'approved' | 'rejected'
  comment?: string
  approved_at?: string
}

export interface ApprovalChainNode {
  order: number
  node_name: string
  node_type: string
  approver_name?: string
  status: 'pending' | 'approved' | 'rejected' | 'skipped'
  comment?: string
  approved_at?: string
  duration?: number
}

export const bpmEnhancedApi = {
  listDefinitions: (params?: {
    page?: number
    page_size?: number
    category?: string
    keyword?: string
  }) => request.get<ApiResponse<PageResult<ProcessDefinition>>>('/bpm/definitions', { params }),

  getDefinition: (id: number) =>
    request.get<ApiResponse<ProcessDefinition>>(`/bpm/definitions/${id}`),

  createDefinition: (data: Partial<ProcessDefinition>) =>
    request.post<ApiResponse<ProcessDefinition>>('/bpm/definitions', data),

  updateDefinition: (id: number, data: Partial<ProcessDefinition>) =>
    request.put<ApiResponse<ProcessDefinition>>(`/bpm/definitions/${id}`, data),

  deleteDefinition: (id: number) => request.delete<ApiResponse<null>>(`/bpm/definitions/${id}`),

  createVersion: (definitionId: number, data?: { change_log?: string }) =>
    request.post<ApiResponse<ProcessVersion>>(`/bpm/definitions/${definitionId}/versions`, data),

  listVersions: (definitionId: number) =>
    request.get<ApiResponse<ProcessVersion[]>>(`/bpm/definitions/${definitionId}/versions`),

  activateVersion: (versionId: number) =>
    request.post<ApiResponse<null>>(`/bpm/versions/${versionId}/activate`),

  saveAsTemplate: (
    definitionId: number,
    data: { template_name: string; category: string; description?: string }
  ) =>
    request.post<ApiResponse<ProcessTemplate>>(`/bpm/definitions/${definitionId}/template`, data),

  listTemplates: (params?: { page?: number; page_size?: number; category?: string }) =>
    request.get<ApiResponse<PageResult<ProcessTemplate>>>('/bpm/templates', { params }),

  getTemplate: (id: number) => request.get<ApiResponse<ProcessTemplate>>(`/bpm/templates/${id}`),

  createFromTemplate: (templateId: number, data?: { process_name?: string }) =>
    request.post<ApiResponse<ProcessDefinition>>(`/bpm/templates/${templateId}/create`, data),

  deleteTemplate: (id: number) => request.delete<ApiResponse<null>>(`/bpm/templates/${id}`),

  getPendingTasks: (params?: { page?: number; page_size?: number }) =>
    request.get<ApiResponse<PageResult<ApprovalTask>>>('/bpm/tasks/pending', { params }),

  getCompletedTasks: (params?: { page?: number; page_size?: number }) =>
    request.get<ApiResponse<PageResult<ApprovalTask>>>('/bpm/tasks/completed', { params }),

  executeApproval: (data: ApprovalAction) =>
    request.post<ApiResponse<null>>('/bpm/approval/execute', data),

  getApprovalChain: (instanceId: string) =>
    request.get<ApiResponse<ApprovalChainNode[]>>(`/bpm/instances/${instanceId}/chain`),
}
