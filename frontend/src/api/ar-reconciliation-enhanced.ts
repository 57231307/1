import { request } from './request';
import type { ApiResponse } from '@/types/api';

/**
 * 应收对账增强域列表分页信封。
 * 后端 ar_reconciliation_enhanced_handler 的 list_results / list_confirmations / list_disputes
 * 均以 `json!({"list","total","page","page_size"})` 承载列表，承载键固定为 `list`
 * （区别于全域 PaginatedResponse 的 items）。显式命名，避免万能 PageResult 掩盖信封漂移。
 */
export interface ArListPage<T> {
  list: T[];
  total: number;
  page: number;
  page_size: number;
}

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

/**
 * 争议域记录：后端 **不存在独立 dispute 实体表/DTO**。
 * create_dispute / list_disputes / get_dispute / resolve_dispute 四个 handler 都复用
 * 对账单（`ar_reconciliation::Model` / customer_dispute 返回体）作为出入参承载：
 * 争议原因写入 dispute_reason，状态即 reconciliation_status='disputed'，
 * 解决 = update_status(id, target_status, resolution→notes)。
 * 因此本类型 = 对账单模型全量键；原先的 dispute_type/dispute_amount/
 * status(open/investigating/resolved/closed)/resolution 字段组在后端无任何对应，属虚构。
 */
export interface DisputeRecord {
  id: number;
  reconciliation_no: string;
  reconciliation_date: string;
  period_start: string;
  period_end: string;
  customer_id: number;
  customer_name: string | null;
  opening_balance: string;
  total_invoices: string;
  total_collections: string;
  closing_balance: string;
  reconciliation_status: string | null;
  confirmed_by_customer: boolean | null;
  dispute_reason: string | null;
  confirmed_by: number | null;
  confirmed_at: string | null;
  created_by: number | null;
  created_at: string;
  updated_at: string;
  notes: string | null;
}

/**
 * 创建争议请求（对齐后端 CreateDisputeApiRequest）。
 * reconciliation_id 虽为 Option 反序列化字段，但 handler 显式判空并 400
 * （「reconciliation_id 不能为空」），业务上必填。
 * 后端只读 reconciliation_id / customer_id / reason / description 四个键；
 * description 优先作为争议原因写入 dispute_reason，reason 为兜底。
 */
export interface CreateDisputePayload {
  reconciliation_id: number;
  customer_id?: number;
  reason?: string;
  description?: string;
}

/**
 * 生成对账单请求（对齐后端 GenerateReconciliationApiRequest，
 * handler generate_reconciliation 由服务端按发票/收款汇总金额并在事务内生成单号）。
 * 「新增对账」界面应走本端点而非 POST /ar-reconciliations 手工创建
 * （后者要求客户端提供单号与三项金额，违反单据号后端自动生成铁律）。
 */
export interface GenerateReconciliationPayload {
  customer_id: number;
  /** YYYY-MM-DD（后端 NaiveDate） */
  start_date: string;
  /** YYYY-MM-DD（后端 NaiveDate） */
  end_date: string;
  notes?: string;
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

// 客户确认列表查询参数：字段集严格对齐后端 ListResultsQuery
// （handlers/ar_reconciliation_handler.rs::list_confirmations 的 Query<ListResultsQuery>）。
// 后端不读 reconciliation_id / confirm_status，不要传。
export interface ConfirmationQueryParams {
  page?: number;
  page_size?: number;
  customer_id?: number;
  /** YYYY-MM-DD（后端 NaiveDate） */
  start_date?: string;
  /** YYYY-MM-DD（后端 NaiveDate） */
  end_date?: string;
}

// 争议记录列表查询参数：字段集严格对齐后端 ListResultsQuery
// （handlers/ar_reconciliation_handler.rs::list_disputes 的 Query<ListResultsQuery>）。
// 后端不读 status / dispute_type（争议状态由 handler 固定为 disputed），不要传。
export interface DisputeQueryParams {
  page?: number;
  page_size?: number;
  customer_id?: number;
  /** YYYY-MM-DD（后端 NaiveDate） */
  start_date?: string;
  /** YYYY-MM-DD（后端 NaiveDate） */
  end_date?: string;
}

// 账龄分析查询参数：字段集严格对齐后端 AgingReportQueryParams
// （handlers/ar_reconciliation_handler.rs::aging_report 的 Query<T>）。
// 基准日键名是 baseline_date（NaiveDate），后端不存在 as_of_date 键。
export interface AgingAnalysisQueryParams {
  customer_id?: number;
  /** YYYY-MM-DD（后端 NaiveDate） */
  baseline_date?: string;
  salesperson_id?: number;
}

export function autoReconcile(params: {
  start_date: string;
  end_date: string;
  customer_id?: number;
}): Promise<ApiResponse<{ task_id: number; message: string }>> {
  return request.post('/ar-reconciliations-enhanced/auto-match', params);
}

export function generateReconciliation(
  data: GenerateReconciliationPayload
): Promise<ApiResponse<DisputeRecord>> {
  return request.post('/ar-reconciliations-enhanced/generate', data);
}

export function getAutoReconciliationResults(
  params?: AutoReconResultQueryParams
): Promise<ApiResponse<ArListPage<AutoReconciliationResult>>> {
  return request.get('/ar-reconciliations-enhanced/auto-match', { params });
}

/**
 * 账龄分析：后端返回单个 AgingReport 对象（非数组），图表读 overall_buckets。
 */
export function getAgingAnalysis(
  params?: AgingAnalysisQueryParams
): Promise<ApiResponse<AgingReport>> {
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
): Promise<ApiResponse<ArListPage<CustomerConfirmation>>> {
  return request.get('/ar-reconciliations-enhanced/confirmations', { params });
}

export function updateConfirmationStatus(
  id: number,
  data: { status: 'confirmed' | 'disputed'; remark?: string }
): Promise<ApiResponse<CustomerConfirmation>> {
  return request.put(`/ar-reconciliations-enhanced/confirmations/${id}/status`, data);
}

export function createDispute(data: CreateDisputePayload): Promise<ApiResponse<DisputeRecord>> {
  return request.post('/ar-reconciliations-enhanced/disputes', data);
}

export function getDisputes(
  params?: DisputeQueryParams
): Promise<ApiResponse<ArListPage<DisputeRecord>>> {
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
