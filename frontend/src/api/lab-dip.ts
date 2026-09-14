import { request } from './request';
import type { ApiResponse, PaginatedResponse, QueryParams } from '@/types/api';

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

export interface CreateLabDipRequestPayload {
  customer_id?: number;
  customer_color_no?: string;
  customer_color_name?: string;
  sample_type?: string;
  fabric_spec?: string;
  fabric_component?: string;
  sample_size?: string;
  /** 主对色光源（必填）：D65/TL84/U3000/CWF/A 等 */
  light_source: string;
  secondary_light_source?: string;
  color_fastness_req?: string;
  eco_requirement?: string;
  sample_versions?: number;
  dye_category?: string;
  /** 客户要求交期（必填），格式 YYYY-MM-DD */
  required_date: string;
  expected_days?: number;
  remarks?: string;
}

export interface LabDipRequestListQuery extends QueryParams {
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

export function approveLabDipRequest(id: number): Promise<ApiResponse<LabDipRequest>> {
  return request.post(`/production/lab-dip/requests/${id}/approve`);
}

export function rejectLabDipRequest(id: number): Promise<ApiResponse<LabDipRequest>> {
  return request.post(`/production/lab-dip/requests/${id}/reject`);
}

export function restartLabDipSampling(id: number): Promise<ApiResponse<LabDipRequest>> {
  return request.post(`/production/lab-dip/requests/${id}/restart`);
}

export function completeLabDipRequest(id: number): Promise<ApiResponse<LabDipRequest>> {
  return request.post(`/production/lab-dip/requests/${id}/complete`);
}
