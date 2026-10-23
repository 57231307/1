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
  // 动态 JSON Schema 字段：类型用 Record<string, unknown>，不用 any
  form_schema?: Record<string, unknown>;
  nodes?: ProcessNode[];
  // 后端持久化真源：节点与边存于 config.nodes / config.edges
  // （解析首任务节点见 bpm_service.rs 的 resolve_first_task_node，推进见 bpm_ops/task.rs 的
  // try_advance_to_next_node，两者均从 config 读取）。GET 出参亦回传 config
  // （bpm_definition_handler 的 model_to_frontend_json）。建单/更新须把
  // nodes+edges 包进 config 提交，否则流程节点被后端静默丢弃（顶层 nodes 非契约字段）、
  // 审批推进因缺 edges 而跳过后续节点。
  config?: ProcessConfig & Record<string, unknown>;
  created_at: string;
  updated_at: string;
  created_by?: string;
}

/**
 * 流程节点类型词表：与审批引擎判定值逐字符一致
 * （bpm_service.rs 的 resolve_first_task_node、bpm_ops/task.rs 的 try_advance_to_next_node
 * 只识别 start_event / user_task / end_event）。
 * 引擎对其余 token（历史词 start/approval/condition/notify/end）无任何执行路径
 * （推进命中未知类型会被 fail-closed 拒绝，见 try_advance_to_next_node 的拒绝分支）。
 */
export type BpmNodeType = 'start_event' | 'user_task' | 'end_event';

/**
 * 流程审批人类型词表：与审批引擎判定值逐字符一致
 * （bpm_ops/mod.rs 的 resolve_node_assignee 只识别 user 与 dept_superior）。
 * - user：assignee_value 为具体用户 ID（须解析为正整数，否则拒绝发起）；
 * - dept_superior：按申请人所在部门的上一级部门负责人解析；节点带 sensitive=true
 *   时再上一级（该领导所在部门的上级负责人）。此类型不需要 assignee_value。
 * 引擎对其余取值（历史词 role/department/dynamic）直接拒绝发起，无执行路径。
 */
export type BpmAssigneeType = 'user' | 'dept_superior';

/**
 * 流程节点。引擎实际读取的键：id / type / name（resolve_first_task_node 与
 * bpm_ops/mod.rs 的 resolve_node_assignee 读 node 的 id / name）、
 * assignee_type / assignee_value（resolve_node_assignee）与 sensitive
 * （resolve_node_assignee 判定是否取再上一级，缺省即非敏感，存量定义无需迁移）。
 * edge_condition 为表单编辑态字段，承载该节点出边的条件表达式；提交时由
 * buildLinearEdges 写入 edges[].condition（引擎只在 edge.condition 上求值），
 * 不随节点持久化（handleSubmit 提交前剥离该键），因此不是引擎读取的节点键。
 * 节点级 condition / next_nodes 引擎从不读取，仍不声明。
 */
export interface ProcessNode {
  id: string;
  type: BpmNodeType;
  name: string;
  assignee_type?: BpmAssigneeType;
  assignee_value?: number;
  sensitive?: boolean;
  edge_condition?: string;
}

/**
 * 流程图边。引擎逐字符读取的键：source / target（resolve_first_task_node 按 source
 * 找 start 出边、try_advance_to_next_node 按 source 匹配出边并取 target）、
 * condition（try_advance_to_next_node 经 evaluate_bpm_condition 求值）。
 * condition 仅在为可解析表达式时有语义：解析失败引擎按 fail-closed 判不匹配
 * （见 bpm_service.rs 的 evaluate_bpm_condition），空串是 poison 值，因此无条件边必须省略该键。
 */
export interface ProcessEdge {
  source: string;
  target: string;
  condition?: string;
}

/**
 * 流程定义 config 持久化形状：nodes + edges 均在 config 内
 * （resolve_first_task_node 与 try_advance_to_next_node 均从 config 读取；缺 edges 时
 * try_advance_to_next_node 匹配不到推进边，首节点批准后流程直接完成 = 后续审批被绕过）。
 */
