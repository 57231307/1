import { request } from './request';
import type { ApiResponse } from '@/types/api';

/** 打印目标连接种类（唯一真相：backend models/status/print.rs print_target_kind，与建表 CHECK 同源） */
export type PrintTargetKind = 'cups' | 'ipp' | 'network' | 'system_command';

/**
 * 打印目标出参（逐列镜像 backend handlers/print_target_handler.rs PrintTargetRecord，
 * 序列化键即其 snake_case 字段名；时间列为后端 RFC3339 字符串）。
 */
export interface PrintTarget {
  id: number;
  target_code: string;
  display_name: string;
  kind: PrintTargetKind;
  queue_name: string | null;
  host: string | null;
  port: number | null;
  endpoint_url: string | null;
  command_template: string | null;
  is_pool: boolean;
  pool_members: string[];
  default_copies: number;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

/**
 * POST /print-targets 载荷（唯一真相：CreatePrintTargetRequest）。
 * target_code/display_name/kind 必填；连接字段按 kind 选用，非池行缺对应必需字段后端 fail-visible 400。
 * 其余为后端 #[serde(default)] 可选键，缺省即省略、不得发送空串。
 */
export interface CreatePrintTargetRequest {
  target_code: string;
  display_name: string;
  kind: PrintTargetKind;
  queue_name?: string;
  host?: string;
  port?: number;
  endpoint_url?: string;
  command_template?: string;
  is_pool?: boolean;
  pool_members?: string[];
  default_copies?: number;
  enabled?: boolean;
}

/** PUT /print-targets/{id} 载荷（唯一真相：UpdatePrintTargetRequest，全字段 Option，仅提交需改列）。 */
export type UpdatePrintTargetRequest = Partial<CreatePrintTargetRequest>;

// 后端 print_handler::list_print_targets 无 Query 提取器（全量返回），传参被忽略 -> 不声明参数
export function getPrintTargetList(): Promise<ApiResponse<PrintTarget[]>> {
  return request.get('/print-targets');
}

export function getPrintTarget(id: number): Promise<ApiResponse<PrintTarget>> {
  return request.get(`/print-targets/${id}`);
}

export function createPrintTarget(
  data: CreatePrintTargetRequest
): Promise<ApiResponse<PrintTarget>> {
  return request.post('/print-targets', data);
}

export function updatePrintTarget(
  id: number,
  data: UpdatePrintTargetRequest
): Promise<ApiResponse<PrintTarget>> {
  return request.put(`/print-targets/${id}`, data);
}

export function deletePrintTarget(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/print-targets/${id}`);
}
