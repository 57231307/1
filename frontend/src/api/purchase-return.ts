import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface PurchaseReturn {
  id?: number
  returnNo: string
  purchaseOrderId?: number
  purchaseOrderNo?: string
  supplierId?: number
  supplierName?: string
  returnDate?: string
  reason?: string
  status?: string
  totalAmount?: number
  items?: PurchaseReturnItem[]
  remarks?: string
  createdBy?: number
  approved_by?: number
  createdAt?: string
  updatedAt?: string
}

export interface PurchaseReturnItem {
  id?: number
  returnId?: number
  productId?: number
  productName?: string
  productCode?: string
  quantity: number
  returnedQuantity?: number
  unitPrice?: number
  amount?: number
  reason?: string
  batchNo?: string
}

export interface PurchaseReturnQueryParams {
  page?: number
  pageSize?: number
  purchaseOrderId?: number
  supplierId?: number
  status?: string
  startDate?: string
  endDate?: string
}

export const purchaseReturnApi = {
  list: (params?: PurchaseReturnQueryParams) =>
    request.get<ApiResponse<{ list: PurchaseReturn[]; total: number }>>('/purchase/returns', {
      params,
    }),

  create: (data: Partial<PurchaseReturn>) =>
    request.post<ApiResponse<PurchaseReturn>>('/purchase/returns', data),

  getById: (id: number) => request.get<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}`),

  update: (id: number, data: Partial<PurchaseReturn>) =>
    request.put<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}`, data),

  delete: (id: number) => request.delete<ApiResponse<void>>(`/purchase/returns/${id}`),

  submit: (id: number) =>
    request.post<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}/submit`),

  approve: (id: number) =>
    request.post<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}/approve`),

  reject: (id: number, reason?: string) =>
    request.post<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}/reject`, { reason }),

  listItems: (id: number) =>
    request.get<ApiResponse<{ items: PurchaseReturnItem[] }>>(`/purchase/returns/${id}/items`),

  createItem: (id: number, data: Partial<PurchaseReturnItem>) =>
    request.post<ApiResponse<PurchaseReturnItem>>(`/purchase/returns/${id}/items`, data),

  updateItem: (id: number, itemId: number, data: Partial<PurchaseReturnItem>) =>
    request.put<ApiResponse<PurchaseReturnItem>>(`/purchase/returns/${id}/items/${itemId}`, data),

  deleteItem: (id: number, itemId: number) =>
    request.delete<ApiResponse<void>>(`/purchase/returns/${id}/items/${itemId}`),
}
