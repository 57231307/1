import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface AccountSubject {
  id: number;
  code: string;
  name: string;
  parent_id?: number;
  level: number;
  category: string;
  /**
   * 余额方向真实出参键为 balance_direction（models/account_subject.rs:22，Option<String>；
   * DDL VARCHAR(20) DEFAULT 'debit'、可空、无 CHECK——m0006_add_general_ledger_and_finance_base.rs:23）。
   * 后端从不输出 direction 键——此前声明 direction 导致列表恒判「贷」、编辑回填恒 debit 并回写覆盖真实值。
   * 取值现状（四值并存，展示须同源）：前端 radio 与 DB 默认写 'debit'/'credit'；
   * 后端余额计算按中文 '借' 比较（account_subject_service.rs:351/454），e2e 直灌 '借'/'贷'。
   * 键存在性随端点而变：GET /subjects、GET /subjects/:id 直出 Model（键恒在，值可为 null）；
   * GET /subjects/tree（SubjectTreeNode，service:494-502）不输出该列——故声明为可选，
   * 列表数据源已改用 GET /subjects 以拿到真实值。
   */
  balance_direction?: string | null;
  // 注：后端 /subjects/tree(SubjectTreeNode) 与 account_subject 模型均**不输出 is_leaf**，
  // 叶子与否由 children 是否为空派生，故此处不声明 is_leaf（声明恒缺值的字段=谎报契约）。
  // account_subjects.status 为 VARCHAR（'active'/'inactive'），非数字
  status: string;
  created_at: string;
  updated_at: string;
  children?: AccountSubject[];
}

// 字段集严格对齐后端 CreateSubjectRequestDto（handlers/account_subject_handler.rs）
// direction 语义映射为 balance_direction；辅助核算位为科目属性，后端非 Option 必填。
export interface AccountSubjectCreateRequest {
  code: string;
  name: string;
  level: number;
  parent_id?: number;
  balance_direction?: string;
  assist_customer: boolean;
  assist_supplier: boolean;
  assist_batch: boolean;
  assist_color_no: boolean;
  enable_dual_unit: boolean;
}

// 字段集严格对齐后端 UpdateSubjectRequestDto：无 code / level；
// status 可选（'active'/'inactive'，编辑对话框启用/停用开关）；辅助核算位必填
export interface AccountSubjectUpdateRequest {
  name?: string;
  balance_direction?: string;
  assist_customer: boolean;
  assist_supplier: boolean;
  assist_batch: boolean;
  assist_color_no: boolean;
  enable_dual_unit: boolean;
  status?: string;
}

// 科目列表查询参数：字段集严格对齐后端 SubjectQuery（handlers/account_subject_handler.rs）。
// 注意：该端点后端无 page/page_size（全量返回），不要传分页键。
export interface SubjectListQuery {
  level?: number;
  parent_id?: number;
  status?: string;
  keyword?: string;
}

export function getSubjectList(params?: SubjectListQuery): Promise<ApiResponse<AccountSubject[]>> {
  return request.get('/subjects', { params });
}

export function getSubjectTree(): Promise<ApiResponse<AccountSubject[]>> {
  return request.get('/subjects/tree');
}

export function getSubject(id: number): Promise<ApiResponse<AccountSubject>> {
  return request.get(`/subjects/${id}`);
}

export function createSubject(
  data: AccountSubjectCreateRequest
): Promise<ApiResponse<AccountSubject>> {
  return request.post('/subjects', data);
}

export function updateSubject(
  id: number,
  data: AccountSubjectUpdateRequest
): Promise<ApiResponse<AccountSubject>> {
  return request.put(`/subjects/${id}`, data);
}

