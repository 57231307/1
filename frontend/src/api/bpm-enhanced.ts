import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface ProcessDefinition {
  id: number;
  // 实体真源键（model_to_frontend_json 恒定回传 code/name；process_key/process_name 为
  // 同值别名键，读取处两者均可，建载荷一律用 name/code）
  code: string;
  name: string;
  process_key: string;
  process_name: string;
  description?: string;
  version?: string;
  status: 'draft' | 'active' | 'suspended' | 'deprecated';
  category?: string;
  // 批次 98 P2-D 修复（v5 复审）：原 any 改为 Record<string, unknown>，动态 JSON Schema 字段
  form_schema?: Record<string, unknown>;
  nodes?: ProcessNode[];
  // 后端持久化真源：节点存于 config.nodes（bpm_service.rs:141 resolve_first_task_node），
  // GET 出参亦回传 config（bpm_definition_handler model_to_frontend_json）。建单/更新须把
  // nodes 包进 config 提交，否则流程节点被后端静默丢弃（顶层 nodes 非契约字段）。
  config?: { nodes?: ProcessNode[] } & Record<string, unknown>;
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
  version: string;
  status: 'draft' | 'active' | 'deprecated';
  change_log?: string;
  created_at: string;
  created_by?: string;
}

/**
 * 流程模板行（GET /bpm/templates 真实形状，唯一真相：bpm_definition_handler.rs
 * page_to_frontend_json → model_to_frontend_json：模板即 category='__TEMPLATE__' 的
 * bpm_process_definition 记录，行列与流程定义完全同构）。
 * 后端不返回 template_key/template_name/icon/usage_count/process_definition（此前为臆造键，
 * 读取处恒 undefined；usage_count 统计为后端缺口，已登记串行清单）。
 */
export type ProcessTemplate = ProcessDefinition;

/**
 * POST /bpm/definitions 载荷（唯一真相：models/dto/bpm_dto.rs CreateProcessDefinitionRequest）。
 * name/code 为非 Option 必填（缺失 → serde 400/422，禁止用 Partial<ProcessDefinition> 掩盖）；
 * 流程节点必须包在 config.nodes（后端从 config 提取，顶层 nodes 非 DTO 字段）。
 */
export interface CreateProcessDefinitionPayload {
  name: string;
  code: string;
  description?: string;
  category?: string;
  version?: string;
  config?: { nodes?: ProcessNode[] } & Record<string, unknown>;
  status?: ProcessDefinition['status'];
}

/**
 * PUT /bpm/definitions/{id} 载荷（唯一真相：UpdateProcessDefinitionRequest）。
 * 全字段 Option；后端无 code 字段——流程编码创建后不可改。
 */
