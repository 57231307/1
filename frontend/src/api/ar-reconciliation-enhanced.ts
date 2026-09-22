import { request } from './request';
import type { ApiResponse, PageResult } from '@/types/api';

/**
 * 账龄分桶（对齐后端 services::ar::AgingBucket）。
 * 金额为 Decimal，serde 序列化为字符串。
 */
export interface AgingBucket {
  label: string;
  min_days: number;
  max_days: number | null;
  amount: string;
  count: number;
}

/** 单客户账龄汇总（对齐后端 services::ar::CustomerAgingSummary） */
export interface CustomerAgingSummary {
  customer_id: number;
  customer_name: string;
  total_amount: string;
  buckets: AgingBucket[];
}

/**
 * 账龄报告（对齐后端 services::ar::AgingReport）。
 * 后端 `ar_reconciliation_handler::aging_report` 以 `to_value(report)` 返回单个对象，
 * 图表应读取 `overall_buckets`，而非把 data 当数组。
 */
export interface AgingReport {
  analysis_date: string;
  total_receivable: string;
  customer_summaries: CustomerAgingSummary[];
  overall_buckets: AgingBucket[];
}

export interface AutoReconciliationResult {
  id: number;
  customer_id: number;
  customer_name: string;
  customer_code: string;
  match_status: 'matched' | 'partial' | 'unmatched';
  invoice_amount: number;
  payment_amount: number;
  difference: number;
  matched_count: number;
  unmatched_count: number;
  created_at: string;
}

/**
 * 对账明细行（对齐后端 services::ar::ReconciliationDetail）。
 * 金额 Decimal 序列化为字符串。
 */
export interface ReconciliationDetailItem {
  id: number;
  reconciliation_id: number;
  item_type: string;
  document_type: string | null;
  document_id: number | null;
  document_no: string | null;
  document_date: string | null;
  amount: string;
  matched_amount: string | null;
  match_status: string;
  matched_item_id: number | null;
  remarks: string | null;
}

/**
 * 对账单详情（含明细）。后端 `ar_reconciliation_handler::get_reconciliation_details`
 * 以 `to_value(details)` 返回单对象 `{ reconciliation, details }`，
 * 明细数组承载在 `details` 键下（不是裸数组）。
 */
export interface ReconciliationWithDetails {
  reconciliation: Record<string, unknown>;
  details: ReconciliationDetailItem[];
}

export interface CustomerConfirmation {
  id: number;
  reconciliation_id: number;
  customer_id: number;
  customer_name: string;
  confirm_status: 'pending' | 'confirmed' | 'disputed';
  confirm_amount: number;
  disputed_amount: number;
  confirmed_at?: string;
  confirmed_by?: string;
  remark?: string;
}

export interface DisputeRecord {
  id: number;
  confirmation_id: number;
  reconciliation_id: number;
  dispute_type: 'amount' | 'quality' | 'delivery' | 'other';
  dispute_amount: number;
  description: string;
  status: 'open' | 'investigating' | 'resolved' | 'closed';
  resolution?: string;
  created_at: string;
  resolved_at?: string;
  created_by?: string;
}

// P2-9c 修复（批次 82 v1 复审）：自动对账结果列表查询参数强类型化
export interface AutoReconResultQueryParams {
  page?: number;
  page_size?: number;
  task_id?: number;
  match_status?: string;
  customer_name?: string;
  status?: string;
  start_date?: string;
  end_date?: string;
}

// P2-9c 修复（批次 82 v1 复审）：客户确认列表查询参数强类型化
export interface ConfirmationQueryParams {
  page?: number;
  page_size?: number;
  reconciliation_id?: number;
  confirm_status?: string;
}

// P2-9c 修复（批次 82 v1 复审）：争议记录列表查询参数强类型化
export interface DisputeQueryParams {
  page?: number;
  page_size?: number;
  status?: string;
  dispute_type?: string;
}

export function autoReconcile(params: {
  start_date: string;
  end_date: string;
  customer_id?: number;
}): Promise<ApiResponse<{ task_id: number; message: string }>> {
  return request.post('/ar-reconciliations-enhanced/auto-match', params);
}

export function getAutoReconciliationResults(
  params?: AutoReconResultQueryParams
): Promise<ApiResponse<PageResult<AutoReconciliationResult>>> {
  return request.get('/ar-reconciliations-enhanced/auto-match', { params });
}

/**
 * 账龄分析：后端返回单个 AgingReport 对象（非数组），图表读 overall_buckets。
 */
export function getAgingAnalysis(params?: {
  customer_id?: number;
  as_of_date?: string;
}): Promise<ApiResponse<AgingReport>> {
  return request.get('/ar-reconciliations-enhanced/aging-report', { params });
}

/**
 * 对账明细：后端返回单对象 `{ reconciliation, details }`，明细在 data.details。
 */
export function getReconciliationDetailItems(
  id: number
): Promise<ApiResponse<ReconciliationWithDetails>> {
  return request.get(`/ar-reconciliations-enhanced/${id}/details`);
}

export function sendCustomerConfirmation(id: number): Promise<ApiResponse<{ message: string }>> {
  return request.post(`/ar-reconciliations-enhanced/${id}/confirm/send`);
}

export function getCustomerConfirmations(
  params?: ConfirmationQueryParams
): Promise<ApiResponse<PageResult<CustomerConfirmation>>> {
  return request.get('/ar-reconciliations-enhanced/confirmations', { params });
}

export function updateConfirmationStatus(
  id: number,
  data: { status: 'confirmed' | 'disputed'; remark?: string }
): Promise<ApiResponse<CustomerConfirmation>> {
  return request.put(`/ar-reconciliations-enhanced/confirmations/${id}/status`, data);
}

export function createDispute(data: Partial<DisputeRecord>): Promise<ApiResponse<DisputeRecord>> {
  return request.post('/ar-reconciliations-enhanced/disputes', data);
}

export function getDisputes(
  params?: DisputeQueryParams
): Promise<ApiResponse<PageResult<DisputeRecord>>> {
  return request.get('/ar-reconciliations-enhanced/disputes', { params });
}

export function resolveDispute(
  id: number,
  data: { resolution: string }
): Promise<ApiResponse<DisputeRecord>> {
  return request.put(`/ar-reconciliations-enhanced/disputes/${id}/resolve`, data);
}

export function getDisputeDetail(id: number): Promise<ApiResponse<DisputeRecord>> {
  return request.get(`/ar-reconciliations-enhanced/disputes/${id}`);
}
