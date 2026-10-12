import { request } from './request';

/**
 * 会计科目实体：与后端 `models/account_subject.rs` 的 `account_subject` Model 逐字段对齐
 * （GET /subjects 列表 handler 直接序列化 `Vec<account_subject::Model>`，响应键 = 模型字段名）。
 * ⚠ 后端没有 category / type / balance_type / description 列：
 *   余额方向真实列名是 balance_direction（Option，'debit'/'credit'），
 *   金额列为 sea_orm Decimal，serde 序列化为字符串。
 */
export interface AccountSubjectEntity {
  id: number;
  code: string;
  name: string;
  level: number;
  parent_id?: number | null;
  full_code?: string | null;
  // 余额方向：Option<String>，取值 'debit' / 'credit'
  balance_direction?: string | null;
  initial_balance_debit: string;
  initial_balance_credit: string;
  current_period_debit: string;
  current_period_credit: string;
  ending_balance_debit: string;
  ending_balance_credit: string;
  // 辅助核算开关（真实列）
  assist_customer: boolean;
  assist_supplier: boolean;
  assist_department: boolean;
  assist_employee: boolean;
  assist_project: boolean;
  assist_batch: boolean;
  assist_color_no: boolean;
  assist_dye_lot: boolean;
  assist_grade: boolean;
  assist_workshop: boolean;
  enable_dual_unit: boolean;
  primary_unit?: string | null;
  secondary_unit?: string | null;
  is_cash_account: boolean;
  is_bank_account: boolean;
  allow_manual_entry: boolean;
  require_summary: boolean;
  // account_subjects.status 为 VARCHAR（'active'/'inactive'）真实列，非布尔
  status: string;
  created_at: string;
  updated_at: string;
}

/**
 * 创建科目请求（对齐 handlers/account_subject_handler.rs::CreateSubjectRequestDto）。
 * code/name/level 为非 Option 且无 serde default 的必填字段，缺失即 422 missing field；
 * parent_id / balance_direction 为 Option，assist 系列 bool 与 enable_dual_unit
 * 带 #[serde(default)]，未采集时省略该键。
 */
export interface CreateSubjectPayload {
  code: string;
  name: string;
  level: number;
  parent_id?: number;
  balance_direction?: string;
  assist_customer?: boolean;
  assist_supplier?: boolean;
  assist_batch?: boolean;
  assist_color_no?: boolean;
  enable_dual_unit?: boolean;
}

/**
 * 更新科目请求（对齐 UpdateSubjectRequestDto）。
 * ⚠ assist_customer/assist_supplier/assist_batch/assist_color_no/enable_dual_unit
 * 是**非 Option bool 且无 serde default** 的必填字段：更新必须携带当前真实值，
 * 缺任何一个都会 422（missing field），不能用默认 false 掩盖（会静默翻转库中开关）。
 * name/balance_direction/status 为 Option，未填写时省略键。
 */
export interface UpdateSubjectPayload {
  name?: string;
  balance_direction?: string;
  assist_customer: boolean;
  assist_supplier: boolean;
  assist_batch: boolean;
  assist_color_no: boolean;
  enable_dual_unit: boolean;
  status?: string;
}

export function getAccountSubjectList(params?: Record<string, unknown>) {
  return request.get('/subjects', { params });
}

export function getAccountSubject(id: number) {
  return request.get(`/subjects/${id}`);
}

export function createAccountSubject(data: CreateSubjectPayload) {
  return request.post('/subjects', data);
}

export function updateAccountSubject(id: number, data: UpdateSubjectPayload) {
  return request.put(`/subjects/${id}`, data);
}

export function deleteAccountSubject(id: number) {
  return request.delete(`/subjects/${id}`);
}

export function getAccountSubjectTree() {
  return request.get('/subjects/tree');
}
