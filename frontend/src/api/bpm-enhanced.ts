import { request } from './request';
import type { ApiResponse, PageResult } from '@/types/api';

export interface ProcessDefinition {
  id: number;
  process_key: string;
  process_name: string;
  description?: string;
  version: number;
  status: 'draft' | 'active' | 'suspended' | 'deprecated';
  category?: string;
  // 批次 98 P2-D 修复（v5 复审）：原 any 改为 Record<string, unknown>，动态 JSON Schema 字段
  form_schema?: Record<string, unknown>;
  nodes?: ProcessNode[];
  created_at: string;
  updated_at: string;
  created_by?: string;
}

export interface ProcessNode {
  id: string;
  type: 'start' | 'end' | 'approval' | 'condition' | 'notify';
  name: string;
  assignee_type?: 'user' | 'role' | 'department' | 'dynamic';
  assignee_value?: string | number;
  condition?: string;
  next_nodes?: string[];
}

export interface ProcessVersion {
  id: number;
  process_definition_id: number;
  version: number;
  status: 'draft' | 'active' | 'deprecated';
  change_log?: string;
  created_at: string;
  created_by?: string;
}

export interface ProcessTemplate {
  id: number;
  template_key: string;
  template_name: string;
  description?: string;
  category: string;
  icon?: string;
  process_definition?: ProcessDefinition;
  usage_count: number;
  created_at: string;
}

/**
 * /bpm/approval/execute 请求体（唯一真相：backend/src/handlers/bpm_handler.rs
 * ExecuteApprovalRequest：task_id / handler_id / handler_name / action / approval_opinion）
 * action 取值域与后端状态机一致：approve | reject
 */
export interface ApprovalAction {
  task_id: number;
  handler_id: number;
  handler_name: string;
  action: 'approve' | 'reject';
  approval_opinion?: string;
}

/** 转办请求体（backend TransferTaskRequest：new_assignee_id / transfer_reason） */
export interface TransferTaskAction {
  new_assignee_id: number;
  transfer_reason: string;
}

/**
 * 待办/已办任务行（唯一真相：backend models/bpm_task.rs Model，
 * 由 /bpm/tasks/pending、/bpm/tasks/completed 原样序列化返回）
 * 任务状态词表为小写：pending / completed / rejected / cancelled
 */
export interface ApprovalTask {
  id: number;
  task_no: string;
  instance_id: number;
  process_definition_id: number;
  node_id: string;
  node_name: string;
  node_type: string;
  task_type?: string | null;
  status?: string | null;
  priority?: string | null;
  actual_handler_id?: number | null;
  actual_handler_name?: string | null;
  action?: string | null;
  approval_opinion?: string | null;
  handled_at?: string | null;
  duration_seconds?: number | null;
  due_date?: string | null;
  is_overdue?: boolean | null;
  overdue_days?: number | null;
  created_at?: string | null;
  updated_at?: string | null;
  remarks?: string | null;
}

/**
 * 审批链节点（唯一真相：backend services/bpm_service_dto.rs ApprovalChainNode）
 * status 复用任务状态词表（pending/completed/rejected/cancelled）
 */
export interface ApprovalChainNode {
  node_id: string;
  node_name: string;
  node_type: string;
  assignee_id?: number | null;
  assignee_name?: string | null;
  status: string;
  comment?: string | null;
  completed_at?: string | null;
  due_time?: string | null;
}

// D14 Batch 5b：原 bpmEnhancedApi.listDefinitions 转为风格 B 函数
export const getBpmDefinitionList = (params?: {
  page?: number;
  page_size?: number;
  category?: string;
  keyword?: string;
}) => request.get<ApiResponse<PageResult<ProcessDefinition>>>('/bpm/definitions', { params });

// D14 Batch 5b：原 bpmEnhancedApi.getDefinition 转为风格 B 函数
export const getBpmDefinitionById = (id: number) =>
  request.get<ApiResponse<ProcessDefinition>>(`/bpm/definitions/${id}`);

// D14 Batch 5b：原 bpmEnhancedApi.createDefinition 转为风格 B 函数
export const createBpmDefinition = (data: Partial<ProcessDefinition>) =>
  request.post<ApiResponse<ProcessDefinition>>('/bpm/definitions', data);

