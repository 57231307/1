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
  status?: string
  receipt_date_from?: string
  receipt_date_to?: string
}

export const purchaseApi = {
  getOrderList: (params?: PurchaseOrderQueryParams) =>
    request.get<ApiResponse<{ list: PurchaseOrder[]; total: number }>>('/purchases/orders', {
      params,
    }),

  getOrderById: (id: number) => request.get<ApiResponse<PurchaseOrder>>(`/purchases/orders/${id}`),

  createOrder: (data: Partial<PurchaseOrder>) =>
    request.post<ApiResponse<PurchaseOrder>>('/purchases/orders', data),

  updateOrder: (id: number, data: Partial<PurchaseOrder>) =>
    request.put<ApiResponse<PurchaseOrder>>(`/purchases/orders/${id}`, data),

  deleteOrder: (id: number) => request.delete<ApiResponse<null>>(`/purchases/orders/${id}`),

  submitOrder: (id: number) => request.post<ApiResponse<null>>(`/purchases/orders/${id}/submit`),

  approveOrder: (id: number) => request.post<ApiResponse<null>>(`/purchases/orders/${id}/approve`),

  rejectOrder: (id: number, reason: string) =>
    request.post<ApiResponse<null>>(`/purchases/orders/${id}/reject`, { reason }),

  getReceipts: (params?: PurchaseReceiptQueryParams) =>
    request.get<ApiResponse<{ list: PurchaseReceipt[]; total: number }>>('/purchases/receipts', {
      params,
    }),

  createReceipt: (data: Partial<PurchaseReceipt>) =>
    request.post<ApiResponse<PurchaseReceipt>>('/purchases/receipts', data),

  receiveItems: (receiptId: number, data: Partial<PurchaseReceiptItem>[]) =>
    request.post<ApiResponse<PurchaseReceipt>>(`/purchases/receipts/${receiptId}/receive`, data),

  /**
   * 生成采购订单号（P1-1 补齐 generate-no 端点）
   * 后端: GET /api/v1/erp/purchases/orders/generate-no
   * 返回: { prefix: "PO", order_no: "PO20260617001" }
   */
  generateOrderNo: () =>
    request.get<ApiResponse<{ prefix: string; order_no: string }>>('/purchases/orders/generate-no'),
}
