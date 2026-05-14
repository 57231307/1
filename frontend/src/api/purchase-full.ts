import { request } from './request'
import type { ApiResponse } from './request'

export interface PurchaseContract {
  id: number
  contract_no: string
  supplier_id: number
  supplier_name: string
  contract_date: string
  total_amount: number
  signed_amount?: number
  status: string
  start_date?: string
  end_date?: string
  payment_terms?: string
  delivery_terms?: string
  remark?: string
  creator_name?: string
  created_at?: string
}

export interface PurchaseReceipt {
  id: number
  receipt_no: string
  order_id: number
  order_no: string
  supplier_id: number
  supplier_name: string
  receipt_date: string
  total_amount: number
  received_amount: number
  status: string
  warehouse_id: number
  warehouse_name: string
  inspector?: string
  remark?: string
}

export interface PurchaseInspection {
  id: number
  inspection_no: string
  receipt_id: number
  receipt_no: string
  inspection_date: string
  inspector: string
  result: string
  quantity_checked: number
  qualified_quantity: number
  unqualified_quantity: number
  defect_rate?: number
  remark?: string
}

export interface PurchaseReturn {
  id: number
  return_no: string
  order_id: number
  order_no: string
  supplier_id: number
  supplier_name: string
  return_date: string
  total_amount: number
  reason: string
  status: string
  remark?: string
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

export const purchaseApi = {
  getOrderList: (params?: PurchaseOrderQueryParams) =>
    request.get<ApiResponse<{ list: any[]; total: number }>>('/purchases/orders', { params }),

  getOrderById: (id: number) =>
    request.get<ApiResponse<any>>(`/purchases/orders/${id}`),

  createOrder: (data: any) =>
    request.post<ApiResponse<any>>('/purchases/orders', data),

  updateOrder: (id: number, data: any) =>
    request.put<ApiResponse<any>>(`/purchases/orders/${id}`, data),

  deleteOrder: (id: number) =>
    request.delete<ApiResponse<null>>(`/purchases/orders/${id}`),

  submitOrder: (id: number) =>
    request.post<ApiResponse<null>>(`/purchases/orders/${id}/submit`),

  approveOrder: (id: number) =>
    request.post<ApiResponse<null>>(`/purchases/orders/${id}/approve`),

  rejectOrder: (id: number, reason: string) =>
    request.post<ApiResponse<null>>(`/purchases/orders/${id}/reject`, { reason }),

  closeOrder: (id: number) =>
    request.post<ApiResponse<null>>(`/purchases/orders/${id}/close`),

  calculateDeliveryDate: (data: any) =>
    request.post<ApiResponse<{ delivery_date: string }>>('/purchases/orders/delivery-date', data),

  exportOrders: (params?: PurchaseOrderQueryParams) =>
    request.get<Blob>('/purchases/orders/export', { params, responseType: 'blob' }),

  listContracts: (params?: any) =>
    request.get<ApiResponse<{ list: PurchaseContract[]; total: number }>>('/purchase-contracts', { params }),

  createContract: (data: Partial<PurchaseContract>) =>
    request.post<ApiResponse<PurchaseContract>>('/purchase-contracts', data),

  getContract: (id: number) =>
    request.get<ApiResponse<PurchaseContract>>(`/purchase-contracts/${id}`),

  updateContract: (id: number, data: Partial<PurchaseContract>) =>
    request.put<ApiResponse<PurchaseContract>>(`/purchase-contracts/${id}`, data),

  deleteContract: (id: number) =>
    request.delete<ApiResponse<null>>(`/purchase-contracts/${id}`),

  approveContract: (id: number) =>
    request.post<ApiResponse<null>>(`/purchase-contracts/${id}/approve`),

  executeContract: (id: number) =>
    request.put<ApiResponse<null>>(`/purchase-contracts/${id}/execute`),

  cancelContract: (id: number) =>
    request.put<ApiResponse<null>>(`/purchase-contracts/${id}/cancel`),

  listReceipts: (params?: any) =>
    request.get<ApiResponse<{ list: PurchaseReceipt[]; total: number }>>('/purchases/receipts', { params }),

  createReceipt: (data: Partial<PurchaseReceipt>) =>
    request.post<ApiResponse<PurchaseReceipt>>('/purchases/receipts', data),

  getReceipt: (id: number) =>
    request.get<ApiResponse<PurchaseReceipt>>(`/purchases/receipts/${id}`),

  updateReceipt: (id: number, data: Partial<PurchaseReceipt>) =>
    request.put<ApiResponse<PurchaseReceipt>>(`/purchases/receipts/${id}`, data),

  listInspections: (params?: any) =>
    request.get<ApiResponse<{ list: PurchaseInspection[]; total: number }>>('/purchases/inspections', { params }),

  createInspection: (data: Partial<PurchaseInspection>) =>
    request.post<ApiResponse<PurchaseInspection>>('/purchases/inspections', data),

  getInspection: (id: number) =>
    request.get<ApiResponse<PurchaseInspection>>(`/purchases/inspections/${id}`),

  listPurchaseReturns: (params?: any) =>
    request.get<ApiResponse<{ list: PurchaseReturn[]; total: number }>>('/purchases/returns', { params }),

  createPurchaseReturn: (data: Partial<PurchaseReturn>) =>
    request.post<ApiResponse<PurchaseReturn>>('/purchases/returns', data),

  submitPurchaseReturn: (id: number) =>
    request.post<ApiResponse<null>>(`/purchases/returns/${id}/submit`),

  approvePurchaseReturn: (id: number) =>
    request.post<ApiResponse<null>>(`/purchases/returns/${id}/approve`),

  rejectPurchaseReturn: (id: number, reason: string) =>
    request.post<ApiResponse<null>>(`/purchases/returns/${id}/reject`, { reason }),
}
