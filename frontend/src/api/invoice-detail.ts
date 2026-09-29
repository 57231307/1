import { request } from './request';
import type { ApiResponse } from '@/types/api';

/**
 * 财务发票（finance_invoice）状态机
 * 后端状态值：pending 待审批 → approved 已审批 → verified 已核销；rejected 为驳回
 * 后端路由：routes/finance.rs finance_payment_invoice_routes（/api/v1/erp/finance/invoices）
 */
export type FinanceInvoiceStatus = 'pending' | 'approved' | 'verified' | 'rejected';

export interface FinanceInvoice {
  id: number;
  invoice_no: string;
  order_id: number | null;
  amount: string | number;
  tax_amount: string | number;
  total_amount: string | number;
  status: FinanceInvoiceStatus;
  invoice_date: string;
  paid_date: string | null;
  payment_method: string | null;
  notes: string | null;
  created_at: string;
  updated_at: string;
}

/** 列表响应为全量返回（无分页参数）：{ invoices, total } */
export interface FinanceInvoiceListResponse {
  invoices: FinanceInvoice[];
  total: number;
}

export interface CreateFinanceInvoicePayload {
  /** 发票号：必填 */
  invoice_no: string;
  /** 发票金额：必填，非负 */
  amount: number;
  /** 税额：必填，非负 */
  tax_amount: number;
  /** 价税合计：必填，非负 */
  total_amount: number;
}

export interface UpdateFinanceInvoicePayload {
  /** 发票状态：可选（驳回时传 rejected） */
  status?: string;
  /** 备注：可选 */
  notes?: string;
}

export function getFinanceInvoiceList(): Promise<ApiResponse<FinanceInvoiceListResponse>> {
  return request.get('/finance/invoices');
}

export function getFinanceInvoice(id: number): Promise<ApiResponse<FinanceInvoice>> {
  return request.get(`/finance/invoices/${id}`);
}

export function createFinanceInvoice(
  data: CreateFinanceInvoicePayload
): Promise<ApiResponse<FinanceInvoice>> {
  return request.post('/finance/invoices', data);
}

export function updateFinanceInvoice(
  id: number,
  data: UpdateFinanceInvoicePayload
): Promise<ApiResponse<FinanceInvoice>> {
  return request.put(`/finance/invoices/${id}`, data);
}

export function deleteFinanceInvoice(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/finance/invoices/${id}`);
}

/** 审批发票（仅 pending 状态可审批，pending → approved） */
export function approveFinanceInvoice(id: number): Promise<ApiResponse<FinanceInvoice>> {
  return request.post(`/finance/invoices/${id}/approve`);
}

/** 核销发票（approved → verified） */
export function verifyFinanceInvoice(id: number): Promise<ApiResponse<FinanceInvoice>> {
  return request.post(`/finance/invoices/${id}/verify`);
}
