import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

/**
 * 打样通知单（lab_dip_request）状态
 * 后端状态机：pending→sampling→submitted→approved/rejected→completed；rejected 可 restart 回 sampling
 */
export type LabDipRequestStatus =
  'pending' | 'sampling' | 'submitted' | 'approved' | 'rejected' | 'completed';

export interface LabDipRequest {
  id: number;
  request_no: string;
  customer_id: number | null;
  customer_color_no: string | null;
  customer_color_name: string | null;
  sample_type: string | null;
  fabric_spec: string | null;
  fabric_component: string | null;
  sample_size: string | null;
  light_source: string;
  secondary_light_source: string | null;
  color_fastness_req: string | null;
  eco_requirement: string | null;
  sample_versions: number;
  dye_category: string | null;
  required_date: string;
  expected_days: number | null;
  status: LabDipRequestStatus;
  remarks: string | null;
  created_at: string;
}

/**
 * 创建打样通知单（对齐后端 CreateLabDipRequestRequest，lab_dip_ops/types.rs:18）。
 * light_source 主对色光源（必填）、required_date 客户要求交期（必填 YYYY-MM-DD）；其余可选。
 */
export interface CreateLabDipRequestPayload {
  customer_id?: number;
  customer_color_no?: string;
  customer_color_name?: string;
  sample_type?: string;
  fabric_spec?: string;
  fabric_component?: string;
  sample_size?: string;
  light_source: string;
  secondary_light_source?: string;
  color_fastness_req?: string;
  eco_requirement?: string;
  sample_versions?: number;
  dye_category?: string;
  required_date: string;
  expected_days?: number;
  remarks?: string;
  created_by?: number;
}

/** 客户确认 OK 样（对齐后端 ApproveOkSampleRequest，lab_dip_handler.rs:162）：sample_id 必填，comment 可选 */
export interface ApproveLabDipPayload {
  sample_id: number;
  comment?: string;
}

/** 完成建库（对齐后端 CompleteRequest，lab_dip_handler.rs:222）：production_recipe_id 必填 */
export interface CompleteLabDipPayload {
  production_recipe_id: number;
}

/**
 * 打样通知单列表查询参数——严格对齐后端 lab_dip_handler.rs::LabDipRequestListQuery。
 * 全部 Option 字段 → 可选；无 rename_all → 保持 snake_case。
 */
export interface LabDipRequestListQuery {
  page?: number;
  page_size?: number;
  request_no?: string;
  customer_id?: number;
  status?: string;
}

export function getLabDipRequestList(
  params?: LabDipRequestListQuery
): Promise<ApiResponse<PaginatedResponse<LabDipRequest>>> {
  return request.get('/production/lab-dip/requests', { params });
}

export function getLabDipRequest(id: number): Promise<ApiResponse<LabDipRequest>> {
  return request.get(`/production/lab-dip/requests/${id}`);
}

export function createLabDipRequest(
  data: CreateLabDipRequestPayload
): Promise<ApiResponse<LabDipRequest>> {
  return request.post('/production/lab-dip/requests', data);
}

export function updateLabDipRequest(
  id: number,
  data: Partial<CreateLabDipRequestPayload>
): Promise<ApiResponse<LabDipRequest>> {
  return request.put(`/production/lab-dip/requests/${id}`, data);
}

export function deleteLabDipRequest(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/production/lab-dip/requests/${id}`);
}

export function startLabDipSampling(id: number): Promise<ApiResponse<LabDipRequest>> {
  return request.post(`/production/lab-dip/requests/${id}/start-sampling`);
}

export function submitLabDipRequest(id: number): Promise<ApiResponse<LabDipRequest>> {
  return request.post(`/production/lab-dip/requests/${id}/submit`);
}

export function approveLabDipRequest(
  id: number,
  data: ApproveLabDipPayload
): Promise<ApiResponse<LabDipRequest>> {
  return request.post(`/production/lab-dip/requests/${id}/approve`, data);
}

export function rejectLabDipRequest(id: number): Promise<ApiResponse<LabDipRequest>> {
  return request.post(`/production/lab-dip/requests/${id}/reject`);
}

export function restartLabDipSampling(id: number): Promise<ApiResponse<LabDipRequest>> {
  return request.post(`/production/lab-dip/requests/${id}/restart`);
}

export function completeLabDipRequest(
  id: number,
  data: CompleteLabDipPayload
): Promise<ApiResponse<LabDipRequest>> {
  return request.post(`/production/lab-dip/requests/${id}/complete`, data);
}
