import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface PurchaseOrder {
  id: number
  order_no: string
  supplier_id: number
  supplier_name: string
  order_date: string
  required_date?: string
  status: string
  total_amount: number
  received_amount?: number
  tax_amount?: number
  payment_status?: string
  contact_person?: string
  contact_phone?: string
  delivery_address?: string
  remark?: string
  remarks?: string
  creator_name?: string
  created_at?: string
  items: PurchaseOrderItem[]
}

export interface PurchaseOrderItem {
  id: number
  product_id: number
  product_name: string
  product_code: string
  quantity: number
  unit?: string
  unit_price: number
  tax_rate?: number
  tax_amount?: number
  subtotal: number
  received_quantity?: number
}

export interface PurchaseOrderQueryParams {
  page?: number
  page_size?: number
  keyword?: string
  supplier_id?: number
  status?: string
  order_date_from?: string
  order_date_to?: string
}

export interface PurchaseReceipt {
  id: number
  receipt_no: string
  order_id: number
  order_no: string
  supplier_id: number
  supplier_name: string
  receipt_date: string
  warehouse_id?: number
  status: 'draft' | 'pending' | 'completed'
  items: PurchaseReceiptItem[]
  remark?: string
  created_at?: string
}

export interface PurchaseReceiptItem {
  id?: number
  receipt_id?: number
  product_id: number
  product_name?: string
  product_code?: string
  expected_quantity?: number
  received_quantity: number
  unit?: string
  remark?: string
}

export interface PurchaseReceiptQueryParams {
  page?: number
  page_size?: number
  keyword?: string
  supplier_id?: number
  warehouse_id?: number
  status?: string
  receipt_date_from?: string
  receipt_date_to?: string
}

// D14 Batch 5b：原 purchaseApi.getOrderList 转为风格 B 函数
export const getPurchaseOrderList = (params?: PurchaseOrderQueryParams) =>
  request.get<ApiResponse<{ list: PurchaseOrder[]; total: number }>>('/purchase/orders', {
    params,
  })

// D14 Batch 5b：原 purchaseApi.getOrderById 转为风格 B 函数
export const getPurchaseOrderById = (id: number) =>
  request.get<ApiResponse<PurchaseOrder>>(`/purchase/orders/${id}`)

// D14 Batch 5b：原 purchaseApi.createOrder 转为风格 B 函数
export const createPurchaseOrder = (data: Partial<PurchaseOrder>) =>
  request.post<ApiResponse<PurchaseOrder>>('/purchase/orders', data)

// D14 Batch 5b：原 purchaseApi.updateOrder 转为风格 B 函数
export const updatePurchaseOrder = (id: number, data: Partial<PurchaseOrder>) =>
  request.put<ApiResponse<PurchaseOrder>>(`/purchase/orders/${id}`, data)

// D14 Batch 5b：原 purchaseApi.deleteOrder 转为风格 B 函数
export const deletePurchaseOrder = (id: number) =>
  request.delete<ApiResponse<null>>(`/purchase/orders/${id}`)

// D14 Batch 5b：原 purchaseApi.submitOrder 转为风格 B 函数
export const submitPurchaseOrder = (id: number) =>
  request.post<ApiResponse<null>>(`/purchase/orders/${id}/submit`)

// D14 Batch 5b：原 purchaseApi.approveOrder 转为风格 B 函数
export const approvePurchaseOrder = (id: number) =>
  request.post<ApiResponse<null>>(`/purchase/orders/${id}/approve`)

// D14 Batch 5b：原 purchaseApi.rejectOrder 转为风格 B 函数
export const rejectPurchaseOrder = (id: number, reason: string) =>
  request.post<ApiResponse<null>>(`/purchase/orders/${id}/reject`, { reason })

// D14 Batch 5b：原 purchaseApi.getReceipts 转为风格 B 函数
export const getPurchaseReceiptList = (params?: PurchaseReceiptQueryParams) =>
  request.get<ApiResponse<{ list: PurchaseReceipt[]; total: number }>>('/purchase/receipts', {
    params,
  })

// D14 Batch 5b：原 purchaseApi.createReceipt 转为风格 B 函数
export const createPurchaseReceipt = (data: Partial<PurchaseReceipt>) =>
  request.post<ApiResponse<PurchaseReceipt>>('/purchase/receipts', data)

// D14 Batch 5b：原 purchaseApi.receiveItems 转为风格 B 函数
export const receivePurchaseItems = (receiptId: number, data: Partial<PurchaseReceiptItem>[]) =>
  request.post<ApiResponse<PurchaseReceipt>>(`/purchase/receipts/${receiptId}/receive`, data)

/**
 * 生成采购订单号（P1-1 补齐 generate-no 端点）
 * 后端: GET /api/v1/erp/purchase/orders/generate-no
 * 返回: { prefix: "PO", order_no: "PO20260617001" }
 */
// D14 Batch 5b：原 purchaseApi.generateOrderNo 转为风格 B 函数
export const generatePurchaseOrderNo = () =>
  request.get<ApiResponse<{ prefix: string; order_no: string }>>('/purchase/orders/generate-no')
