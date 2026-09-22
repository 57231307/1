import { request } from './request';
import type { ApiResponse, PaginatedResponse, QueryParams } from '@/types/api';

export interface APInvoice {
  id: number;
  invoice_no: string;
  supplier_id: number;
  supplier_name: string;
  invoice_date: string;
  invoice_amount: number;
  tax_amount: number;
  verified_amount: number;
  unverified_amount: number;
  status: string;
  payment_status: string;
  due_date?: string;
  remark?: string;
  created_at: string;
}

export interface APPayment {
  id: number;
  payment_no: string;
  supplier_id: number;
  supplier_name: string;
  payment_date: string;
  payment_amount: number;
  payment_method: string;
  status: string;
  bank_account?: string;
  remark?: string;
  created_at: string;
}

export interface APPaymentRequest {
  id: number;
  request_no: string;
  supplier_id: number;
  supplier_name: string;
  request_amount: number;
  approved_amount?: number;
  request_date: string;
  status: string;
  payment_method?: string;
  payment_type?: string;
  currency?: string;
  bank_name?: string;
  bank_account?: string;
  notes?: string;
  remark?: string;
  created_at: string;
}

export interface APVerification {
  id: number;
  verification_no: string;
  supplier_id?: number;
  verification_type?: string;
  verification_date: string;
  total_amount: number;
  verification_status?: string;
  invoice_id: number;
  invoice_no: string;
  payment_id?: number;
  payment_no?: string;
  status: string;
  notes?: string;
  created_at: string;
}

export interface APReconciliation {
  id: number;
  reconciliation_no: string;
  supplier_id: number;
  supplier_name: string;
  reconciliation_date: string;
  total_invoice_amount: number;
  total_payment_amount: number;
  difference_amount: number;
  status: string;
  confirmed_by?: string;
  confirmed_at?: string;
  created_at: string;
}

export function getAPInvoiceList(
  params?: QueryParams
): Promise<ApiResponse<PaginatedResponse<APInvoice>>> {
  return request.get('/ap/invoices', { params });
}

export function getAPInvoice(id: number): Promise<ApiResponse<APInvoice>> {
  return request.get(`/ap/invoices/${id}`);
}

export function createAPInvoice(data: Partial<APInvoice>): Promise<ApiResponse<APInvoice>> {
  return request.post('/ap/invoices', data);
}

export function updateAPInvoice(
  id: number,
  data: Partial<APInvoice>
): Promise<ApiResponse<APInvoice>> {
  return request.put(`/ap/invoices/${id}`, data);
}

