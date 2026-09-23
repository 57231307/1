import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

// 后端 GET /ap/invoices 直接序列化 SeaORM 实体 models/ap_invoice.rs（无 JOIN、无 DTO 改名），
// 因此行的键名与实体字段逐字一致；此前声明的 supplier_name/invoice_amount/verified_amount/
// unverified_amount/status/payment_status 在响应里并不存在（列表这几列恒空）。
export type APInvoiceStatus = 'DRAFT' | 'AUDITED' | 'PARTIAL_PAID' | 'PAID' | 'CANCELLED';

export interface APInvoice {
  id: number;
  invoice_no: string;
  supplier_id: number;
  invoice_type: string;
  invoice_date: string;
  due_date: string;
  /** 发票金额（实体列 amount） */
  amount: number;
  /** 已付金额 */
  paid_amount: number;
  /** 未付金额 */
  unpaid_amount: number;
  tax_amount: number;
  currency: string;
  /** 词表来源 models/ap_invoice.rs:68 与 ap_invoice_ops/crud.rs 的 common::STATUS_* */
  invoice_status: APInvoiceStatus;
  notes?: string;
  created_at: string;
}

// 行由 GET /ap/payments 直接序列化 SeaORM 实体 models/ap_payment.rs（无 JOIN、无 DTO 改名）：
// 状态列是 payment_status，词表 REGISTERED/CONFIRMED（models/status/general.rs:37/40）。
// 原先声明的 supplier_name 与 status 在响应里不存在 ⇒ 供应商列恒空、确认按钮门控恒真。
export type APPaymentStatus = 'REGISTERED' | 'CONFIRMED';

export interface APPayment {
  id: number;
  payment_no: string;
  supplier_id: number;
  payment_date: string;
  payment_amount: number;
  payment_method: string;
  payment_status: APPaymentStatus;
  currency?: string;
  bank_name?: string;
  bank_account?: string;
  transaction_no?: string;
  notes?: string;
  created_at: string;
}

// 行由 GET /ap/payment-requests 直接序列化 SeaORM 实体 models/ap_payment_request.rs 返回
// （无 JOIN、无 DTO 改名）：审批状态列名是 approval_status，词表见 models/status/finance.rs:53-58
// （DRAFT/APPROVING/APPROVED/REJECTED）。此前声明的 supplier_name / approved_amount / status /
// remark 在响应中不存在 —— 状态列恒空、五处 v-if 恒假，编辑/提交/审批/驳回/删除按钮结构性不可达。
export type APPaymentRequestStatus = 'DRAFT' | 'APPROVING' | 'APPROVED' | 'REJECTED';

export interface APPaymentRequest {
  id: number;
  request_no: string;
  supplier_id: number;
  request_amount: number;
  request_date: string;
  approval_status: APPaymentRequestStatus;
  payment_method?: string;
  payment_type?: string;
  currency?: string;
  expected_payment_date?: string;
  bank_name?: string;
  bank_account?: string;
  notes?: string;
  created_at: string;
}

// GET /ap/verifications 直接序列化实体 models/ap_verification.rs（主表无发票/付款单号，
// 明细在 ap_verification_item）；状态列 verification_status 词表 COMPLETED/CANCELLED
// （models/status/general.rs:25/28，服务写入见 ap_verification_service.rs:200/450/578）。
export type APVerificationStatus = 'COMPLETED' | 'CANCELLED';

export interface APVerification {
  id: number;
  verification_no: string;
  supplier_id: number;
  verification_type?: string;
  verification_date: string;
  total_amount: number;
  verification_status: APVerificationStatus;
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

/**
 * 应付发票列表查询参数：对齐后端 `ap_invoice_handler::ApInvoiceQueryParams`
 * （backend/src/handlers/ap_invoice_handler.rs:29，list_ap_invoices 的 Query 提取器 :41）。
 * 后端无 `#[serde(rename_all)]`，字段保持 snake_case；start_date/end_date 为 NaiveDate（"YYYY-MM-DD"）。
 */
export interface ApInvoiceQueryParams {
  supplier_id?: number;
  invoice_status?: string;
  invoice_type?: string;
  start_date?: string;
  end_date?: string;
  page?: number;
  page_size?: number;
}

export function getAPInvoiceList(
  params?: ApInvoiceQueryParams
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
/**
 * 付款列表查询参数：对齐后端 `ap_payment_handler::ApPaymentQueryParams`
 * （backend/src/handlers/ap_payment_handler.rs:25，list_payments 的 Query 提取器 :37）。
 */
export interface ApPaymentQueryParams {
  supplier_id?: number;
  payment_status?: string;
  payment_method?: string;
  start_date?: string;
  end_date?: string;
  page?: number;
  page_size?: number;
}

export function getAPPaymentList(
  params?: ApPaymentQueryParams
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
/**
 * 付款申请列表查询参数：对齐后端 `ap_payment_request_handler::ApPaymentRequestQueryParams`
 * （backend/src/handlers/ap_payment_request_handler.rs:28，list_requests 的 Query 提取器 :40）。
 */
export interface ApPaymentRequestQueryParams {
  supplier_id?: number;
  approval_status?: string;
  payment_type?: string;
  start_date?: string;
  end_date?: string;
  page?: number;
  page_size?: number;
}

export function getAPPaymentRequestList(
  params?: ApPaymentRequestQueryParams
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
/**
 * 核销列表查询参数：对齐后端 `ap_verification_handler::ApVerificationQueryParams`
 * （backend/src/handlers/ap_verification_handler.rs:23，list_verifications 的 Query 提取器 :34）。
 */
export interface ApVerificationQueryParams {
  supplier_id?: number;
  verification_type?: string;
  start_date?: string;
  end_date?: string;
  page?: number;
  page_size?: number;
}

export function getAPVerificationList(
  params?: ApVerificationQueryParams
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
/**
 * 对账单列表查询参数：对齐后端 `ap_reconciliation_handler::ApReconciliationQueryParams`
 * （backend/src/handlers/ap_reconciliation_handler.rs:25，list_reconciliations 的 Query 提取器 :36）。
 */
export interface ApReconciliationQueryParams {
  supplier_id?: number;
  reconciliation_status?: string;
  start_date?: string;
  end_date?: string;
  page?: number;
  page_size?: number;
}

export function getAPReconciliationList(
  params?: ApReconciliationQueryParams
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

/**
 * 统计报表查询参数：对齐后端 `ap_report_handler::ApStatisticsQueryParams`
 * （backend/src/handlers/ap_report_handler.rs:25，get_statistics_report 的 Query 提取器 :34）。
 * start_date/end_date 后端为非 Option 的 NaiveDate（必填），TS 侧同样必填；无分页字段。
 */
export interface ApStatisticsQueryParams {
  supplier_id?: number;
  start_date: string;
  end_date: string;
}

export function getAPStatisticsReport(
  params?: ApStatisticsQueryParams
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
