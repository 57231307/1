import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

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
  /**
   * 后端列表/详情出参并行补齐的富化键（sales_order_item LEFT JOIN sales_orders /
   * customers / users 派生）：键恒存在、无值即 null，禁止在前端标 `?` 或用 `?? ` 掩盖缺键。
   * order_no/customer_name 来自关联销售订单；approver_name 来自审批人用户；
   * sales_order_item_id 为可选 FK（NULL=订单级批色、未绑定具体明细行）。
   */
  order_no: string | null;
  customer_name: string | null;
  approver_name: string | null;
  sales_order_item_id: number | null;
  created_at: string;
  updated_at: string;
}

/**
 * 创建大货批色申请（对齐后端 CreateBulkColorApprovalDto，bulk_color_approval_handler.rs:44）。
 * 大货样确认必须绑定销售订单+染缸批次：sales_order_id、dye_batch_id、customer_id 均必填；其余可选。
 */
export interface CreateBulkColorApprovalPayload {
  sales_order_id: number;
  dye_batch_id: number;
  customer_id: number;
  /**
   * 可选绑定销售订单明细行：传入时后端以该行的 product_id/color_no 覆盖回填
   * （前端所传 product_id/color_no 仅作展示派生，最终以后端按明细行派生结果为准）。
   */
  sales_order_item_id?: number;
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

/**
 * 大货批色审批列表查询参数——严格对齐后端
 * bulk_color_approval_service.rs::ListBulkColorApprovalQuery。
 * 全部 Option 字段 → 可选；无 rename_all → 保持 snake_case。
 * 状态字段为 approval_status（不是 status），日期为 from_date/to_date（ISO 串）。
 */
export interface BulkColorApprovalListQuery {
  page?: number;
  page_size?: number;
  sales_order_id?: number;
  dye_batch_id?: number;
  customer_id?: number;
  approval_status?: string;
  from_date?: string;
  to_date?: string;
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