// D14 Batch 5b：原 bpmEnhancedApi.updateDefinition 转为风格 B 函数
export const updateBpmDefinition = (id: number, data: Partial<ProcessDefinition>) =>
  request.put<ApiResponse<ProcessDefinition>>(`/bpm/definitions/${id}`, data);

// D14 Batch 5b：原 bpmEnhancedApi.deleteDefinition 转为风格 B 函数
export const deleteBpmDefinition = (id: number) =>
  request.delete<ApiResponse<null>>(`/bpm/definitions/${id}`);

// D14 Batch 5b：原 bpmEnhancedApi.createVersion 转为风格 B 函数
export const createBpmVersion = (definitionId: number, data?: { change_log?: string }) =>
  request.post<ApiResponse<ProcessVersion>>(`/bpm/definitions/${definitionId}/versions`, data);

// D14 Batch 5b：原 bpmEnhancedApi.listVersions 转为风格 B 函数
export const getBpmVersionList = (definitionId: number) =>
  request.get<ApiResponse<ProcessVersion[]>>(`/bpm/definitions/${definitionId}/versions`);

// D14 Batch 5b：原 bpmEnhancedApi.activateVersion 转为风格 B 函数
export const activateBpmVersion = (versionId: number) =>
  request.post<ApiResponse<null>>(`/bpm/versions/${versionId}/activate`);

// D14 Batch 5b：原 bpmEnhancedApi.saveAsTemplate 转为风格 B 函数
export const saveBpmAsTemplate = (
  definitionId: number,
  data: { template_name: string; category: string; description?: string }
) => request.post<ApiResponse<ProcessTemplate>>(`/bpm/definitions/${definitionId}/template`, data);

// D14 Batch 5b：原 bpmEnhancedApi.listTemplates 转为风格 B 函数
export const getBpmTemplateList = (params?: {
  page?: number;
  page_size?: number;
  category?: string;
}) => request.get<ApiResponse<PageResult<ProcessTemplate>>>('/bpm/templates', { params });

// D14 Batch 5b：原 bpmEnhancedApi.getTemplate 转为风格 B 函数
export const getBpmTemplateById = (id: number) =>
  request.get<ApiResponse<ProcessTemplate>>(`/bpm/templates/${id}`);

// D14 Batch 5b：原 bpmEnhancedApi.createFromTemplate 转为风格 B 函数
export const createBpmFromTemplate = (templateId: number, data?: { process_name?: string }) =>
  request.post<ApiResponse<ProcessDefinition>>(`/bpm/templates/${templateId}/create`, data);

// D14 Batch 5b：原 bpmEnhancedApi.deleteTemplate 转为风格 B 函数
export const deleteBpmTemplate = (id: number) =>
  request.delete<ApiResponse<null>>(`/bpm/templates/${id}`);

// D14 Batch 5b：原 bpmEnhancedApi.getPendingTasks 转为风格 B 函数
export const getBpmPendingTaskList = (params?: { page?: number; page_size?: number }) =>
  request.get<ApiResponse<PageResult<ApprovalTask>>>('/bpm/tasks/pending', { params });

// D14 Batch 5b：原 bpmEnhancedApi.getCompletedTasks 转为风格 B 函数
export const getBpmCompletedTaskList = (params?: { page?: number; page_size?: number }) =>
  request.get<ApiResponse<PageResult<ApprovalTask>>>('/bpm/tasks/completed', { params });

// D14 Batch 5b：原 bpmEnhancedApi.executeApproval 转为风格 B 函数
// 请求体字段与后端 ExecuteApprovalRequest 逐字一致（缺 handler_id/handler_name 会 422）
export const executeBpmApproval = (data: ApprovalAction) =>
  request.post<ApiResponse<string>>('/bpm/approval/execute', data);

// 转办任务（后端独立端点，不走 approval/execute：approval/execute 的 action 取值域仅 approve/reject）
export const transferBpmEnhancedTask = (taskId: number, data: TransferTaskAction) =>
  request.post<ApiResponse<string>>(`/bpm/tasks/${taskId}/transfer`, data);

// D14 Batch 5b：原 bpmEnhancedApi.getApprovalChain 转为风格 B 函数
export const getBpmEnhancedApprovalChain = (instanceId: number) =>
  request.get<ApiResponse<ApprovalChainNode[]>>(`/bpm/instances/${instanceId}/chain`);
