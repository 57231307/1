import { request } from './request';
import type { ApiResponse } from '@/types/api';

/**
 * 出口退税（免抵退）域 API
 *
 * 后端路由（backend/src/routes/export_refund.rs，nest 前缀 /api/v1/erp）：
 * - POST /export-refunds/customs-declarations               创建出口报关单
 * - GET  /export-refunds/sales-orders/{so_id}/documents-verification  单证齐全校验
 * - POST /export-refunds/refund-calculation                 免抵退税额计算
 * - POST /export-refunds/refund-declarations                生成退税申报表
 * - GET  /export-refunds/refund-declarations               退税申报表列表（data 为数组）
 * - GET  /export-refunds/{id}/print                        申报表 DOCX 打印
 */

/** 出口退税申报表（对齐 backend/src/models/export_refund_declaration.rs） */
export interface ExportRefundDeclaration {
  id: number;
  declaration_no: string;
  period_year: number;
  period_month: number;
  declaration_date: string;
  export_sales_amount: string | number;
  refundable_vat_amount: string | number;
  exempt_vat_amount: string | number;
  credit_vat_amount: string | number;
  actual_refund_amount: string | number;
  carryforward_amount: string | number;
  refund_rate: string | number;
  documents_complete: boolean;
  status: string;
  remarks: string | null;
  created_by: number | null;
  created_at: string;
  updated_at: string;
}

/** 创建出口报关单请求（对齐 CreateCustomsDeclarationRequest，created_by 由后端注入） */
export interface CreateCustomsDeclarationPayload {
  declaration_no: string;
  sales_order_id?: number;
  customer_id?: number;
  product_id?: number;
  export_date: string;
  destination_country?: string;
  currency_code?: string;
  total_amount: number;
  exchange_rate: number;
  customs_code?: string;
  remarks?: string;
}

/** 生成退税申报表请求（对齐 GenerateRefundDeclarationRequest） */
export interface GenerateRefundDeclarationPayload {
  period_year: number;
  period_month: number;
  refund_rate: number;
  input_vat_amount: number;
  carryforward_from_prev: number;
}

/** 免抵退税额计算输入（对齐 RefundCalculationInput） */
export interface RefundCalculationInput {
  export_sales_amount: number;
  refund_rate: number;
  input_vat_amount: number;
  carryforward_from_prev: number;
}

/** 免抵退税额计算结果（对齐 RefundCalculationResult） */
export interface RefundCalculationResult {
  refundable_vat_amount: number;
  actual_refund_amount: number;
  exempt_vat_amount: number;
  carryforward_amount: number;
}

/** 申报表状态 → 标签类型映射 */
export const refundDeclarationStatusTagMap: Record<
  string,
  'info' | 'warning' | 'success' | 'danger'
> = {
  draft: 'info',
  pending: 'warning',
  submitted: 'warning',
  approved: 'success',
  refunded: 'success',
  rejected: 'danger',
};

/** 获取退税申报表列表（后端直接返回数组） */
export function getRefundDeclarationList(params?: {
  period_year?: number;
  period_month?: number;
}): Promise<ApiResponse<ExportRefundDeclaration[]>> {
  return request.get('/export-refunds/refund-declarations', { params });
}

/** 生成退税申报表 */
export function generateRefundDeclaration(
  data: GenerateRefundDeclarationPayload
): Promise<ApiResponse<ExportRefundDeclaration>> {
  return request.post('/export-refunds/refund-declarations', data);
}

/** 创建出口报关单 */
export function createCustomsDeclaration(
  data: CreateCustomsDeclarationPayload
): Promise<ApiResponse<Record<string, unknown>>> {
  return request.post('/export-refunds/customs-declarations', data);
}

/** 校验销售订单单证齐全（报关单 + 核销单） */
export function verifyDocumentsCompleteness(
  salesOrderId: number
): Promise<ApiResponse<{ documents_complete: boolean; sales_order_id: number }>> {
  return request.get(`/export-refunds/sales-orders/${salesOrderId}/documents-verification`);
}

/** 计算免抵退税额 */
export function calculateRefund(
  data: RefundCalculationInput
): Promise<ApiResponse<RefundCalculationResult>> {
  return request.post('/export-refunds/refund-calculation', data);
}

/** 申报表打印页地址（DOCX 下载） */
export function getRefundDeclarationPrintUrl(id: number): string {
  const baseURL = import.meta.env.VITE_API_BASE_URL || '/api/v1/erp';
  return `${baseURL}/export-refunds/${id}/print`;
}
