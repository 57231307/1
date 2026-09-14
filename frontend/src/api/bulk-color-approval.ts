import { request } from './request';
import type { ApiResponse, PaginatedResponse, QueryParams } from '@/types/api';

/**
 * 大货批色审批（bulk_color_approval）状态
 * 后端状态机：pending→sampled→sent_to_customer→approved/rejected/rework；downgraded/scrapped 为终态
 */
export type BulkColorApprovalStatus =
  | 'pending'
  | 'sampled'
  | 'sent_to_customer'
  | 'approved'
  | 'rejected'
  | 'rework'
  | 'downgraded'
  | 'scrapped';

export interface BulkColorApproval {
  id: number;
  sales_order_id: number;
  dye_batch_id: number;
  customer_id: number;
  production_order_id: number | null;
  product_id: number | null;
  color_no: string | null;
  dye_lot_no: string | null;
  batch_no: string | null;
  sample_type: string;
  sample_length_m: string | number | null;
  approval_status: BulkColorApprovalStatus;
  approver_id: number | null;
  approval_date: string | null;
  sent_to_customer_at: string | null;
  customer_feedback: string | null;
  delta_e_value: string | number | null;
  reject_reason: string | null;
  delivery_blocking: boolean;
  attachment_url: string | null;
  remark: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateBulkColorApprovalPayload {
  /** 销售订单 ID（必填） */
  sales_order_id: number;
  /** 染色批次 ID（必填） */
  dye_batch_id: number;
  /** 客户 ID（必填） */
  customer_id: number;
  production_order_id?: number;
  product_id?: number;
  color_no?: string;
  dye_lot_no?: string;
  batch_no?: string;
  sample_type?: string;
  remark?: string;
}

export interface CutSamplePayload {
  sample_length_m: number;
  sample_piece_id?: number;
  attachment_url?: string;
  delta_e_value?: number;
}

export interface BulkColorApprovalListQuery extends QueryParams {
  approval_status?: string;
}

export function getBulkColorApprovalList(
  params?: BulkColorApprovalListQuery
): Promise<ApiResponse<PaginatedResponse<BulkColorApproval>>> {
  return request.get('/bulk-color-approvals', { params });
}

export function getBulkColorApproval(id: number): Promise<ApiResponse<BulkColorApproval>> {
  return request.get(`/bulk-color-approvals/${id}`);
}

export function createBulkColorApproval(
  data: CreateBulkColorApprovalPayload
): Promise<ApiResponse<BulkColorApproval>> {
  return request.post('/bulk-color-approvals', data);
}

export function cutBulkColorSample(
  id: number,
  data: CutSamplePayload
): Promise<ApiResponse<BulkColorApproval>> {
  return request.post(`/bulk-color-approvals/${id}/cut-sample`, data);
}

export function sendBulkColorToCustomer(id: number): Promise<ApiResponse<BulkColorApproval>> {
  return request.post(`/bulk-color-approvals/${id}/send-to-customer`);
}

export function approveBulkColor(
  id: number,
  data?: { feedback?: string; delta_e_value?: number }
): Promise<ApiResponse<BulkColorApproval>> {
  return request.post(`/bulk-color-approvals/${id}/approve`, data ?? {});
}

export function rejectBulkColor(
  id: number,
  data: { reject_reason: string; feedback?: string }
): Promise<ApiResponse<BulkColorApproval>> {
  return request.post(`/bulk-color-approvals/${id}/reject`, data);
}

export function reworkBulkColor(
  id: number,
  data: { reject_reason: string; feedback?: string }
): Promise<ApiResponse<BulkColorApproval>> {
  return request.post(`/bulk-color-approvals/${id}/rework`, data);
}

export function downgradeBulkColor(
  id: number,
  data: { reject_reason: string }
): Promise<ApiResponse<BulkColorApproval>> {
  return request.post(`/bulk-color-approvals/${id}/downgrade`, data);
}

export function scrapBulkColor(
  id: number,
  data: { reject_reason: string }
): Promise<ApiResponse<BulkColorApproval>> {
  return request.post(`/bulk-color-approvals/${id}/scrap`, data);
}

export function getBulkColorApprovalHistory(id: number): Promise<ApiResponse<unknown[]>> {
  return request.get(`/bulk-color-approvals/${id}/history`);
}

export function getBulkColorApprovalStatistics(): Promise<ApiResponse<unknown>> {
  return request.get('/bulk-color-approvals/statistics');
}
