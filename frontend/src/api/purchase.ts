import { request } from './request';
import type { ApiResponse } from '@/types/api';

// 列表/详情出参 = 后端 PurchaseOrderDto（services/po/order.rs:19，handler purchase_order_handler.rs:26）。
// DTO 键以 snake_case 原样序列化，order_status 经 #[serde(rename="status")] 改名；
// supplier_name/warehouse_name/department_name 为 LEFT-JOIN 列，DTO 里是 Option ⇒ string | null。
export interface PurchaseOrder {
  id: number;
  order_no: string;
  supplier_id: number;
  supplier_name: string | null;
  order_date: string;
  expected_delivery_date: string | null;
  actual_delivery_date: string | null;
  warehouse_id: number;
  warehouse_name: string | null;
  department_id: number;
  department_name: string | null;
  purchaser_id: number;
  currency: string;
  exchange_rate: number;
  total_amount: number;
  total_amount_foreign: number;
  total_quantity: number;
  total_quantity_alt: number;
  status: string;
  payment_terms: string | null;
  shipping_terms: string | null;
  /** 后端 PurchaseOrderDto.notes（备注） */
  notes: string | null;
  created_by: number;
  created_at: string;
  updated_at: string;
  /**
   * 需要后端 JOIN：PurchaseOrderDto 仅有 created_by 数值，无创建人姓名
   * （backend/src/services/po/order.rs:19）。列已保留，待后端补 created_by -> users 名称。
   */
  creator_name?: string | null;
  /**
   * 需要后端聚合：PurchaseOrderDto 无 received_amount 字段
   * （backend/src/services/po/order.rs:19）。列已保留，待后端补该汇总。
   */
  received_amount?: number | null;
  /**
   * 需要后端字段：PurchaseOrderDto 无 payment_status（backend/src/services/po/order.rs:19）。
   * 列已保留，待后端确定付款状态来源与词表。
   */
  payment_status?: string | null;
  items: PurchaseOrderItem[];
}

// 详情明细 = 后端 get_order 直接序列化的 purchase_order_item::Model
// （handlers/purchase_order_handler.rs:116），键为实体 snake_case；
// 实体只有 product_id，无产品名/编码，需后端 JOIN products 补全（列已保留）。
export interface PurchaseOrderItem {
  id: number;
  order_id: number;
  line_no: number;
  product_id: number;
  /** 需要后端 JOIN：purchase_order_item.product_id -> products.product_name（Model 无名列） */
  product_name?: string | null;
  /** 需要后端 JOIN：purchase_order_item.product_id -> products.product_code（Model 无编码列） */
  product_code?: string | null;
  quantity: number;
  unit_price: number;
  subtotal: number;
  tax_amount: number;
  total_amount: number;
  received_quantity: number;
  /**
   * 后端 purchase_order_item.quantity_tolerance_pct（可空，NULL=用品类/全局默认）。
   * 后端 rust_decimal 经 JSON 序列化为字符串（如 "5.00"），NULL→null；
   * 本接口同时用于创建入参 items（useCreate.submitCreate 提交数值），故取并集 number | string | null。
   * 消费点：详情展示 PurchaseViewDialog 需 Number() 归一。
   */
  quantity_tolerance_pct: number | string | null;
  /** 后端 purchase_order_item.notes（备注） */
  notes?: string | null;
}

export interface PurchaseOrderQueryParams {
  page?: number;
  page_size?: number;
  keyword?: string;
  supplier_id?: number;
  status?: string;
  order_date_from?: string;
  order_date_to?: string;
}

export interface PurchaseReceipt {
  id: number;
  receipt_no: string;
  order_id: number;
  order_no: string;
  supplier_id: number;
  supplier_name: string;
  receipt_date: string;
  warehouse_id?: number;
  status: 'draft' | 'pending' | 'completed';
  items: PurchaseReceiptItem[];
  remark?: string;
  created_at?: string;
}