export interface UpdateProcessDefinitionPayload {
  name?: string;
  description?: string;
  category?: string;
  config?: { nodes?: ProcessNode[] } & Record<string, unknown>;
  status?: ProcessDefinition['status'];
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

/**
 * GET /bpm/definitions 真实响应载荷（唯一真相：backend
 * handlers/bpm_definition_handler.rs::page_to_frontend_json 第 47-61 行）。
 * 逐条 model_to_frontend_json 后以统一分页信封承载列表，列表键为 `items`，
 * 另含 total/page/page_size（与 utils/response.rs 的 PaginatedResponse 一致）。
 */
export interface ProcessDefinitionPage {
  items: ProcessDefinition[];
  total: number;
  page: number;
  page_size: number;
}

/**
 * GET /bpm/templates 真实响应载荷：与 definitions 共用 page_to_frontend_json 构造，
 * 承载列表的键同为 `items`。
 */
export interface ProcessTemplatePage {
  items: ProcessTemplate[];
  total: number;
  page: number;
  page_size: number;
}

/**
 * GET /bpm/tasks/pending、/bpm/tasks/completed 真实响应载荷（唯一真相：
 * handlers/bpm_handler.rs::get_pending_tasks/get_completed_tasks 调
 * services/bpm_ops/task.rs::query_user_tasks 返回 PaginatedResponse<bpm_task::Model>，
 * utils/response.rs:34 序列化为 {items,total,page,page_size}）。
 * 承载列表的键为 `items`。
 */
export interface ApprovalTaskPage {
  items: ApprovalTask[];
  total: number;
  page: number;
  page_size: number;
}

// D14 Batch 5b：原 bpmEnhancedApi.listDefinitions 转为风格 B 函数
// 查询键唯一真相：bpm_dto.rs ProcessDefinitionQuery{category,status,page,page_size}；
// 后端无 keyword 参数（名称模糊搜索为后端缺口，登记串行清单），前端不得声明被静默丢弃的键。
export const getBpmDefinitionList = (params?: {
  page?: number;
  page_size?: number;
  category?: string;
  status?: string;
}) => request.get<ApiResponse<ProcessDefinitionPage>>('/bpm/definitions', { params });

// D14 Batch 5b：原 bpmEnhancedApi.getDefinition 转为风格 B 函数
export const getBpmDefinitionById = (id: number) =>
  request.get<ApiResponse<ProcessDefinition>>(`/bpm/definitions/${id}`);

// D14 Batch 5b：原 bpmEnhancedApi.createDefinition 转为风格 B 函数
// 载荷 name/code 必填见 CreateProcessDefinitionPayload（后端 DTO 非 Option，禁止 Partial 掩盖）
export const createBpmDefinition = (data: CreateProcessDefinitionPayload) =>
  request.post<ApiResponse<ProcessDefinition>>('/bpm/definitions', data);

// D14 Batch 5b：原 bpmEnhancedApi.updateDefinition 转为风格 B 函数
export const updateBpmDefinition = (id: number, data: UpdateProcessDefinitionPayload) =>
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
// 查询键唯一真相：bpm_dto.rs TemplateQuery{page,page_size}；后端不读 category
// （模板分类与 usage_count 统计为后端缺口，登记串行清单），前端不得声明被静默丢弃的键。
export const getBpmTemplateList = (params?: { page?: number; page_size?: number }) =>
  request.get<ApiResponse<ProcessTemplatePage>>('/bpm/templates', { params });

// D14 Batch 5b：原 bpmEnhancedApi.getTemplate 转为风格 B 函数
export const getBpmTemplateById = (id: number) =>
  request.get<ApiResponse<ProcessTemplate>>(`/bpm/templates/${id}`);

// D14 Batch 5b：原 bpmEnhancedApi.createFromTemplate 转为风格 B 函数
// 后端 create_from_template(template_id, Json<CreateProcessDefinitionRequest>)：name/code 必填
// （批次 199 P1-6 后请求体真实生效，客户端字段优先、缺省回退模板值），
// 未提供 config 时继承模板 config.nodes，故载荷必带新流程的 name+code。
export const createBpmFromTemplate = (templateId: number, data: CreateProcessDefinitionPayload) =>
  request.post<ApiResponse<ProcessDefinition>>(`/bpm/templates/${templateId}/create`, data);

// D14 Batch 5b：原 bpmEnhancedApi.deleteTemplate 转为风格 B 函数
export const deleteBpmTemplate = (id: number) =>
  request.delete<ApiResponse<null>>(`/bpm/templates/${id}`);

// D14 Batch 5b：原 bpmEnhancedApi.getPendingTasks 转为风格 B 函数
export const getBpmPendingTaskList = (params?: { page?: number; page_size?: number }) =>
  request.get<ApiResponse<ApprovalTaskPage>>('/bpm/tasks/pending', { params });

// D14 Batch 5b：原 bpmEnhancedApi.getCompletedTasks 转为风格 B 函数
export const getBpmCompletedTaskList = (params?: { page?: number; page_size?: number }) =>
  request.get<ApiResponse<ApprovalTaskPage>>('/bpm/tasks/completed', { params });

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