export function deleteAPInvoice(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/ap/invoices/${id}`);
}

export function approveAPInvoice(id: number): Promise<ApiResponse<void>> {
  return request.post(`/ap/invoices/${id}/approve`);
}

// 后端 ap_invoice_handler::CancelInvoiceRequest 必填 reason（取消原因，用于审计留痕）
export function cancelAPInvoice(id: number, reason: string): Promise<ApiResponse<void>> {
  return request.post(`/ap/invoices/${id}/cancel`, { reason });
}

export function autoGenerateAPInvoices(data: {
  receipt_id: number;
}): Promise<ApiResponse<APInvoice>> {
  return request.post('/ap/invoices/auto-generate', data);
}

// 账龄分析项（对齐后端 ap_invoice_ops::types::AgingAnalysisItem；金额为 Decimal.to_string）
export interface APAgingItem {
  aging_bucket: string;
  invoice_count: number;
  total_amount: string;
}

/**
 * 账龄分析：后端 `ap_invoice_handler::get_aging_analysis` 返回 `Vec<AgingAnalysisItem>`（裸数组）。
 */
export function getAPAgingAnalysis(params?: {
  supplier_id?: number;
  date?: string;
}): Promise<ApiResponse<APAgingItem[]>> {
  return request.get('/ap/invoices/aging', { params });
}

/**
 * 付款列表：后端 `ap_payment_handler::list_payments` 返回
 * `PaginatedResponse<...>`（data = {items,total,page,page_size}）。
 */
export function getAPPaymentList(
  params?: QueryParams
): Promise<ApiResponse<PaginatedResponse<APPayment>>> {
  return request.get('/ap/payments', { params });
}

export function getAPPayment(id: number): Promise<ApiResponse<APPayment>> {
  return request.get(`/ap/payments/${id}`);
}

export function createAPPayment(data: Partial<APPayment>): Promise<ApiResponse<APPayment>> {
  return request.post('/ap/payments', data);
}

export function updateAPPayment(
  id: number,
  data: Partial<APPayment>
): Promise<ApiResponse<APPayment>> {
  return request.put(`/ap/payments/${id}`, data);
}

export function confirmAPPayment(id: number): Promise<ApiResponse<void>> {
  return request.post(`/ap/payments/${id}/confirm`);
}

/**
 * 付款申请列表：后端 `ap_payment_request_handler::list_requests` 返回
 * `PaginatedResponse`（data = {items,total,page,page_size}）。
 */
export function getAPPaymentRequestList(
  params?: QueryParams
): Promise<ApiResponse<PaginatedResponse<APPaymentRequest>>> {
  return request.get('/ap/payment-requests', { params });
}

export function getAPPaymentRequest(id: number): Promise<ApiResponse<APPaymentRequest>> {
  return request.get(`/ap/payment-requests/${id}`);
}

export function createAPPaymentRequest(
  data: Partial<APPaymentRequest>
): Promise<ApiResponse<APPaymentRequest>> {
  return request.post('/ap/payment-requests', data);
}

export function updateAPPaymentRequest(
  id: number,
  data: Partial<APPaymentRequest>
): Promise<ApiResponse<APPaymentRequest>> {
  return request.put(`/ap/payment-requests/${id}`, data);
}

export function deleteAPPaymentRequest(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/ap/payment-requests/${id}`);
}

export function submitAPPaymentRequest(id: number): Promise<ApiResponse<void>> {
  return request.post(`/ap/payment-requests/${id}/submit`);
}

export function approveAPPaymentRequest(id: number): Promise<ApiResponse<void>> {
  return request.post(`/ap/payment-requests/${id}/approve`);
}

export function rejectAPPaymentRequest(id: number, reason: string): Promise<ApiResponse<void>> {
  return request.post(`/ap/payment-requests/${id}/reject`, { reason });
}

/**
 * 核销列表：后端 `ap_verification_handler::list_verifications` 返回
 * `PaginatedResponse`（data = {items,total,page,page_size}）。
 */
export function getAPVerificationList(
  params?: QueryParams
): Promise<ApiResponse<PaginatedResponse<APVerification>>> {
  return request.get('/ap/verifications', { params });
}

export function getAPVerification(id: number): Promise<ApiResponse<APVerification>> {
  return request.get(`/ap/verifications/${id}`);
}

export function autoVerifyAP(data: { supplier_id: number }): Promise<ApiResponse<APVerification>> {
  return request.post('/ap/verifications/auto', data);
}

export function manualVerifyAP(data: {
  invoice_id: number;
  payment_id: number;
  amount: number;
}): Promise<ApiResponse<APVerification>> {
  return request.post('/ap/verifications/manual', data);
}

// 后端 ap_verification_handler::CancelVerificationRequest 必填 reason（取消原因）
export function cancelAPVerification(id: number, reason: string): Promise<ApiResponse<void>> {
  return request.post(`/ap/verifications/${id}/cancel`, { reason });
}

export function getUnverifiedAPInvoices(): Promise<ApiResponse<APInvoice[]>> {
  return request.get('/ap/verifications/unverified/invoices');
}

export function getUnverifiedAPPayments(): Promise<ApiResponse<APPayment[]>> {
  return request.get('/ap/verifications/unverified/payments');
}

/**
 * 对账单列表：后端 `ap_reconciliation_handler::list_reconciliations` 返回
 * `PaginatedResponse`（data = {items,total,page,page_size}）。
 */
