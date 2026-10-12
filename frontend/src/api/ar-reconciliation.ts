import { request } from './request';

/**
 * 应收对账列表/详情项：与后端 handlers/ar_reconciliation_handler.rs::ReconciliationResponse 逐字段对齐
 * （list/get/create handler 均输出该 DTO；Decimal 序列化为字符串，NaiveDate 为 'YYYY-MM-DD'，
 * created_at 为 rfc3339 字符串）。
 * ⚠ 后端响应不存在 start_date/end_date/total_invoice/total_payment/total_adjustment/balance/
 *   status/customer_code/created_by_name 这些键；对账起止键名是 period_start/period_end，
 *   状态键名是 reconciliation_status（小写词表 draft/sent/confirmed/disputed/closed/cancelled，
 *   写入方 backend/src/models/status/finance.rs::ar）。前端自造键会恒为空列并让状态门控按钮不可达。
 */
export interface ArReconciliationEntity {
  id: number;
  reconciliation_no: string;
  customer_id: number;
  customer_name?: string | null;
  period_start: string;
  period_end: string;
  opening_balance: string;
  total_invoices: string;
  total_collections: string;
  closing_balance: string;
  reconciliation_status?: string | null;
  created_at: string;
}

// 列表查询参数：字段集严格对齐后端 ListReconciliationsQuery
// （handlers/ar_reconciliation_handler.rs::list_reconciliations 的 Query<T>）。
// 后端不读 keyword / customer_name（客户名筛选无实现，只能按 customer_id），不要传。
export interface ArReconciliationQueryParams {
  page?: number;
  page_size?: number;
  customer_id?: number;
  status?: string;
  /** YYYY-MM-DD（后端 NaiveDate） */
  start_date?: string;
  /** YYYY-MM-DD（后端 NaiveDate） */
  end_date?: string;
}

/**
 * 手工创建对账单请求（对齐后端 CreateReconciliationApiRequest）。
 * 除 customer_name 外全部为非 Option 必填：缺任一个即 422 missing field；
 * 起止键名是 period_start/period_end（不是 start_date/end_date）。
 * ⚠ 该端点要求调用方提供 reconciliation_no 与三项金额；界面上的「新增对账」
 * 走的是 POST /ar-reconciliations-enhanced/generate（后端汇总发票/收款自动生成），
 * 见 ar-reconciliation-enhanced.ts::generateReconciliation。
 */
export interface CreateArReconciliationPayload {
  reconciliation_no: string;
  customer_id: number;
  customer_name?: string;
  period_start: string;
  period_end: string;
  opening_balance: number;
  total_invoices: number;
  total_collections: number;
}

/**
 * 更新对账单请求（对齐后端 UpdateReconciliationApiRequest）：
 * 仅支持四项金额/备注字段，全部 Option（只传要改的键）；closing_balance 由 service 重算。
 * 客户、起止期间、状态都不走该端点（状态变更走 PUT /{id}/status）。
 */
export interface UpdateArReconciliationPayload {
  opening_balance?: number;
  total_invoices?: number;
  total_collections?: number;
  notes?: string;
}

export function getArReconciliationList(params?: ArReconciliationQueryParams) {
  return request.get('/ar-reconciliations', { params });
}

export function getArReconciliation(id: number) {
  return request.get(`/ar-reconciliations/${id}`);
}

export function createArReconciliation(data: CreateArReconciliationPayload) {
  return request.post('/ar-reconciliations', data);
}

export function updateArReconciliation(id: number, data: UpdateArReconciliationPayload) {
  return request.put(`/ar-reconciliations/${id}`, data);
}

export function deleteArReconciliation(id: number) {
  return request.delete(`/ar-reconciliations/${id}`);
}

export function confirmReconciliation(id: number) {
  return request.put(`/ar-reconciliations/${id}/status`, { status: 'confirmed' });
}
