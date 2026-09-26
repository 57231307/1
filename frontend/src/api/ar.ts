import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

/**
 * 应收发票（列表/详情载荷）。
 * 后端 `ar_invoice_handler::list_ar_invoices` 直接 `to_value` 返回 SeaORM 实体
 * （无 JOIN、无 DTO 改名），字段与 `backend/src/models/ar_invoice.rs` 逐字一致：
 * 金额键为 received_amount / unpaid_amount（非 verified_amount / unverified_amount），
 * status 值集为大写 DRAFT/APPROVED/PAID/PARTIAL_PAID/CANCELLED（见 ar_invoice_service 写入点）。
 * 实体无 payment_status / remark 列，故此处不再声明。
 */
export interface ARInvoice {
  id: number;
  invoice_no: string;
  customer_id: number;
  customer_name: string;
  invoice_date: string;
  invoice_amount: number;
  tax_amount: number;
  received_amount: number;
  unpaid_amount: number;
  status: string;
  due_date?: string;
  created_at: string;
}

/**
 * 收款单列表/详情载荷（GET /ar/payments → ar_ops/json_helpers::collection_to_json）。
 * 金额键为 amount / collection_amount，两者均为 rust_decimal::Decimal.to_string() 出的字符串，
 * 响应里不存在 payment_amount 键（此前误声明 payment_amount:number ⇒ 列表金额恒显 0）。
 */
export interface ARPayment {
  id: number;
  payment_no: string;
  customer_id: number;
  customer_name: string;
  payment_date: string;
  /** 金额（后端键 amount，值为 rust_decimal 字符串，非 number、非 payment_amount） */
  amount: string;
  /** 后端 collection_to_json 同时回填的别名键，值同为 string */
  collection_amount: string;
  payment_method: string;
  status: string;
  bank_account?: string;
  remark?: string;
  created_at: string;
}

/**
 * 创建收款入参：逐字段对齐后端 `CreateArPaymentRequest`
 * （backend/src/handlers/ar_payment_handler.rs:31-43）——金额键是 amount（非 payment_amount），
 * customer_id/amount/payment_method/payment_date 必填，bank_account/remark/invoice_ids 可选。
 * amount 为 rust_decimal：serde 接受 JSON number，故 el-input-number 的 number 直接可用。
 */
export interface CreateArPaymentRequest {
  customer_id: number;
  amount: number;
  payment_method: string;
  payment_date: string;
  bank_account?: string;
  remark?: string;
  invoice_ids?: number[];
}

/** 更新收款入参：对齐后端 `UpdateArPaymentRequest`（全字段可选，无 customer_id，金额键 amount）。 */
export interface UpdateArPaymentRequest {
  amount?: number;
  payment_method?: string;
  payment_date?: string;
  bank_account?: string;
  remark?: string;
}

export interface ARVerification {
  id: number;
  verification_no: string;
  invoice_id: number;
  invoice_no: string;
  payment_id?: number;
  payment_no?: string;
  verification_amount: number;
  verification_date: string;
  status: string;
  created_at: string;
}

/**
 * 应收对账单（列表/详情载荷）。
 * 字段对齐后端 `ar_reconciliation_handler::ReconciliationResponse`
 * （backend/src/handlers/ar_reconciliation_handler.rs:39）。
 * 后端金额字段为 Decimal.to_string()，故类型为 string。
 */
export interface ARReconciliation {
  id: number;
  reconciliation_no: string;
  customer_id: number;
  customer_name: string | null;
  period_start: string;
  period_end: string;
  opening_balance: string;
  total_invoices: string;
  total_collections: string;
  closing_balance: string;
  reconciliation_status: string | null;
  created_at: string;
}

/** 创建对账单请求体，对齐后端 `CreateReconciliationApiRequest`（period/金额必填）。 */
export interface CreateARReconciliationRequest {
  reconciliation_no: string;
  customer_id: number;
  customer_name?: string | null;
  period_start: string;
  period_end: string;
  opening_balance: number;
  total_invoices: number;
  total_collections: number;
}

/**
 * 应收发票列表查询参数：对齐后端 `ar_invoice_handler::ArInvoiceQuery`
 * （backend/src/handlers/ar_invoice_handler.rs:35，list_ar_invoices 的 Query 提取器 :62）。
 * 后端无 rename_all，字段保持 snake_case。
 */
export interface ArInvoiceQuery {
  customer_id?: number;
  status?: string;
  page?: number;
  page_size?: number;
}

export function getARInvoiceList(params?: ArInvoiceQuery): Promise<ApiResponse<ARInvoice[]>> {
  return request.get('/ar/invoices', { params });
}

