import { request } from './request';
import type { ApiResponse } from '@/types/api-response';
import type {
  MonitorStatsResponse,
  MonitorPendingTasksParams,
  MonitorInstancesParams,
} from '@/types/bpm';

/**
 * 后端 models/dto/bpm_dto.rs StartProcessRequest 的必填项：
 * process_key / business_type / business_id / title / initiator_id / initiator_name
 * （原 types/bpm.ts 的 {process_key, business_key} 两字段契约缺 4 个必填项，提交必 422）
 */
export interface BpmStartProcessRequest {
  process_key: string;
  business_type: string;
  business_id: number;
  title: string;
  initiator_id: number;
  initiator_name: string;
  initiator_department_id?: number;
  priority?: string;
  form_data?: Record<string, unknown>;
  variables?: Record<string, unknown>;
}

/** 后端 StartProcessResponse */
export interface BpmStartProcessResponse {
  instance_id: number;
  instance_no: string;
  task_ids: number[];
}

/**
 * 后端 models/dto/bpm_dto.rs ApproveTaskRequest 的字段（task_id/handler_id/handler_name/action 必填）。
 * action 取值域 approve | reject（services/bpm_ops/task.rs ALL_APPROVE_ACTIONS），越界值后端显式拒绝。
 */
export interface BpmApproveTaskRequest {
  task_id: number;
  handler_id: number;
  handler_name: string;
  action: 'approve' | 'reject';
  approval_opinion?: string;
  attachment_urls?: string[];
}

/**
 * 后端 handlers/bpm_handler.rs 任务列表返回 PaginatedResponse<bpm_task::Model>：
 * 行对象字段以 models/bpm_task.rs 为准（id / task_no / instance_id / node_name / status ...）。
 * 任务状态词表为小写 pending / completed / rejected / cancelled。
 */
export interface BPMTask {
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
  approval_opinion?: string | null;
  handled_at?: string | null;
  due_date?: string | null;
  is_overdue?: boolean | null;
  created_at?: string | null;
  updated_at?: string | null;
  remarks?: string | null;
}

/**
 * 后端 models/bpm_process_instance.rs Model（/bpm/monitor/instances 原样序列化）。
 * 流程实例状态词表为大写：PROCESSING / COMPLETED / TERMINATED / CANCELLED
 * （models/status/bpm_crm_contract.rs 的 bpm_instance，写入点见 services/bpm_ops/*）。
 */
export interface BPMInstance {
  id: number;
  instance_no: string;
  process_definition_id: number;
  business_type: string;
  business_id: number;
  applicant_id: number;
  title: string;
  priority?: string | null;
  current_node_id?: string | null;
  current_node_name?: string | null;
  status?: string | null;
  initiator_id: number;
  initiator_name: string;
  initiator_department_id?: number | null;
  current_handler_ids?: number[] | null;
  current_handler_names?: string[] | null;
  started_at?: string | null;
  completed_at?: string | null;
  duration_seconds?: number | null;
  created_at?: string | null;
  updated_at?: string | null;
}

/**
 * 后端 services/bpm_service_dto.rs ApprovalChainNode（/bpm/instances/{id}/approval-chain）。
 * status 复用任务状态词表（小写 pending/completed/rejected/cancelled）。
 */