export interface PurchaseReceiptItem {
  id?: number;
  receipt_id?: number;
  product_id: number;
  product_name?: string;
  product_code?: string;
  expected_quantity?: number;
  received_quantity: number;
  unit?: string;
  remark?: string;
}

export interface PurchaseReceiptQueryParams {
  page?: number;
  page_size?: number;
  keyword?: string;
  supplier_id?: number;
  warehouse_id?: number;
  status?: string;
  receipt_date_from?: string;
  receipt_date_to?: string;
}

// D14 Batch 5b：原 purchaseApi.getOrderList 转为风格 B 函数
export const getPurchaseOrderList = (params?: PurchaseOrderQueryParams) =>
  request.get<ApiResponse<{ items: PurchaseOrder[]; total: number }>>('/purchase/orders', {
    params,
  });

// D14 Batch 5b：原 purchaseApi.getOrderById 转为风格 B 函数
export const getPurchaseOrderById = (id: number) =>
  request.get<ApiResponse<PurchaseOrder>>(`/purchase/orders/${id}`);

// D14 Batch 5b：原 purchaseApi.createOrder 转为风格 B 函数
export const createPurchaseOrder = (data: Partial<PurchaseOrder>) =>
  request.post<ApiResponse<PurchaseOrder>>('/purchase/orders', data);

// D14 Batch 5b：原 purchaseApi.updateOrder 转为风格 B 函数
export const updatePurchaseOrder = (id: number, data: Partial<PurchaseOrder>) =>
  request.put<ApiResponse<PurchaseOrder>>(`/purchase/orders/${id}`, data);

// D14 Batch 5b：原 purchaseApi.deleteOrder 转为风格 B 函数
export const deletePurchaseOrder = (id: number) =>
  request.delete<ApiResponse<null>>(`/purchase/orders/${id}`);

// D14 Batch 5b：原 purchaseApi.submitOrder 转为风格 B 函数
export const submitPurchaseOrder = (id: number) =>
  request.post<ApiResponse<null>>(`/purchase/orders/${id}/submit`);

// D14 Batch 5b：原 purchaseApi.approveOrder 转为风格 B 函数
export const approvePurchaseOrder = (id: number) =>
  request.post<ApiResponse<null>>(`/purchase/orders/${id}/approve`);

// D14 Batch 5b：原 purchaseApi.rejectOrder 转为风格 B 函数
export const rejectPurchaseOrder = (id: number, reason: string) =>
  request.post<ApiResponse<null>>(`/purchase/orders/${id}/reject`, { reason });

// D14 Batch 5b：原 purchaseApi.getReceipts 转为风格 B 函数
export const getPurchaseReceiptList = (params?: PurchaseReceiptQueryParams) =>
  request.get<ApiResponse<{ items: PurchaseReceipt[]; total: number }>>('/purchase/receipts', {
    params,
  });

// D14 Batch 5b：原 purchaseApi.createReceipt 转为风格 B 函数
export const createPurchaseReceipt = (data: Partial<PurchaseReceipt>) =>
  request.post<ApiResponse<PurchaseReceipt>>('/purchase/receipts', data);

// D14 Batch 5b：原 purchaseApi.receiveItems 转为风格 B 函数
export const receivePurchaseItems = (receiptId: number, data: Partial<PurchaseReceiptItem>[]) =>
  request.post<ApiResponse<PurchaseReceipt>>(`/purchase/receipts/${receiptId}/receive`, data);

/**
 * 生成采购订单号（P1-1 补齐 generate-no 端点）
 * 后端: GET /api/v1/erp/purchase/orders/generate-no
 * 返回: { prefix: "PO", order_no: "PO20260617001" }
 */
// D14 Batch 5b：原 purchaseApi.generateOrderNo 转为风格 B 函数
export const generatePurchaseOrderNo = () =>
  request.get<ApiResponse<{ prefix: string; order_no: string }>>('/purchase/orders/generate-no');