export function getARInvoice(id: number): Promise<ApiResponse<ARInvoice>> {
  return request.get(`/ar/invoices/${id}`);
}

export function createARInvoice(data: Partial<ARInvoice>): Promise<ApiResponse<ARInvoice>> {
  return request.post('/ar/invoices', data);
}

export function updateARInvoice(
  id: number,
  data: Partial<ARInvoice>
): Promise<ApiResponse<ARInvoice>> {
  return request.put(`/ar/invoices/${id}`, data);
}

export function deleteARInvoice(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/ar/invoices/${id}`);
}

export function approveARInvoice(id: number): Promise<ApiResponse<void>> {
  return request.post(`/ar/invoices/${id}/approve`);
}

// 后端 ar_invoice_handler::CancelReason 必填 reason（取消原因，用于审计留痕）
export function cancelARInvoice(id: number, reason: string): Promise<ApiResponse<void>> {
  return request.post(`/ar/invoices/${id}/cancel`, { reason });
}

/**
 * 对账单列表：后端 `ar_reconciliation_handler::list_reconciliations` 返回
 * `PaginatedResponse<ReconciliationResponse>`（data = {items,total,page,page_size}）。
 */
/**
 * 应收对账单列表查询参数：对齐后端 `ar_reconciliation_handler::ListReconciliationsQuery`
 * （backend/src/handlers/ar_reconciliation_handler.rs:103，list_reconciliations 的 Query 提取器 :116）。
 */
export interface ArReconciliationQuery {
  status?: string;
  customer_id?: number;
  page?: number;
  page_size?: number;
  start_date?: string;
  end_date?: string;
}

export function getARReconciliationList(
  params?: ArReconciliationQuery
): Promise<ApiResponse<PaginatedResponse<ARReconciliation>>> {
  return request.get('/ar-reconciliations', { params });
}

export function getARReconciliation(id: number): Promise<ApiResponse<ARReconciliation>> {
  return request.get(`/ar-reconciliations/${id}`);
}

export function createARReconciliation(
  data: CreateARReconciliationRequest
): Promise<ApiResponse<ARReconciliation>> {
  return request.post('/ar-reconciliations', data);
}

export function updateARReconciliationStatus(
  id: number,
  status: string
): Promise<ApiResponse<void>> {
  return request.put(`/ar-reconciliations/${id}/status`, { status });
}

/**
 * 收款列表：后端 `ar_payment_handler::list_payments` 以 json! 手搓载荷，
 * 承载数组的键为 `list`（data = {list,total,page,page_size}）。
 * 注意：与 AP 侧同类列表用 `items` 键不一致 —— 见汇报「需后端统一」。
 */
/**
 * 收款列表查询参数：对齐后端 `ar_payment_handler::ArPaymentQuery`
 * （backend/src/handlers/ar_payment_handler.rs:19，list_payments 的 Query 提取器 :61）。
 */
export interface ArPaymentQuery {
  page?: number;
  page_size?: number;
  status?: string;
  customer_id?: number;
  payment_no?: string;
}

export function getARPaymentList(
  params?: ArPaymentQuery
): Promise<ApiResponse<{ list: ARPayment[]; total: number; page: number; page_size: number }>> {
  return request.get('/ar/payments', { params });
}

export function getARPayment(id: number): Promise<ApiResponse<ARPayment>> {
  return request.get(`/ar/payments/${id}`);
}

export function createARPayment(data: CreateArPaymentRequest): Promise<ApiResponse<ARPayment>> {
  return request.post('/ar/payments', data);
}

export function updateARPayment(
  id: number,
  data: UpdateArPaymentRequest
): Promise<ApiResponse<ARPayment>> {
  return request.put(`/ar/payments/${id}`, data);
}

export function confirmARPayment(id: number): Promise<ApiResponse<void>> {
  return request.post(`/ar/payments/${id}/confirm`);
}

/**
 * 收款单 DOCX 打印：后端 GET /ar/collections/{id}/print（routes/finance.rs:893
 * → print_handler::ar_collection_print_docx）返回 docx 二进制流，前端转 Blob 触发下载。
 * request 响应拦截器对 blob 响应返回完整 AxiosResponse（真 Blob 在其 data 上），
 * 此处归一化，兼容「直接 Blob」与「AxiosResponse.data」两种形态（对齐 ap.ts printAPPaymentDocx）。
 */
export async function printARCollectionDocx(id: number): Promise<Blob> {
  const res = await request.get<Blob>(`/ar/collections/${id}/print`, { responseType: 'blob' });
  const payload = res as unknown as Blob | { data: Blob };
  return payload instanceof Blob ? payload : payload.data;
}

/**
 * 核销列表：后端 `ar_verification_handler::list_verifications` 以 json! 手搓载荷，
 * 承载数组的键为 `list`（data = {list,total,page,page_size}）。
 */
/**
 * 核销列表查询参数：对齐后端 `ar_verification_handler::ArVerificationQuery`
 * （backend/src/handlers/ar_verification_handler.rs:15，list_verifications 的 Query 提取器 :38）。
 */
export interface ArVerificationQuery {
  page?: number;
  page_size?: number;
  invoice_id?: number;
  payment_id?: number;
  status?: string;
}

export function getARVerificationList(
  params?: ArVerificationQuery
): Promise<
  ApiResponse<{ list: ARVerification[]; total: number; page: number; page_size: number }>
> {
  return request.get('/ar/verifications', { params });
}

export function getARVerification(id: number): Promise<ApiResponse<ARVerification>> {
  return request.get(`/ar/verifications/${id}`);
}

export function autoVerifyAR(): Promise<ApiResponse<unknown>> {
  return request.post('/ar/verifications/auto');
}

export function manualVerifyAR(data: {
  invoice_id: number;
  payment_id: number;
  amount: number;
}): Promise<ApiResponse<ARVerification>> {
  return request.post('/ar/verifications/manual', data);
}

export function cancelARVerification(id: number): Promise<ApiResponse<void>> {
  return request.post(`/ar/verifications/${id}/cancel`);
}

export function getUnverifiedARInvoices(): Promise<ApiResponse<ARInvoice[]>> {
  return request.get('/ar/verifications/unverified/invoices');
}

export function getUnverifiedARPayments(): Promise<ApiResponse<ARPayment[]>> {
  return request.get('/ar/verifications/unverified/payments');
}

// P2-15 修复（批次 86 v2 复审）：4 处报表 ApiResponse<any> → 显式接口

/** AR 统计报表汇总行 */
export interface ARStatisticsReport {
  total_invoice_amount: number;
  total_received_amount: number;
  total_unreceived_amount: number;
  total_verified_amount: number;
  total_unverified_amount: number;
  invoice_count: number;
  overdue_count: number;
  [key: string]: unknown;
}

/** AR 日报表行 */
export interface ARDailyReport {
  date: string;
  invoice_amount: number;
  received_amount: number;
  verified_amount: number;
  invoice_count: number;
  [key: string]: unknown;
}

/** AR 月报表行 */
export interface ARMonthlyReport {
  month: string;
  invoice_amount: number;
  received_amount: number;
  verified_amount: number;
  invoice_count: number;
  [key: string]: unknown;
}

/** AR 账龄报表行 */
export interface ARAgingReport {
  customer_id: number;
  customer_name: string;
  age_0_30: number;
  age_31_60: number;
  age_61_90: number;
  age_91_180: number;
  age_180_plus: number;
  total_amount: number;
  [key: string]: unknown;
}

/**
 * 统计报表查询参数：对齐后端 `ar_report_handler::ArReportQuery`
 * （backend/src/handlers/ar_report_handler.rs:19，get_statistics_report 的 Query 提取器 :34）。
 * 后端无 rename_all、无分页字段，全部为 Option。
 */
export interface ArReportQuery {
  start_date?: string;
  end_date?: string;
  customer_id?: number;
  baseline_date?: string;
  salesperson_id?: number;
}

export function getARStatisticsReport(
  params?: ArReportQuery
): Promise<ApiResponse<ARStatisticsReport>> {
  return request.get('/ar/reports/statistics', { params });
}

// 后端 ar_report_handler::ArReportQuery 读取 start_date/end_date/customer_id/baseline_date/
// salesperson_id，此前前端误传 date / year+month（均被 serde 静默丢弃）。
// 日报：以所选日期为单日区间 [date, date] 传给 start_date/end_date。
export function getARDailyReport(date: string): Promise<ApiResponse<ARDailyReport>> {
  return request.get('/ar/reports/daily', { params: { start_date: date, end_date: date } });
}

// 月报：将 (year, month) 换算为该月起止日期，按 start_date/end_date 传参（不再发明 year/month）。
export function getARMonthlyReport(
  year: number,
  month: number
): Promise<ApiResponse<ARMonthlyReport>> {
  const pad = (n: number) => String(n).padStart(2, '0');
  const startDate = `${year}-${pad(month)}-01`;
  const lastDay = new Date(year, month, 0).getDate();
  const endDate = `${year}-${pad(month)}-${pad(lastDay)}`;
  return request.get('/ar/reports/monthly', {
    params: { start_date: startDate, end_date: endDate },
  });
}

export function getARAgingReport(): Promise<ApiResponse<ARAgingReport>> {
  return request.get('/ar/reports/aging');
}