export function deleteSubject(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/subjects/${id}`);
}

/**
 * 凭证响应模型（P0 契约修复，本轮）：
 * - 后端 GET /vouchers 列表返回 Vec<voucher::Model>（models/voucher.rs）：
 *   从不返回 period_id/period_name/total_debit/total_credit/created_by_name（vouchers 表无这些列，
 *   handlers/voucher_handler.rs list_vouchers 无 JOIN/聚合）。原类型将其标必填 = 假契约，
 *   视图读取恒 undefined。现改为可选并注释「后端当前不返回，待迁移/聚合富化」。
 * - entries 仅 GET /vouchers/:id 详情（VoucherDetailResponse.entries）携带，列表行无 ⇒ 可选。
 * - 分录 debit/credit/金额类为 rust_decimal ⇒ JSON **字符串**（VoucherItemResponseDto）。
 */
export interface Voucher {
  id: number;
  voucher_no: string;
  voucher_date: string;
  voucher_type: string;
  status: string;
  batch_no?: string | null;
  color_no?: string | null;
  created_by: number;
  reviewed_by?: number | null;
  reviewed_at?: string | null;
  posted_by?: number | null;
  posted_at?: string | null;
  created_at: string;
  updated_at: string;
  /** 后端当前不返回（vouchers 无该列）：历史前端自创键，保留可选仅为过渡期兼容 */
  period_id?: number;
  period_name?: string;
  /** 后端列表/详情均不返回合计列；仅前端由 entries 派生后填充（见 useVchr.viewVoucher） */
  total_debit?: number;
  total_credit?: number;
  created_by_name?: string;
  /** 仅详情端点返回 */
  entries?: VoucherEntry[];
}

/**
 * 凭证分录出参（后端 VoucherItemResponseDto 双命名之一族，finance 规范键）。
 * debit/credit/quantity/unit_price 为 Decimal → string；subject_id 可空（DB 列 Option）。
 */
export interface VoucherEntry {
  id: number;
  line_no?: number;
  subject_id: number | null;
  subject_code: string;
  subject_name: string;
  debit: string;
  credit: string;
  summary: string | null;
}

// 凭证分录请求行：字段对齐后端 VoucherItemDto（handlers/voucher_handler.rs）
// 仅保留本视图收集且后端读取的字段，避免"填了被 serde 丢弃"。
export interface VoucherEntryRequest {
  subject_id: number;
  debit: number;
  credit: number;
  summary: string;
}

// 后端 CreateVoucherRequestDto 的分录键是 items，不是 entries
export interface VoucherCreateRequest {
  voucher_date: string;
  voucher_type: string;
  items: VoucherEntryRequest[];
}

// 后端 UpdateVoucherRequestDto：items 为可选，字段名同样是 items
export interface VoucherUpdateRequest {
  voucher_date?: string;
  voucher_type?: string;
  items?: VoucherEntryRequest[];
}

// 凭证列表查询参数：字段集严格对齐后端 VoucherQuery（handlers/voucher_handler.rs）。
// 后端无 keyword/order_by/order_dir/supplier_name/customer_name 等通用键，不要传。
export interface VoucherListQuery {
  voucher_no?: string;
  voucher_type?: string;
  status?: string;
  start_date?: string;
  end_date?: string;
  batch_no?: string;
  color_no?: string;
  page?: number;
  page_size?: number;
}

/** 后端 VoucherTypeDefinition（available_voucher_types 单一词表源：code=记/收/付/转） */
export interface VoucherTypeDefinition {
  code: string;
  name: string;
}

/** GET /vouchers/types —— 凭证类型下拉选项（前端不得再硬编码第二套类型常量） */
export function getVoucherTypesApi(): Promise<ApiResponse<VoucherTypeDefinition[]>> {
  return request.get('/vouchers/types');
}

export function getVoucherList(params?: VoucherListQuery): Promise<ApiResponse<Voucher[]>> {
  return request.get('/vouchers', { params });
}

export function getVoucher(id: number): Promise<ApiResponse<Voucher>> {
  return request.get(`/vouchers/${id}`);
}

export function createVoucher(data: VoucherCreateRequest): Promise<ApiResponse<Voucher>> {
  return request.post('/vouchers', data);
}

export function submitVoucher(id: number): Promise<ApiResponse<void>> {
  return request.post(`/vouchers/${id}/submit`);
}

export function reviewVoucher(id: number): Promise<ApiResponse<void>> {
  return request.post(`/vouchers/${id}/review`);
}

export function postVoucher(id: number): Promise<ApiResponse<void>> {
  return request.post(`/vouchers/${id}/post`);
}

export function updateVoucher(
  id: number,
  data: VoucherUpdateRequest
): Promise<ApiResponse<Voucher>> {
  return request.put(`/vouchers/${id}`, data);
}

export function deleteVoucher(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/vouchers/${id}`);
}
