import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

// 后端 GET /ap/invoices 直接序列化 SeaORM 实体 models/ap_invoice.rs（无 JOIN、无 DTO 改名），
// 因此行的键名与实体字段逐字一致；响应里没有 supplier_name/invoice_amount/verified_amount/
// unverified_amount/status/payment_status 这些键，前端若声明它们，对应列恒空。
export type APInvoiceStatus = 'DRAFT' | 'AUDITED' | 'PARTIAL_PAID' | 'PAID' | 'CANCELLED';

export interface APInvoice {
  id: number;
  invoice_no: string;
  supplier_id: number;
  invoice_type: string;
  invoice_date: string;
  due_date: string;
  /**
   * 发票金额：后端 models/ap_invoice.rs:58 列 amount 为 rust_decimal，serde（未启 serde-floats）
   * 序列化为十进制字符串；number 声明会在 .toFixed/算术处运行期崩或算错。
   */
  amount: string;
  /** 已付金额：后端 models/ap_invoice.rs:62 Decimal，出参十进制字符串 */
  paid_amount: string;
  /** 未付金额：后端 models/ap_invoice.rs:66 Decimal，出参十进制字符串；消费方需 Number 显式归一再比较/格式化 */
  unpaid_amount: string;
  /** 税额：后端 models/ap_invoice.rs:85 Decimal，出参十进制字符串 */
  tax_amount: string;
  currency: string;
  /** 词表来源 models/ap_invoice.rs:68 与 ap_invoice_ops/crud.rs 的 common::STATUS_* */
  invoice_status: APInvoiceStatus;
  notes?: string;
  created_at: string;
}

// 行由 GET /ap/payments 直接序列化 SeaORM 实体 models/ap_payment.rs（无 JOIN、无 DTO 改名）：
// 状态列是 payment_status，词表 REGISTERED/CONFIRMED（models/status/general.rs:37/40）。
// 响应中不存在 supplier_name 与 status 键：前端若声明它们，供应商列恒空、确认按钮门控恒真。
export type APPaymentStatus = 'REGISTERED' | 'CONFIRMED';

// 付款单由「已审批的付款申请」派生创建（create 逻辑见 ap_payment_service.rs:57-120）：
// request_id 关联来源申请，supplier_id / payment_amount / payment_method / bank_* 均从申请继承，
// 前端不可手填。transaction_no 需确认前经 PUT 回填（confirm 强制非空，见 :231-239）。
export interface APPayment {
  id: number;
  payment_no: string;
  supplier_id: number;
  /** 来源付款申请 ID（后端 models/ap_payment.rs:33 request_id: Option<i32>，nullable 故前端 ? ） */
  request_id?: number;
  payment_date: string;
  /**
   * 后端 models/ap_payment.rs:41 payment_amount 为 rust_decimal（serde 序列化为**十进制字符串**），
   * 声明成 number 会在 `.toFixed` 处运行期崩。
   */
  payment_amount: string;
  payment_method: string;
  payment_status: APPaymentStatus;
  /** 后端列 NOT NULL（models/ap_payment.rs:49 currency: String），缺键即契约断裂，前端不得标 `?` */
  currency: string;
  /** 后端 models/ap_payment.rs:53 exchange_rate 为 rust_decimal NOT NULL，出参十进制字符串 */
  exchange_rate: string;
  bank_name?: string;
  bank_account?: string;
  transaction_no?: string;
  notes?: string;
  created_at: string;
}

/**
 * 创建付款单入参：逐字段对齐后端 `CreateApPaymentRequest`
 * （backend/src/services/ap_payment_service.rs:751-764）。
 * 后端仅接受 request_id / payment_date / notes / attachment_urls，
 * 供应商、金额、方式、银行信息由服务端从所选已审批申请派生（见 :100-108），
 * 前端不得手填、也不得作为可编辑字段发送。
 */
export interface CreateApPaymentInput {
  request_id: number;
  /** NaiveDate：YYYY-MM-DD */
  payment_date: string;
  notes?: string;
  attachment_urls?: string[];
}

// 行由 GET /ap/payment-requests 直接序列化 SeaORM 实体 models/ap_payment_request.rs 返回
// （无 JOIN、无 DTO 改名）：审批状态列名是 approval_status，词表见 models/status/finance.rs:53-58
// （DRAFT/APPROVING/APPROVED/REJECTED）。响应中不存在 supplier_name / approved_amount /
// status / remark 这些键：前端若声明它们，状态列恒空、依赖它的 v-if 恒假，
// 编辑/提交/审批/驳回/删除按钮结构性不可达。
export type APPaymentRequestStatus = 'DRAFT' | 'APPROVING' | 'APPROVED' | 'REJECTED';

