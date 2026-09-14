import { request } from './request';
import {
  getVoucherList as getVoucherListApi,
  getVoucher as getVoucherApi,
  deleteVoucher as deleteVoucherApi,
  postVoucher as postVoucherApi,
} from './finance';
import type { ApiResponse, QueryParams } from '@/types/api';

export interface VoucherEntry {
  id?: number;
  account_subject_id: number;
  account_subject_code?: string;
  account_subject_name?: string;
  debit_amount: number;
  credit_amount: number;
  description?: string;
}

export interface VoucherEntity {
  id?: number;
  voucher_no: string;
  voucher_date: string;
  period_id: number;
  period_name?: string;
  type: string;
  status: string;
  description?: string;
  total_debit: number;
  total_credit: number;
  entries: VoucherEntry[];
  created_by?: number;
  created_by_name?: string;
  approved_by?: number;
  approved_by_name?: string;
  posted_by?: number;
  posted_by_name?: string;
  created_at?: string;
  approved_at?: string;
  posted_at?: string;
}

export function createVoucher(data: Partial<VoucherEntity>): Promise<ApiResponse<VoucherEntity>> {
  return request.post('/vouchers', data);
}

export function updateVoucher(
  id: number,
  data: Partial<VoucherEntity>
): Promise<ApiResponse<VoucherEntity>> {
  return request.put(`/vouchers/${id}`, data);
}

export function approveVoucher(id: number): Promise<ApiResponse<void>> {
  return request.post(`/vouchers/${id}/review`);
}

export function unpostVoucher(id: number): Promise<ApiResponse<void>> {
  return request.post(`/vouchers/${id}/unpost`);
}

export function getVoucherTypes(): Promise<ApiResponse<string[]>> {
  return request.get('/vouchers/types');
}

export function generateVoucherNo(): Promise<ApiResponse<{ voucher_no: string }>> {
  return request.get('/vouchers/generate-no');
}

// ===== 凭证基础操作：实现收敛至 finance.ts（原 voucher.ts 重复定义已删除）=====
// 以下为类型适配包装：保留本域 VoucherEntity 类型契约，调用方零破坏
export async function getVoucherList(params?: QueryParams): Promise<ApiResponse<VoucherEntity[]>> {
  const res = await getVoucherListApi(params);
  return res as unknown as ApiResponse<VoucherEntity[]>;
}

export async function getVoucher(id: number): Promise<ApiResponse<VoucherEntity>> {
  const res = await getVoucherApi(id);
  return res as unknown as ApiResponse<VoucherEntity>;
}

export function deleteVoucher(id: number): Promise<ApiResponse<void>> {
  return deleteVoucherApi(id);
}

export function postVoucher(id: number): Promise<ApiResponse<void>> {
  return postVoucherApi(id);
}