export interface ProcessConfig {
  nodes?: ProcessNode[];
  edges?: ProcessEdge[];
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
 * 后端不返回 template_key/template_name/icon/usage_count/process_definition 等键
 * （GET /bpm/templates 出参 = 流程定义行列，无模板专属键），类型因此不声明它们；
 * usage_count 统计后端尚未实现。
 */
export type ProcessTemplate = ProcessDefinition;

/**
 * POST /bpm/definitions 载荷（唯一真相：models/dto/bpm_dto.rs CreateProcessDefinitionRequest）。
 * name/code 为非 Option 必填（缺失 → serde 400/422，禁止用 Partial<ProcessDefinition> 掩盖）；
 * 流程节点与边必须包在 config.nodes / config.edges 提交（后端只从 config 提取，
 * 顶层 nodes 非 DTO 字段；缺 edges 会绕过后续审批推进）。
 */
export interface CreateProcessDefinitionPayload {
  name: string;
  code: string;
  description?: string;
  category?: string;
  version?: string;
  config?: ProcessConfig & Record<string, unknown>;
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
  config?: ProcessConfig & Record<string, unknown>;
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
 * handlers/bpm_definition_handler.rs::page_to_frontend_json 第 56-68 行）。
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

// 流程定义分页列表（GET /bpm/definitions）
// 查询键唯一真相：bpm_dto.rs ProcessDefinitionQuery{category,status,page,page_size}；
// 后端无 keyword 参数（名称模糊搜索后端未提供），前端不得声明被静默丢弃的键。
export const getBpmDefinitionList = (params?: {
  page?: number;
  page_size?: number;
  category?: string;
  status?: string;
}) => request.get<ApiResponse<ProcessDefinitionPage>>('/bpm/definitions', { params });

// 单个流程定义（GET /bpm/definitions/{id}）
export const getBpmDefinitionById = (id: number) =>
  request.get<ApiResponse<ProcessDefinition>>(`/bpm/definitions/${id}`);

// 创建流程定义（POST /bpm/definitions）
// 载荷 name/code 必填见 CreateProcessDefinitionPayload（后端 DTO 非 Option，禁止 Partial 掩盖）
export const createBpmDefinition = (data: CreateProcessDefinitionPayload) =>
  request.post<ApiResponse<ProcessDefinition>>('/bpm/definitions', data);

// 更新流程定义（PUT /bpm/definitions/{id}）
export const updateBpmDefinition = (id: number, data: UpdateProcessDefinitionPayload) =>
  request.put<ApiResponse<ProcessDefinition>>(`/bpm/definitions/${id}`, data);

// 删除流程定义（DELETE /bpm/definitions/{id}）
export const deleteBpmDefinition = (id: number) =>
  request.delete<ApiResponse<null>>(`/bpm/definitions/${id}`);

// 为流程定义创建新版本（POST /bpm/definitions/{id}/versions）
export const createBpmVersion = (definitionId: number, data?: { change_log?: string }) =>
  request.post<ApiResponse<ProcessVersion>>(`/bpm/definitions/${definitionId}/versions`, data);

// 流程定义的版本列表（GET /bpm/definitions/{id}/versions）
export const getBpmVersionList = (definitionId: number) =>
  request.get<ApiResponse<ProcessVersion[]>>(`/bpm/definitions/${definitionId}/versions`);

// 激活指定版本（POST /bpm/versions/{id}/activate）
export const activateBpmVersion = (versionId: number) =>
  request.post<ApiResponse<null>>(`/bpm/versions/${versionId}/activate`);

// 将流程定义存为模板（POST /bpm/definitions/{id}/template）
export const saveBpmAsTemplate = (
  definitionId: number,
  data: { template_name: string; category: string; description?: string }
) => request.post<ApiResponse<ProcessTemplate>>(`/bpm/definitions/${definitionId}/template`, data);

// 模板分页列表（GET /bpm/templates）
// 查询键唯一真相：bpm_dto.rs TemplateQuery{page,page_size}；后端不读 category
// （模板分类与 usage_count 统计后端未提供），前端不得声明被静默丢弃的键。
export const getBpmTemplateList = (params?: { page?: number; page_size?: number }) =>
  request.get<ApiResponse<ProcessTemplatePage>>('/bpm/templates', { params });

// 单个模板（GET /bpm/templates/{id}）
export const getBpmTemplateById = (id: number) =>
  request.get<ApiResponse<ProcessTemplate>>(`/bpm/templates/${id}`);

// 从模板创建新流程定义（POST /bpm/templates/{id}/create）：
// 后端 create_from_template(template_id, Json<CreateProcessDefinitionRequest>)：name/code 必填
// （请求体真实生效：客户端字段优先、缺省回退模板值），
// 未提供 config 时继承模板 config.nodes，故载荷必带新流程的 name+code。
export const createBpmFromTemplate = (templateId: number, data: CreateProcessDefinitionPayload) =>
  request.post<ApiResponse<ProcessDefinition>>(`/bpm/templates/${templateId}/create`, data);

// 删除模板（DELETE /bpm/templates/{id}）
export const deleteBpmTemplate = (id: number) =>
  request.delete<ApiResponse<null>>(`/bpm/templates/${id}`);

// 待办任务分页列表（GET /bpm/tasks/pending）
export const getBpmPendingTaskList = (params?: { page?: number; page_size?: number }) =>
  request.get<ApiResponse<ApprovalTaskPage>>('/bpm/tasks/pending', { params });

// 已办任务分页列表（GET /bpm/tasks/completed）
export const getBpmCompletedTaskList = (params?: { page?: number; page_size?: number }) =>
  request.get<ApiResponse<ApprovalTaskPage>>('/bpm/tasks/completed', { params });

// 执行审批（POST /bpm/approval/execute，action 取值域 approve/reject）
// 请求体字段与后端 ExecuteApprovalRequest 逐字一致（缺 handler_id/handler_name 会 422）
export const executeBpmApproval = (data: ApprovalAction) =>
  request.post<ApiResponse<string>>('/bpm/approval/execute', data);

// 转办任务（后端独立端点，不走 approval/execute：approval/execute 的 action 取值域仅 approve/reject）
export const transferBpmEnhancedTask = (taskId: number, data: TransferTaskAction) =>
  request.post<ApiResponse<string>>(`/bpm/tasks/${taskId}/transfer`, data);

// 审批链节点列表（GET /bpm/instances/{id}/chain）
export const getBpmEnhancedApprovalChain = (instanceId: number) =>
  request.get<ApiResponse<ApprovalChainNode[]>>(`/bpm/instances/${instanceId}/chain`);
