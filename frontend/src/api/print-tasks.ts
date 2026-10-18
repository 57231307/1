import { request } from './request';
import type { ApiResponse } from '@/types/api';
import type { PaginatedResponse } from '@/types/api';

/** 打印任务状态词表（唯一真相：backend models/status/print.rs print_task_status::ALL，与建表 CHECK 同源） */
export type PrintTaskStatus = 'pending' | 'queued' | 'dispatched' | 'succeeded' | 'failed';

/** 目标种类（backend print_tasks.target_kind：printer 单机 / pool 池） */
export type PrintTaskTargetKind = 'printer' | 'pool';

/** 派发后端种类（backend models/status/print.rs print_target_kind） */
export type PrintTaskBackendKind = 'cups' | 'ipp' | 'network' | 'system_command';

/**
 * POST /print-tasks 载荷（唯一真相：print_task_handler::CreatePrintTaskRequest）。
 * doc_type + doc_id 必填定位单据实例；target_name 省略时由后端取适用模板的 default_target_name，
 * 两者都缺即 fail-visible 400。template_id 省略时后端按 doc_type 取「启用且默认」模板行。
 */
export interface CreatePrintTaskRequest {
  doc_type: string;
  doc_id: number;
  target_name?: string;
  template_id?: number;
  copies?: number;
  paper_size?: string;
  orientation?: string;
  margin_top_mm?: number;
  margin_bottom_mm?: number;
  margin_left_mm?: number;
  margin_right_mm?: number;
  barcode_enabled?: boolean;
}

/** 受理成功出参（PrintTaskJobRef）：轮询详情所需的任务主键与任务号 */
export interface PrintTaskJobRef {
  id: number;
  job_code: string;
}

/** 打印任务详情（逐列镜像 PrintTaskRecord，时间列为后端 RFC3339 字符串） */
export interface PrintTask {
  id: number;
  job_code: string;
  requested_by: number;
  requested_by_name: string;
  doc_type: string;
  doc_id: number;
  template_id: number | null;
  target_kind: PrintTaskTargetKind;
  target_name: string;
  copies: number;
  paper_size: string;
  orientation: string;
  margin_top_mm: number;
  margin_bottom_mm: number;
  margin_left_mm: number;
  margin_right_mm: number;
  barcode_enabled: boolean;
  source_module: string;
  status: PrintTaskStatus;
  backend_kind: PrintTaskBackendKind;
  external_job_id: string | null;
  error_message: string | null;
  dispatched_at: string | null;
  completed_at: string | null;
  created_at: string;
  updated_at: string;
}

/** GET /print-tasks 查询参数（后端 ListPrintTasksQuery：page/page_size/doc_type/status） */
export interface ListPrintTasksParams {
  page?: number;
  page_size?: number;
  doc_type?: string;
  status?: string;
}

/** 受理一次打印任务，返回 { id, job_code } 供轮询 getPrintTask */
export function createPrintTask(
  data: CreatePrintTaskRequest
): Promise<ApiResponse<PrintTaskJobRef>> {
  return request.post('/print-tasks', data);
}

/** 单任务详情：前端据 status/error_message 判定真实终态，不自行推断成败 */
export function getPrintTask(id: number): Promise<ApiResponse<PrintTask>> {
  return request.get(`/print-tasks/${id}`);
}

/** 任务台账（分页元数据置于 data，与 PaginatedResponse 单一口径一致） */
export function getPrintTaskList(
  params?: ListPrintTasksParams
): Promise<ApiResponse<PaginatedResponse<PrintTask>>> {
  return request.get('/print-tasks', { params });
}
