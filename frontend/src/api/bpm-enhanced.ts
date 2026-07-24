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
  // 批次 98 P2-D 修复（v5 复审）：原 any 改为 Record<string, unknown>，动态 JSON Schema 字段
  form_schema?: Record<string, unknown>
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
  // 批次 98 P2-D 修复（v5 复审）：原 any 改为 Record<string, unknown>，与 types/bpm.ts 保持一致
  variables?: Record<string, unknown>
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

// D14 Batch 5b：原 bpmEnhancedApi.listDefinitions 转为风格 B 函数
export const getBpmDefinitionList = (params?: {
  page?: number
  page_size?: number
  category?: string
  keyword?: string
}) => request.get<ApiResponse<PageResult<ProcessDefinition>>>('/bpm/definitions', { params })

// D14 Batch 5b：原 bpmEnhancedApi.getDefinition 转为风格 B 函数
export const getBpmDefinitionById = (id: number) =>
  request.get<ApiResponse<ProcessDefinition>>(`/bpm/definitions/${id}`)

// D14 Batch 5b：原 bpmEnhancedApi.createDefinition 转为风格 B 函数
export const createBpmDefinition = (data: Partial<ProcessDefinition>) =>
  request.post<ApiResponse<ProcessDefinition>>('/bpm/definitions', data)

// D14 Batch 5b：原 bpmEnhancedApi.updateDefinition 转为风格 B 函数
export const updateBpmDefinition = (id: number, data: Partial<ProcessDefinition>) =>
  request.put<ApiResponse<ProcessDefinition>>(`/bpm/definitions/${id}`, data)

// D14 Batch 5b：原 bpmEnhancedApi.deleteDefinition 转为风格 B 函数
export const deleteBpmDefinition = (id: number) =>
  request.delete<ApiResponse<null>>(`/bpm/definitions/${id}`)

// D14 Batch 5b：原 bpmEnhancedApi.createVersion 转为风格 B 函数
export const createBpmVersion = (definitionId: number, data?: { change_log?: string }) =>
  request.post<ApiResponse<ProcessVersion>>(`/bpm/definitions/${definitionId}/versions`, data)

// D14 Batch 5b：原 bpmEnhancedApi.listVersions 转为风格 B 函数
export const getBpmVersionList = (definitionId: number) =>
  request.get<ApiResponse<ProcessVersion[]>>(`/bpm/definitions/${definitionId}/versions`)

// D14 Batch 5b：原 bpmEnhancedApi.activateVersion 转为风格 B 函数
export const activateBpmVersion = (versionId: number) =>
  request.post<ApiResponse<null>>(`/bpm/versions/${versionId}/activate`)

// D14 Batch 5b：原 bpmEnhancedApi.saveAsTemplate 转为风格 B 函数
export const saveBpmAsTemplate = (
  definitionId: number,
  data: { template_name: string; category: string; description?: string }
) =>
  request.post<ApiResponse<ProcessTemplate>>(`/bpm/definitions/${definitionId}/template`, data)

// D14 Batch 5b：原 bpmEnhancedApi.listTemplates 转为风格 B 函数
export const getBpmTemplateList = (params?: {
  page?: number
  page_size?: number
  category?: string
}) => request.get<ApiResponse<PageResult<ProcessTemplate>>>('/bpm/templates', { params })

// D14 Batch 5b：原 bpmEnhancedApi.getTemplate 转为风格 B 函数
export const getBpmTemplateById = (id: number) =>
  request.get<ApiResponse<ProcessTemplate>>(`/bpm/templates/${id}`)

// D14 Batch 5b：原 bpmEnhancedApi.createFromTemplate 转为风格 B 函数
export const createBpmFromTemplate = (templateId: number, data?: { process_name?: string }) =>
  request.post<ApiResponse<ProcessDefinition>>(`/bpm/templates/${templateId}/create`, data)

// D14 Batch 5b：原 bpmEnhancedApi.deleteTemplate 转为风格 B 函数
export const deleteBpmTemplate = (id: number) =>
  request.delete<ApiResponse<null>>(`/bpm/templates/${id}`)

// D14 Batch 5b：原 bpmEnhancedApi.getPendingTasks 转为风格 B 函数
export const getBpmPendingTaskList = (params?: { page?: number; page_size?: number }) =>
  request.get<ApiResponse<PageResult<ApprovalTask>>>('/bpm/tasks/pending', { params })

// D14 Batch 5b：原 bpmEnhancedApi.getCompletedTasks 转为风格 B 函数
export const getBpmCompletedTaskList = (params?: { page?: number; page_size?: number }) =>
  request.get<ApiResponse<PageResult<ApprovalTask>>>('/bpm/tasks/completed', { params })

// D14 Batch 5b：原 bpmEnhancedApi.executeApproval 转为风格 B 函数
export const executeBpmApproval = (data: ApprovalAction) =>
  request.post<ApiResponse<null>>('/bpm/approval/execute', data)

// D14 Batch 5b：原 bpmEnhancedApi.getApprovalChain 转为风格 B 函数
export const getBpmEnhancedApprovalChain = (instanceId: string) =>
  request.get<ApiResponse<ApprovalChainNode[]>>(`/bpm/instances/${instanceId}/chain`)