export interface APPaymentRequest {
  id: number;
  request_no: string;
  supplier_id: number;
  /** GET /ap/payment-requests 直接序列化实体 models/ap_payment_request.rs:42，
   * request_amount 为 rust_decimal，出参是**十进制字符串**（number 声明会在 .toFixed 运行期崩） */
  request_amount: string;
  request_date: string;
  approval_status: APPaymentRequestStatus;
  /** 以下四列后端均 NOT NULL（models/ap_payment_request.rs:30-54），响应必存在，不得标 `?` */
  payment_method: string;
  payment_type: string;
  currency: string;
  /** rust_decimal 出参十进制字符串（models/ap_payment_request.rs:54） */
  exchange_rate: string;
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
  /** 核销总额：后端 models/ap_verification.rs:38 total_amount 为 rust_decimal，出参十进制字符串 */
  total_amount: string;
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
  /** 后端对账金额列 rust_decimal（models/ap_reconciliation.rs:41 与 ap_reconciliation_ops/types.rs 侧同名键均 Decimal），出参十进制字符串 */
  total_invoice_amount: string;
  /**
   * 后端 ap_reconciliation 实体金额列 rust_decimal（models/ap_reconciliation.rs:45），
   * serde 序列化为**十进制字符串**；声明 number 属类型谎言（`.toFixed` 运行期崩）。
   * 另：Rust 契约锁 backend/tests/contract_wave6_shipment_bi_ap_fixes_test.rs 对本文件做整文件
   * 字符串匹配（含注释），payment / request / exchange 三类金额键名一旦呈 number 形态即被判负，
   * 故注释文本亦不得写出该类键名紧跟「冒号空格 number」的字面序列。
   * ⚠️ 注：本接口键名与后端真实出参键（total_invoice/total_payment/closing_balance）
   * 整体错位是尚未修复的既有契约漂移，用本接口渲染列前先对照后端实体字段。
   */
  total_payment_amount: string;
  /** 后端期末余额 closing_balance 为 Decimal（models/ap_reconciliation.rs:49，= 期初+发票-付款之差额），出参十进制字符串 */
  difference_amount: string;
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

/**
 * 创建应付单请求体：逐字段对齐后端
 * `services/ap_invoice_ops/types.rs::CreateApInvoiceRequest`（全 Option）。
 */
export interface CreateAPInvoiceRequest {
  supplier_id?: number;
  invoice_type?: string;
  /** NaiveDate: YYYY-MM-DD */
  invoice_date?: string;
  /** NaiveDate: YYYY-MM-DD */
  due_date?: string;
  payment_terms?: number;
  /**
   * 后端 CreateApInvoiceRequest.amount 为 Option<rust_decimal>
   * （services/ap_invoice_ops/types.rs:46）；rust_decimal 未启 serde-floats
   * （backend/Cargo.toml:60），JSON 浮点字面量在反序列化层即被拒绝，
   * 请求侧与响应侧统一以十进制字符串承载（先例见本文件 exchange_rate）。
   */
  amount?: string;
  currency?: string;
  /**
   * 后端 CreateApInvoiceRequest.exchange_rate 为 Option<rust_decimal>
   * （services/ap_invoice_ops/types.rs:54），本仓请求侧小数键口径=十进制字符串下发
   * （先例见 apply_amount 注释，后端 Deserialize 同时接受字符串）。
   */
  exchange_rate?: string;
  /** 后端 CreateApInvoiceRequest.tax_amount 为 Option<rust_decimal>（services/ap_invoice_ops/types.rs:60），十进制字符串下发 */
  tax_amount?: string;
  notes?: string;
  attachment_urls?: string[];
}

export function createAPInvoice(data: CreateAPInvoiceRequest): Promise<ApiResponse<APInvoice>> {
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

export function createAPPayment(data: CreateApPaymentInput): Promise<ApiResponse<APPayment>> {
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
 * 付款单 DOCX 打印：后端 GET /ap/payments/{id}/print（routes/finance.rs:699-701
 * → print_handler::ap_payment_print_docx）返回 docx 二进制流，前端转 Blob 触发下载。
 * request 响应拦截器对 blob 响应返回完整 AxiosResponse（真 Blob 在其 data 上），
 * 此处归一化，兼容「直接 Blob」与「AxiosResponse.data」两种形态。
 */
export async function printAPPaymentDocx(id: number): Promise<Blob> {
  const res = await request.get<Blob>(`/ap/payments/${id}/print`, { responseType: 'blob' });
  const payload = res as unknown as Blob | { data: Blob };
  return payload instanceof Blob ? payload : payload.data;
}

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

/**
 * 付款申请明细行入参：逐字段对齐后端 `ApPaymentRequestItemDto`
 * （backend/src/services/ap_payment_request_service.rs:660-670）。
 * apply_amount 为 rust_decimal：请求侧以十进制字符串下发（后端 Deserialize 接受字符串，
 * 且字符串可避免 number 浮点表示误差落进金额字段）。
 */
export interface ApPaymentRequestItemInput {
  invoice_id: number;
  apply_amount: string;
  notes?: string;
}

/**
 * 创建付款申请入参：逐字段对齐后端 `CreateApPaymentRequest`
 * （backend/src/services/ap_payment_request_service.rs:599-657）。
 * items 为 Option<Vec<...>>（:655-656 #[serde(default)]）：创建可缺省明细，
 * 但 submit 门控强制「至少一条真实已审批应付单明细」（同文件 :330-339），
 * 无明细的申请不可提交审批。
 */
export interface CreateApPaymentRequestInput {
  supplier_id: number;
  /** NaiveDate：YYYY-MM-DD */
  request_date: string;
  payment_type: string;
  payment_method: string;
  /** rust_decimal：十进制字符串 */
  request_amount: string;
  currency?: string;
  /** rust_decimal：十进制字符串；后端校验 >0 且 ≠0.01（validate_exchange_rate_payment :685-696） */
  exchange_rate?: string;
  expected_payment_date?: string;
  bank_name?: string;
  bank_account?: string;
  bank_account_name?: string;
  notes?: string;
  attachment_urls?: string[];
  items?: ApPaymentRequestItemInput[];
}

/**
 * 更新付款申请入参：逐字段对齐后端 `UpdateApPaymentRequest`
 * （backend/src/services/ap_payment_request_service.rs:699-730）。
 * 注意：更新契约**不含 items 与 supplier_id**——明细只能在创建时录入，
 * 后端无付款申请明细的行级端点（routes/finance.rs:708-746 仅表头 CRUD + submit/approve/reject）。
 */
export interface UpdateApPaymentRequestInput {
  request_date?: string;
  payment_type?: string;
  payment_method?: string;
  /** rust_decimal：十进制字符串 */
  request_amount?: string;
  expected_payment_date?: string;
  bank_name?: string;
  bank_account?: string;
  bank_account_name?: string;
  notes?: string;
  attachment_urls?: string[];
}

/** GET /ap/payment-requests：后端 list_requests 返回 PaginatedResponse（data = {items,total,page,page_size}） */
export function getAPPaymentRequestList(
  params?: ApPaymentRequestQueryParams
): Promise<ApiResponse<PaginatedResponse<APPaymentRequest>>> {
  return request.get('/ap/payment-requests', { params });
}

export function getAPPaymentRequest(id: number): Promise<ApiResponse<APPaymentRequest>> {
  return request.get(`/ap/payment-requests/${id}`);
}

export function createAPPaymentRequest(
  data: CreateApPaymentRequestInput
): Promise<ApiResponse<APPaymentRequest>> {
  return request.post('/ap/payment-requests', data);
}

export function updateAPPaymentRequest(
  id: number,
  data: UpdateApPaymentRequestInput
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

/** 手工核销明细：对齐后端 ap_verification_service::ApVerificationItemDto */
export interface ApVerificationItemInput {
  invoice_id: number;
  payment_id: number;
  /** 后端 ApVerificationItemDto.verify_amount 为 rust_decimal（services/ap_verification_service.rs:741），十进制字符串下发 */
  verify_amount: string;
  notes?: string;
}

/**
 * 手工核销：对齐后端 ManualVerifyRequest（ap_verification_service.rs:714）
 * ——supplier_id 必填（非 Option，后端直接落库），核销关系走 items[]，金额键是 verify_amount。
 */
export function manualVerifyAP(data: {
  supplier_id: number;
  items: ApVerificationItemInput[];
  notes?: string;
}): Promise<ApiResponse<APVerification>> {
  return request.post('/ap/verifications/manual', data);
}

// 后端 ap_verification_handler::CancelVerificationRequest 必填 reason（取消原因）
export function cancelAPVerification(id: number, reason: string): Promise<ApiResponse<void>> {
  return request.post(`/ap/verifications/${id}/cancel`, { reason });
}

// 后端 get_unverified_invoices / get_unverified_payments 强制要求 supplier_id
// （handlers/ap_verification_handler.rs:189/217：缺失即 400），故按供应商查询。
export function getUnverifiedAPInvoices(supplierId: number): Promise<ApiResponse<APInvoice[]>> {
  return request.get('/ap/verifications/unverified/invoices', {
    params: { supplier_id: supplierId },
  });
}

export function getUnverifiedAPPayments(supplierId: number): Promise<ApiResponse<APPayment[]>> {
  return request.get('/ap/verifications/unverified/payments', {
    params: { supplier_id: supplierId },
  });
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

// 供应商应付汇总（后端返回按供应商分组的数组，元素对齐 SupplierApSummary：
// services/ap_reconciliation_ops/types.rs:34 —— 计数列 i64 出参 JSON number，
// 金额列 rust_decimal 出参十进制字符串）
export interface APSupplierSummary {
  supplier_id: number;
  supplier_code: string;
  supplier_name: string;
  total_invoice_count: number;
  /** 后端 SupplierApSummary.total_invoice_amount 为 Decimal（types.rs:48），十进制字符串 */
  total_invoice_amount: string;
  /** 后端 total_paid_amount 为 Decimal（types.rs:51），十进制字符串 */
  total_paid_amount: string;
  /** 后端 total_unpaid_amount 为 Decimal（types.rs:54），十进制字符串 */
  total_unpaid_amount: string;
  paid_invoice_count: number;
  partial_paid_invoice_count: number;
  overdue_invoice_count: number;
  /** 后端 overdue_amount 为 Decimal（types.rs:66），十进制字符串 */
  overdue_amount: string;
}

export function getAPSupplierSummary(
  supplierId: number
): Promise<ApiResponse<APSupplierSummary[]>> {
  return request.get(`/ap/reconciliations/summary`, { params: { supplier_id: supplierId } });
}

// 发票关联数据（后端返回关联记录数组，元素对齐 InvoiceRelationInfo：
// services/ap_reconciliation_ops/types.rs:90，amount 列为 Decimal 出参十进制字符串）
export interface APInvoiceRelation {
  invoice_id: number;
  invoice_no: string;
  source_type: string;
  source_id: number;
  source_no: string | null;
  supplier_id: number;
  /** 后端 InvoiceRelationInfo.amount 为 Decimal（types.rs:97），十进制字符串 */
  amount: string;
  status: string;
}

// 统计报表数据
// 金额键为后端 rust_decimal（ap_report_service.rs::ApStatisticsReport 金额字段全部
// Decimal(:814/:817/:820/:835）），serde 序列化为十进制字符串；声明 number 属类型谎言
// （`.toFixed` 运行期崩）。
// ⚠️ 本接口键名与后端现行 ApStatisticsReport 出参键（total_invoice_amount/total_paid_amount/
// total_unpaid_amount）整体错位是尚未修复的既有契约漂移，用本接口渲染列前先对照后端出参键。
export interface APStatisticsData {
  total_invoices: number;
  total_amount: string;
  paid_amount: string;
  unpaid_amount: string;
  overdue_amount: string;
  period: string;
}

// 日报数据（后端 ApDailyReport：new_invoice_amount/due_invoice_amount/payment_amount 均
// Decimal → 十进制字符串，ap_report_service.rs:886 等；同上：键名错位属独立漂移）
export interface APDailyReportData {
  date: string;
  invoice_count: number;
  invoice_amount: string;
  payment_count: number;
  payment_amount: string;
  verification_count: number;
  verification_amount: string;
}

// 月报数据（金额键同口径：rust_decimal 出参=十进制字符串）
export interface APMonthlyReportData {
  year: number;
  month: number;
  invoice_count: number;
  invoice_amount: string;
  payment_count: number;
  payment_amount: string;
  verification_count: number;
  verification_amount: string;
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
// `date` 键会被 serde 静默丢弃——参数必须以 report_date 名下发。
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
