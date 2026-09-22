import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface AccountSubject {
  id: number;
  code: string;
  name: string;
  parent_id?: number;
  level: number;
  category: string;
  direction: string;
  is_leaf: boolean;
  status: number;
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

// 字段集严格对齐后端 UpdateSubjectRequestDto：无 status / code / level，辅助核算位必填
export interface AccountSubjectUpdateRequest {
  name?: string;
  balance_direction?: string;
  assist_customer: boolean;
  assist_supplier: boolean;
  assist_batch: boolean;
  assist_color_no: boolean;
  enable_dual_unit: boolean;
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

export interface Voucher {
  id: number;
  voucher_no: string;
  voucher_date: string;
  period_id: number;
  period_name?: string;
  voucher_type: string;
  entries: VoucherEntry[];
  total_debit: number;
  total_credit: number;
  status: string;
  created_by: number;
  created_by_name?: string;
  created_at: string;
  updated_at: string;
}

export interface VoucherEntry {
  id: number;
  subject_id: number;
  subject_code: string;
  subject_name: string;
  debit: number;
  credit: number;
  summary: string;
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
  voucher_type?: string;
  status?: string;
  start_date?: string;
  end_date?: string;
  batch_no?: string;
  color_no?: string;
  page?: number;
  page_size?: number;
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
