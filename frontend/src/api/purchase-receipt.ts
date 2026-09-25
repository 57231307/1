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

/**
 * 创建入库明细请求 —— 与后端 DTO 逐字段对齐
 * backend/src/services/purchase_receipt_dto.rs:54 CreateReceiptItemRequest
 * 键名 snake_case；非 Option 必填：line_no/material_id/material_code/material_name/
 *   quantity/quantity_alt/unit_master；batch_no 后端 DTO 为 Option 但
 *   create_receipt→validate_receipt_item_dimensions(crud.rs:112) 建单期强校验非空，
 *   故此处收紧为必填 string（不给 undefined 兜底掩盖缺键）。
 * color_code/lot_no/piece_no/grade 为染色布追溯维度（validate_fabric_trace 口径），
 *   后端建单期「染色布必填」分支尚未落地（crud.rs:107-111 TODO），故按可选传递。
 */
export interface CreateReceiptItemRequest {
  order_item_id?: number;
  line_no: number;
  /** 物料（产品）id；validate_receipt_item_dimensions 要求 >0 */
  material_id: number;
  material_code: string;
  material_name: string;
  batch_no: string;
  color_code?: string;
  lot_no?: string;
  /** 染色匹号（匹号领域：入库使用染色匹号） */
  piece_no?: string;
  grade?: string;
  gram_weight?: number;
  width?: number;
  quantity: number;
  quantity_alt: number;
  unit_master: string;
  unit_alt?: string;
  unit_price?: number;
  location_code?: string;
  package_no?: string;
  production_date?: string;
  shelf_life?: number;
  notes?: string;
}

/**
 * 创建采购入库单请求 —— 与后端 DTO 逐字段对齐
 * backend/src/services/purchase_receipt_dto.rs:11 CreatePurchaseReceiptRequest
 * 非 Option 必填：supplier_id/receipt_date/warehouse_id/items；
 * order_id 为 Option：按单收货传采购订单 id，后端据此把入库明细挂到订单行累加收货进度。
 * 注：PurchaseReceiptEntity 是响应/编辑回显模型（含 id/单号/状态等生成列，明细键名为 product_id），
 *   不能作创建入参类型——缺 order_id 且明细键名（material_id/material_code/...）与契约不符。
 */
export interface CreatePurchaseReceiptRequest {
  order_id?: number;
  supplier_id: number;
  receipt_date: string;
  warehouse_id: number;
  department_id?: number;
  inspector_id?: number;
  notes?: string;
  attachment_urls?: string[];
  items: CreateReceiptItemRequest[];
}

export function getPurchaseReceiptList(params?: PurchaseReceiptQueryParams) {
  return request.get<ApiResponse<{ items: PurchaseReceiptEntity[]; total: number }>>(
    '/purchase/receipts',
    { params }
  );
}

export function getPurchaseReceipt(id: number) {
  return request.get<ApiResponse<PurchaseReceiptEntity>>(`/purchase/receipts/${id}`);
}

export function createPurchaseReceipt(data: CreatePurchaseReceiptRequest) {
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