export function getAPReconciliationList(
  params?: QueryParams
): Promise<ApiResponse<PaginatedResponse<APReconciliation>>> {
  return request.get('/ap/reconciliations', { params });
}

export function getAPReconciliation(id: number): Promise<ApiResponse<APReconciliation>> {
  return request.get(`/ap/reconciliations/${id}`);
}

export function generateAPReconciliation(data: {
  supplier_id: number;
  start_date: string;
  end_date: string;
}): Promise<ApiResponse<APReconciliation>> {
  return request.post('/ap/reconciliations/generate', data);
}

export function confirmAPReconciliation(id: number): Promise<ApiResponse<void>> {
  return request.post(`/ap/reconciliations/${id}/confirm`);
}

export function disputeAPReconciliation(id: number, reason: string): Promise<ApiResponse<void>> {
  return request.post(`/ap/reconciliations/${id}/dispute`, { reason });
}

export function autoReconcileAllAP(data: {
  start_date: string;
  end_date: string;
}): Promise<ApiResponse<void>> {
  return request.post('/ap/reconciliations/auto', data);
}

// 供应商应付汇总（后端返回按供应商分组的数组，元素对齐 SupplierApSummary）
export interface APSupplierSummary {
  supplier_id: number;
  supplier_code: string;
  supplier_name: string;
  total_invoice_count: number;
  total_invoice_amount: number;
  total_paid_amount: number;
  total_unpaid_amount: number;
  paid_invoice_count: number;
  partial_paid_invoice_count: number;
  overdue_invoice_count: number;
  overdue_amount: number;
}

export function getAPSupplierSummary(
  supplierId: number
): Promise<ApiResponse<APSupplierSummary[]>> {
  return request.get(`/ap/reconciliations/summary`, { params: { supplier_id: supplierId } });
}

// 发票关联数据（后端返回关联记录数组，元素对齐 InvoiceRelationInfo）
export interface APInvoiceRelation {
  invoice_id: number;
  invoice_no: string;
  source_type: string;
  source_id: number;
  source_no: string | null;
  supplier_id: number;
  amount: number;
  status: string;
}

// 统计报表数据
export interface APStatisticsData {
  total_invoices: number;
  total_amount: number;
  paid_amount: number;
  unpaid_amount: number;
  overdue_amount: number;
  period: string;
}

// 日报数据
export interface APDailyReportData {
  date: string;
  invoice_count: number;
  invoice_amount: number;
  payment_count: number;
  payment_amount: number;
  verification_count: number;
  verification_amount: number;
}

// 月报数据
export interface APMonthlyReportData {
  year: number;
  month: number;
  invoice_count: number;
  invoice_amount: number;
  payment_count: number;
  payment_amount: number;
  verification_count: number;
  verification_amount: number;
}

// 账龄报表数据
export interface APAgingReportData {
  current: number;
  days_30: number;
  days_60: number;
  days_90: number;
  over_90: number;
  total: number;
}

export function getAPInvoiceRelations(id: number): Promise<ApiResponse<APInvoiceRelation[]>> {
  return request.get(`/ap/invoices/${id}/relations`);
}

export function getAPStatisticsReport(
  params?: QueryParams
): Promise<ApiResponse<APStatisticsData>> {
  return request.get('/ap/reports/statistics', { params });
}

// 后端 ap_report_handler::ApDailyQueryParams 读取 report_date（必填，NaiveDate）与可选 supplier_id，
// 而非前端此前误传的 date（会被 serde 静默丢弃）。
export function getAPDailyReport(date: string): Promise<ApiResponse<APDailyReportData>> {
  return request.get('/ap/reports/daily', { params: { report_date: date } });
}

export function getAPMonthlyReport(
  year: number,
  month: number
): Promise<ApiResponse<APMonthlyReportData>> {
  return request.get('/ap/reports/monthly', { params: { year, month } });
}

export function getAPAgingReport(): Promise<ApiResponse<APAgingReportData>> {
  return request.get('/ap/reports/aging');
}
