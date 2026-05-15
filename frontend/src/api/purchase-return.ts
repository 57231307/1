import { request } from './request'
import type { ApiResponse } from './request'

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
    request.get<ApiResponse<{ list: PurchaseReturn[]; total: number }>>('/purchases/returns', { params }),

  create: (data: Partial<PurchaseReturn>) =>
    request.post<ApiResponse<PurchaseReturn>>('/purchases/returns', data),

  getById: (id: number) =>
    request.get<ApiResponse<PurchaseReturn>>(`/purchases/returns/${id}`),

  update: (id: number, data: Partial<PurchaseReturn>) =>
    request.put<ApiResponse<PurchaseReturn>>(`/purchases/returns/${id}`, data),

  delete: (id: number) =>
    request.delete<ApiResponse<void>>(`/purchases/returns/${id}`),

  submit: (id: number) =>
    request.post<ApiResponse<PurchaseReturn>>(`/purchases/returns/${id}/submit`),

  approve: (id: number) =>
    request.post<ApiResponse<PurchaseReturn>>(`/purchases/returns/${id}/approve`),

  reject: (id: number, reason?: string) =>
    request.post<ApiResponse<PurchaseReturn>>(`/purchases/returns/${id}/reject`, { reason }),

  listItems: (id: number) =>
    request.get<ApiResponse<{ items: PurchaseReturnItem[] }>>(`/purchases/returns/${id}/items`),

  createItem: (id: number, data: Partial<PurchaseReturnItem>) =>
    request.post<ApiResponse<PurchaseReturnItem>>(`/purchases/returns/${id}/items`, data),

  updateItem: (id: number, itemId: number, data: Partial<PurchaseReturnItem>) =>
    request.put<ApiResponse<PurchaseReturnItem>>(`/purchases/returns/${id}/items/${itemId}`, data),

  deleteItem: (id: number, itemId: number) =>
    request.delete<ApiResponse<void>>(`/purchases/returns/${id}/items/${itemId}`),
}
