import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

/**
 * 应收发票（列表/详情载荷）。
 * 后端 `ar_invoice_handler::list_ar_invoices` 返回标准 `PaginatedResponse<ar_invoice::Model>`
 * （data = {items,total,page,page_size}），items 为 SeaORM 实体（无 JOIN、无 DTO 改名），
 * 字段与 `backend/src/models/ar_invoice.rs` 逐字一致：
 * 金额键为 received_amount / unpaid_amount（非 verified_amount / unverified_amount），
 * status 值集为大写 DRAFT/APPROVED/PAID/PARTIAL_PAID/CANCELLED（见 ar_invoice_service 写入点）。
 * 实体无 payment_status / remark 列，故此处不再声明。
 * 金额字段（invoice_amount/received_amount/unpaid_amount/tax_amount）为 rust_decimal：
 * 后端依赖 `rust_decimal` 仅启用 `serde` feature（Cargo.toml，非 serde-float），
 * 经 `serde_json::to_value` 序列化为 JSON **字符串**（如 "1234.56"），故类型为 string；
 * tax_amount 后端为 Option<Decimal> ⇒ string | null。展示走 utils 侧 decimal-string 安全范式。
 */
export interface ARInvoice {
  id: number;
  invoice_no: string;
  customer_id: number;
  customer_name: string;
  invoice_date: string;
  invoice_amount: string;
  tax_amount: string | null;
  received_amount: string;
  unpaid_amount: string;
  status: string;
  // 后端 models/ar_invoice.rs:17 due_date 为 NaiveDate(NOT NULL)，响应必带 → 必填 string
  due_date: string;
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

/**
 * 更新收款入参：对齐后端 `UpdateArPaymentRequest`
 * （backend/src/handlers/ar_payment_handler.rs，双层 Option 三态：键缺席=保持原值、
 * 显式 null=清空[仅可空列]、有值=覆盖；无 customer_id，身份/审计字段不经请求体）。
 * amount/payment_date 对应 NOT NULL 列（collection_amount/collection_date），
 * 后端更新链路已真实支持修改：金额执行与创建同源的 >0/精度≤2 位校验与
 * "新金额≥已核销分配金额"一致性门（ar_ops/collection.rs::update_payment），
 * 日期执行所属期间关账检查；二者显式 null 被业务拒绝，故此处声明为可选但不可传 null。
 * amount 入站为 JSON number（rust_decimal serde 接受），与出参 string 形态解耦。
 */
export interface UpdateArPaymentRequest {
  /** 收款金额：NOT NULL 列，编辑对话框必送；不允许 null（后端业务拒绝） */
  amount?: number;
  /** 收款日期（YYYY-MM-DD）：NOT NULL 列，编辑对话框必送；不允许 null */
  payment_date?: string;
  /** 收款方式（可空列，后端另支持显式 null 清空） */
  payment_method?: string;
  /** 银行账号（可空列） */
  bank_account?: string;
  /** 备注（可空列；显式 null=清空为 NULL） */
  remark?: string | null;
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

/**
 * 应收发票列表：后端 `ar_invoice_handler::list_ar_invoices` 返回标准
 * `ApiResponse<PaginatedResponse<ARInvoice>>`（data = {items,total,page,page_size}）。
 * items 为 ar_invoice::Model 数组，金额字段为 rust_decimal 序列化的 JSON 字符串（见 ARInvoice 注释）。
 */
export function getARInvoiceList(
  params?: ArInvoiceQuery
): Promise<ApiResponse<PaginatedResponse<ARInvoice>>> {
  return request.get('/ar/invoices', { params });
}

export function getARInvoice(id: number): Promise<ApiResponse<ARInvoice>> {
  return request.get(`/ar/invoices/${id}`);
}

/**
 * 创建应收发票入参：对齐后端 `CreateArInvoiceRequestDto`（backend/src/handlers/ar_invoice_handler.rs:46），
 * 全部字段 Option。金额键 invoice_amount 为 `Option<Decimal>`：serde 接受 JSON number，
 * 故 el-input-number 的 number 可直接提交；请求侧金额是 number（与响应侧 ARInvoice 的 string 解耦，
 * 因后端入参/出参对 Decimal 的 serde 处理方向不同）。invoice_no 由后端自生成，不入参。
 */
export interface CreateARInvoiceRequest {
  customer_id?: number;
  invoice_date?: string;
  due_date?: string;
  customer_name?: string;
  source_type?: string;
  source_bill_id?: number;
  source_bill_no?: string;
  invoice_amount?: number;
  batch_no?: string;
  color_no?: string;
  sales_order_no?: string;
}

/**
 * 更新应收发票入参：对齐后端 `UpdateArInvoiceRequest`
 * （backend/src/services/ar_invoice_service.rs:26），全字段可选，金额键 invoice_amount（number）。
 */
export interface UpdateARInvoiceRequest {
  invoice_date?: string;
  due_date?: string;
  invoice_amount?: number;
}

export function createARInvoice(data: CreateARInvoiceRequest): Promise<ApiResponse<ARInvoice>> {
  return request.post('/ar/invoices', data);
}

export function updateARInvoice(
  id: number,
  data: UpdateARInvoiceRequest
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

/**
 * AR 统计报表聚合行（单聚合对象载荷，非行集）。
 * 出参形状以定稿 DTO 为唯一事实：键与 `backend/src/services/ar_ops/report.rs:141-149`
 * `build_statistics_response` 的 `json!` 字面逐字一致；handler `get_statistics_report`
 * （backend/src/handlers/ar_report_handler.rs:60）将 service 构造原样放入 ApiResponse 载荷，
 * 无二次加工。金额键（total_amount/paid_amount/unpaid_amount/overdue_amount）为后端
 * `Decimal.to_string()` 出的字符串（rust_decimal 仅启用 serde feature，非 float 序列化，
 * 本仓裁定出参为字符串），计数键 total_invoices/overdue_count 为 JSON number，
 * collection_rate 为 f64 回款率。
 * `[key: string]: unknown` 索引签名仅供报表 tab 通用表格从载荷推导列（列 = Object.keys），
 * 不代表后端另有出参键；真实键由后端契约形状锁逐键钉死，前端侧由
 * `tests/unit/ar-report-keys.test.ts` 三向对锁（后端源码 ↔ 本声明 ↔ 钉死清单）。
 */
export interface ARStatisticsReport {
  total_invoices: number;
  total_amount: string;
  paid_amount: string;
  unpaid_amount: string;
  overdue_count: number;
  overdue_amount: string;
  collection_rate: number;
  [key: string]: unknown;
}

/**
 * AR 日报表行：出参形状以后端定稿 DTO 为唯一事实
 * （backend/src/services/ar_ops/report.rs:181-187，按 invoice_date GROUP BY 的聚合行）。
 * 金额为后端 Decimal.to_string() 的字符串，非 number。
 */
export interface ARDailyReport {
  date: string;
  invoice_count: number;
  invoice_amount: string;
  paid_amount: string;
  unpaid_amount: string;
  [key: string]: unknown;
}

/**
 * AR 月报表行：同后端 report.rs:271-277（按 to_char 'YYYY-MM' GROUP BY 的聚合行），
 * 金额同为字符串。
 */
export interface ARMonthlyReport {
  month: string;
  invoice_count: number;
  invoice_amount: string;
  paid_amount: string;
  unpaid_amount: string;
  [key: string]: unknown;
}

/**
 * AR 账龄报表聚合行（单聚合对象载荷，全体/筛选口径下的一组合计，非按客户行集）。
 * 出参形状以定稿 DTO 为唯一事实：键与 `backend/src/services/ar_ops/report.rs:498-506`
 * `build_aging_response` 的 `json!` 字面逐字一致；handler `get_aging_report`
 * （backend/src/handlers/ar_report_handler.rs:168）将 service 构造原样放入 ApiResponse 载荷。
 * 分桶由后端 SQL CASE WHEN 按 due_date 计算（未到期 not_due + 逾期 0-30/31-60/61-90/90+），
 * total_overdue 为四桶逾期合计；全部金额键为 `Decimal.to_string()` 字符串（本仓裁定），
 * invoice_count 为 JSON number。按业务员维度的行集走独立端点
 * `/ar/reports/aging/by-salesperson`，与本类型无关。
 * 索引签名同 ARStatisticsReport 的说明。
 */
export interface ARAgingReport {
  not_due: string;
  bucket_0_30: string;
  bucket_31_60: string;
  bucket_61_90: string;
  bucket_90_plus: string;
  total_overdue: string;
  invoice_count: number;
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
// salesperson_id。日报：以所选日期为单日区间 [date, date] 传给 start_date/end_date。
// 载荷为裸数组行集（backend/src/services/ar_ops/report.rs:191 Ok(json!(Vec))）。
export function getARDailyReport(date: string): Promise<ApiResponse<ARDailyReport[]>> {
  return request.get('/ar/reports/daily', { params: { start_date: date, end_date: date } });
}

// 月报：将 (year, month) 换算为该月起止日期，按 start_date/end_date 传参（不发明 year/month 参数）。
// 载荷为裸数组行集（backend/src/services/ar_ops/report.rs:281 Ok(json!(Vec))）。
export function getARMonthlyReport(
  year: number,
  month: number
): Promise<ApiResponse<ARMonthlyReport[]>> {
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
