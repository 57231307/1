import { request } from './request';
import {
  getVoucherList as getVoucherListApi,
  getVoucher as getVoucherApi,
  deleteVoucher as deleteVoucherApi,
  submitVoucher as submitVoucherApi,
  postVoucher as postVoucherApi,
  type VoucherListQuery,
} from './finance';
import type { ApiResponse } from '@/types/api';

/**
 * 凭证详情分录出参（后端 VoucherItemResponseDto，handlers/voucher_handler.rs）。
 * 后端为兼容 finance.ts/voucher.ts 两套历史命名同时下发双键
 * （subject_id/account_subject_id、debit/debit_amount、summary/description），
 * rust_decimal::Decimal 序列化为**字符串**，绑定数值控件前需 Number() 归一。
 */
export interface VoucherEntry {
  id: number;
  voucher_id: number;
  line_no: number;
  account_subject_id: number | null;
  account_subject_code: string;
  account_subject_name: string;
  debit_amount: string;
  credit_amount: string;
  description: string | null;
}

/**
 * 凭证列表行 / 详情顶层 = 后端 voucher::Model（models/voucher.rs 全字段核对，snake_case）。
 *
 * P0 契约修复（本轮）：
 * - 原前端自创键 type / description / period_id / period_name / total_debit / total_credit /
 *   approved_by / approved_by_name / posted_by_name / approved_at 后端从不返回
 *   （凭证类型真实键为 voucher_type；审核列为 reviewed_by/reviewed_at；列表/详情均无合计与摘要列）。
 * - entries 仅详情端点（GET /vouchers/:id 的 VoucherDetailResponse.entries）携带，列表行无。
 */
export interface VoucherEntity {
  id?: number;
  voucher_no: string;
  voucher_type: string;
  voucher_date: string;
  status: string;
  source_type?: string | null;
  source_module?: string | null;
  source_bill_id?: number | null;
  source_bill_no?: string | null;
  batch_no?: string | null;
  color_no?: string | null;
  dye_lot_no?: string | null;
  workshop?: string | null;
  production_order_no?: string | null;
  quantity_meters?: string | null;
  quantity_kg?: string | null;
  gram_weight?: string | null;
  attachment_count?: number;
  created_by?: number;
  reviewed_by?: number | null;
  reviewed_at?: string | null;
  posted_by?: number | null;
  posted_at?: string | null;
  created_at?: string;
  updated_at?: string;
  /** 仅 GET /vouchers/:id 详情返回（后端键名 entries，双命名分录） */
  entries?: VoucherEntry[];
}

/**
 * 凭证分录请求行：对齐后端 VoucherItemDto 规范键（handlers/voucher_handler.rs:64，
 * alias 仅为兼容旧契约，新代码按规范键提交）。debit/credit 为 DTO 非 Option ⇒ 必填。
 */
export interface VoucherItemPayload {
  line_no?: number;
  subject_id?: number | null;
  subject_code?: string | null;
  subject_name?: string | null;
  debit: number;
  credit: number;
  summary?: string | null;
}

/** 创建凭证请求体：对齐后端 CreateVoucherRequestDto（voucher_type/items 必填） */
export interface CreateVoucherPayload {
  voucher_type: string;
  voucher_date: string;
  source_type?: string;
  source_module?: string;
  source_bill_id?: number;
  source_bill_no?: string;
  batch_no?: string;
  color_no?: string;
  items: VoucherItemPayload[];
}

/** 更新凭证请求体：对齐后端 UpdateVoucherRequestDto（PATCH：Some=覆盖；items=整表替换分录） */
export interface UpdateVoucherPayload {
  voucher_type?: string;
  voucher_date?: string;
  items?: VoucherItemPayload[];
}

/** 后端 VoucherTypeDefinition（available_voucher_types 静态词表：code=记/收/付/转） */
export interface VoucherTypeDefinition {
  code: string;
  name: string;
}

export function createVoucher(data: CreateVoucherPayload): Promise<ApiResponse<VoucherEntity>> {
  return request.post('/vouchers', data);
}

export function updateVoucher(
  id: number,
  data: UpdateVoucherPayload
): Promise<ApiResponse<VoucherEntity>> {
  return request.put(`/vouchers/${id}`, data);
}

/**
 * 提交凭证（状态机入口 draft→submitted）。
 * 后端路由 POST /vouchers/{id}/submit（routes/finance.rs:226 → voucher_handler::submit_voucher
 * → voucher_ops/workflow.rs::submit，门=VOUCHER_DRAFT）；实现收敛至 finance.ts。
 */
export function submitVoucher(id: number): Promise<ApiResponse<void>> {
  return submitVoucherApi(id);
}

export function approveVoucher(id: number): Promise<ApiResponse<void>> {
  return request.post(`/vouchers/${id}/review`);
}

export function unpostVoucher(id: number): Promise<ApiResponse<void>> {
  return request.post(`/vouchers/${id}/unpost`);
}

export function getVoucherTypes(): Promise<ApiResponse<VoucherTypeDefinition[]>> {
  return request.get('/vouchers/types');
}

export function generateVoucherNo(): Promise<ApiResponse<{ voucher_no: string }>> {
  return request.get('/vouchers/generate-no');
}

// ===== 凭证基础操作：实现收敛至 finance.ts（原 voucher.ts 重复定义已删除）=====
// 以下为类型适配包装：保留本域 VoucherEntity 类型契约，调用方零破坏
export async function getVoucherList(
  params?: VoucherListQuery
): Promise<ApiResponse<VoucherEntity[]>> {
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