export interface ApprovalChainItem {
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
 * 任务状态词表（唯一真相：models/status/bpm_crm_contract.rs bpm_task，小写）。
 * 写入点见 services/bpm_ops/task.rs。
 */
export type BPMTaskStatus = 'pending' | 'completed' | 'rejected' | 'cancelled';

/**
 * 流程实例状态词表（唯一真相：models/status/bpm_crm_contract.rs bpm_instance，大写）。
 */
export type BPMInstanceStatus = 'PROCESSING' | 'COMPLETED' | 'TERMINATED' | 'CANCELLED';

/**
 * 业务关联查询响应（唯一真相：services/bpm_service_dto.rs:10-20 BpmBusinessRelation，
 * 由 handlers/bpm_handler.rs get_business_relation 原样序列化，扁平结构）。
 */
export interface BpmBusinessRelation {
  has_process: boolean;
  instance_id: number;
  instance_no: string;
  process_status: string;
  started_at: string;
  completed_at: string | null;
  task_count: number;
  completed_tasks: number;
  pending_tasks: number;
}

/**
 * 流程实例详情响应（唯一真相：services/bpm_service_dto.rs:52-57 ProcessInstanceDetail）。
 * 后端返回的是**嵌套**结构：instance 为完整 bpm_process_instance::Model，
 * tasks 为 bpm_task::Model 数组，approval_chain 为 ApprovalChainNode 数组。
 */
export interface BpmInstanceDetail {
  instance: BPMInstance;
  definition_name: string;
  tasks: BPMTask[];
  approval_chain: ApprovalChainItem[];
}

/** 可视化响应内嵌的实例节点（handlers/bpm_handler.rs:121-129，字段为后端 json! 手挑的子集） */
export interface BpmVizInstanceNode {
  id: number;
  instance_no: string;
  business_type: string;
  business_id: number;
  status: string | null;
  start_time: string | null;
  end_time: string | null;
}

/** 可视化响应内嵌的任务节点（handlers/bpm_handler.rs:106-116） */
export interface BpmVizTaskNode {
  id: number;
  task_no: string;
  node_id: string;
  node_name: string;
  status: string | null;
  assignee_id: number | null;
  created_at: string | null;
  completed_at: string | null;
  comment: string | null;
}

/**
 * 流程可视化响应（唯一真相：handlers/bpm_handler.rs:120-138 get_process_visualization，
 * 返回 { instance, definition, tasks, timeline } 的**嵌套**结构）。
 */
export interface BpmProcessVisualization {
  instance: BpmVizInstanceNode;
  definition: { id: number; code: string; name: string; config: unknown } | null;
  tasks: BpmVizTaskNode[];
  timeline: BpmVizTaskNode[];
}

// D14 Batch 5b：原 bpmApi.startProcess 转为风格 B 函数
export const startBpmProcess = (data: BpmStartProcessRequest) =>
  request.post<ApiResponse<BpmStartProcessResponse>>('/bpm/process/start', data);

// D14 Batch 5b：原 bpmApi.approveTask 转为风格 B 函数
export const approveBpmTask = (data: BpmApproveTaskRequest) =>
  request.post<ApiResponse<null>>('/bpm/tasks/approve', data);

// 任务列表：后端返回 PaginatedResponse<bpm_task::Model>（items/total/page/page_size）
export const getBpmTaskList = (params?: {
  user_id?: number;
  status?: string;
  page?: number;
  page_size?: number;
}) =>
  request.get<ApiResponse<{ items: BPMTask[]; total: number; page: number; page_size: number }>>(
    '/bpm/tasks',
    { params }
  );

// D14 Batch 5b：原 bpmApi.transferTask 转为风格 B 函数
// 请求体字段与后端 TransferTaskRequest 逐字一致：new_assignee_id / transfer_reason
export const transferBpmTask = (taskId: number, newAssigneeId: number, transferReason: string) =>
  request.post<ApiResponse<null>>(`/bpm/tasks/${taskId}/transfer`, {
    new_assignee_id: newAssigneeId,
    transfer_reason: transferReason,
  });

// D14 Batch 5b：原 bpmApi.urgeTask 转为风格 B 函数（后端 UrgeTaskRequest.urge_message 必填）
export const urgeBpmTask = (taskId: number, urgeMessage: string) =>
  request.post<ApiResponse<null>>(`/bpm/tasks/${taskId}/urge`, { urge_message: urgeMessage });

// D14 Batch 5b：原 bpmApi.getBusinessRelation 转为风格 B 函数
export const getBpmBusinessRelation = (businessType: string, businessId: number) =>
  request.get<ApiResponse<BpmBusinessRelation>>('/bpm/business-relation', {
    params: { business_type: businessType, business_id: businessId },
  });

// D14 Batch 5b：原 bpmApi.getProcessVisualization 转为风格 B 函数
export const getBpmProcessVisualization = (instanceId: string) =>
  request.get<ApiResponse<BpmProcessVisualization>>(`/bpm/visualization/${instanceId}`);

// D14 Batch 5b：原 bpmApi.getApprovalChain 转为风格 B 函数
export const getBpmApprovalChain = (instanceId: string) =>
  request.get<ApiResponse<ApprovalChainItem[]>>(`/bpm/instances/${instanceId}/approval-chain`);

// D14 Batch 5b：原 bpmApi.getInstanceDetail 转为风格 B 函数
export const getBpmInstanceById = (instanceId: string) =>
  request.get<ApiResponse<BpmInstanceDetail>>(`/bpm/instances/${instanceId}/detail`);

// D14 Batch 5b：原 bpmApi.getMonitorStats 转为风格 B 函数
export const getBpmMonitorStats = () =>
  request.get<ApiResponse<MonitorStatsResponse>>('/bpm/monitor/stats');

// D14 Batch 5b：原 bpmApi.getPendingTasksForMonitor 转为风格 B 函数
export const getBpmPendingTaskList = (params?: MonitorPendingTasksParams) =>
  request.get<ApiResponse<{ items: BPMTask[]; total: number; page: number; page_size: number }>>(
    '/bpm/monitor/pending-tasks',
    {
      params,
    }
  );

// D14 Batch 5b：原 bpmApi.listInstancesForMonitor 转为风格 B 函数
export const getBpmInstanceListForMonitor = (params?: MonitorInstancesParams) =>
  request.get<
    ApiResponse<{ items: BPMInstance[]; total: number; page: number; page_size: number }>
  >('/bpm/monitor/instances', {
    params,
  });

// 批次 157d-3 新增：撤回流程实例（接收实例主键 id: number）
// D14 Batch 5b：原 bpmApi.cancelInstance 转为风格 B 函数
export const cancelBpmInstance = (instanceId: number, cancelReason?: string) =>
  request.post<ApiResponse<null>>(`/bpm/instances/${instanceId}/cancel`, {
    cancel_reason: cancelReason,
  });
