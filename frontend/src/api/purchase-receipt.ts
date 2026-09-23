import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface PurchaseReceiptEntity {
  id?: number;
  receipt_no: string;
  receipt_date: string;
  purchase_order_id?: number;
  purchase_order_no?: string;
  supplier_id: number;
  supplier_name?: string;
  warehouse_id: number;
  warehouse_name?: string;
  status: string;
  total_amount: number;
  remark?: string;
  created_at?: string;
  created_by?: number;
  created_by_name?: string;
  approved_at?: string;
  approved_by?: number;
  approved_by_name?: string;
  items?: ReceiptItem[];
}

/**
 * 入库明细行：字段名与后端 purchase_receipt_item Model /
 * CreateReceiptItemRequest 对齐（material_code/material_name/unit_master/
 * quantity_alt/unit_price/notes）；编辑回显直接消费后端返回，不再另起别名
 */
export interface ReceiptItem {
  id?: number;
  receipt_id?: number;
  line_no?: number;
  /** 产品（物料）ID；提交明细时按 CreateReceiptItemRequest 映射为 material_id */
  product_id: number;
  material_code?: string;
  material_name?: string;
  batch_no?: string;
  color_code?: string;
  lot_no?: string;
  grade?: string;
  gram_weight?: number;
  width?: number;
  /** 入库数量（主单位） */
  quantity: number;
  /** 入库数量（辅助单位，面料行业常为公斤） */
  quantity_alt?: number;
  /** 主单位（取自产品档案，不手工录入） */
  unit_master?: string;
  unit_alt?: string;
  unit_price?: number;
  amount?: number;
  location_code?: string;
  notes?: string;
}

// P2-9c 修复（批次 82 v1 复审）：PurchaseReceiptQueryParams 已在 purchase.ts 定义，此处复用避免重复导出
import type { PurchaseReceiptQueryParams } from './purchase';
export type { PurchaseReceiptQueryParams };

export function getPurchaseReceiptList(params?: PurchaseReceiptQueryParams) {
  return request.get<ApiResponse<{ items: PurchaseReceiptEntity[]; total: number }>>(
    '/purchase/receipts',
    { params }
  );
}

export function getPurchaseReceipt(id: number) {
  return request.get<ApiResponse<PurchaseReceiptEntity>>(`/purchase/receipts/${id}`);
}

export function createPurchaseReceipt(data: Partial<PurchaseReceiptEntity>) {
  return request.post<ApiResponse<PurchaseReceiptEntity>>('/purchase/receipts', data);
}

export function updatePurchaseReceipt(id: number, data: Partial<PurchaseReceiptEntity>) {
  return request.put<ApiResponse<PurchaseReceiptEntity>>(`/purchase/receipts/${id}`, data);
}

export function deletePurchaseReceipt(id: number) {
  return request.delete<ApiResponse<void>>(`/purchase/receipts/${id}`);
}

// 后端真实端点：POST /purchase/receipts/{id}/confirm（purchase_receipt_handler::confirm_receipt）
export function approvePurchaseReceipt(id: number) {
  return request.post<ApiResponse<PurchaseReceiptEntity>>(`/purchase/receipts/${id}/confirm`);
}

/**
 * 获取入库单明细
 * 后端 purchase_receipt_handler::list_receipt_items 直接返回 ApiResponse<Vec<Value>>，
 * data 即明细数组本身（裸数组，非 {items} 信封）。
 */
export function getReceiptItems(id: number) {
  return request.get<ApiResponse<ReceiptItem[]>>(`/purchase/receipts/${id}/items`);
}

export function createReceiptItem(id: number, data: Partial<ReceiptItem>) {
  return request.post<ApiResponse<ReceiptItem>>(`/purchase/receipts/${id}/items`, data);
}

export function updateReceiptItem(id: number, itemId: number, data: Partial<ReceiptItem>) {
  return request.put<ApiResponse<ReceiptItem>>(`/purchase/receipts/${id}/items/${itemId}`, data);
}

export function deleteReceiptItem(id: number, itemId: number) {
  return request.delete<ApiResponse<void>>(`/purchase/receipts/${id}/items/${itemId}`);
}

/**
 * 生成采购入库单号
 * GET /purchase/receipts/generate-no
 *
 * 单据号格式：`RK{yyyyMMdd}{4 位流水}`，例如 `RK202605140001`。
 * 后端通过 DocumentNumberGenerator 统计当日同前缀单据数量 + 1 计算流水。
 */
export const generatePurchaseReceiptNo = (): Promise<ApiResponse<{ receipt_no: string }>> =>
  request.get('/purchase/receipts/generate-no');
