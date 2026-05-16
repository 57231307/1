import { request } from './request'
import type { ApiResponse } from './request'

export interface SalesReturn {
  id?: number
  returnNo: string
  salesOrderId?: number
  salesOrderNo?: string
  customerId?: number
  customerName?: string
  returnDate?: string
  reason?: string
  status?: string
  totalAmount?: number
  items?: SalesReturnItem[]
  return_items?: SalesReturnItem[]
  remarks?: string
  createdBy?: number
  approved_by?: number
  createdAt?: string
  updatedAt?: string
}

export interface SalesReturnItem {
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

export interface SalesReturnQueryParams {
  page?: number
  pageSize?: number
  salesOrderId?: number
  customerId?: number
  status?: string
  startDate?: string
  endDate?: string
}

export const salesReturnApi = {
  list: (params?: SalesReturnQueryParams) =>
    request.get<ApiResponse<{ list: SalesReturn[]; total: number }>>('/sales-returns', { params }),

  create: (data: Partial<SalesReturn>) =>
    request.post<ApiResponse<SalesReturn>>('/sales-returns', data),

  getById: (id: number) =>
    request.get<ApiResponse<SalesReturn>>(`/sales-returns/${id}`),

  update: (id: number, data: Partial<SalesReturn>) =>
    request.put<ApiResponse<SalesReturn>>(`/sales-returns/${id}`, data),

  delete: (id: number) =>
    request.delete<ApiResponse<void>>(`/sales-returns/${id}`),

  submit: (id: number) =>
    request.post<ApiResponse<SalesReturn>>(`/sales-returns/${id}/submit`),

  approve: (id: number) =>
    request.post<ApiResponse<SalesReturn>>(`/sales-returns/${id}/approve`),

  reject: (id: number, reason?: string) =>
    request.post<ApiResponse<SalesReturn>>(`/sales-returns/${id}/reject`, { reason }),

  execute: (id: number) =>
    request.post<ApiResponse<SalesReturn>>(`/sales-returns/${id}/execute`),

  listItems: (id: number) =>
    request.get<ApiResponse<{ items: SalesReturnItem[] }>>(`/sales-returns/${id}/items`),

  createItem: (id: number, data: Partial<SalesReturnItem>) =>
    request.post<ApiResponse<SalesReturnItem>>(`/sales-returns/${id}/items`, data),

  updateItem: (id: number, itemId: number, data: Partial<SalesReturnItem>) =>
    request.put<ApiResponse<SalesReturnItem>>(`/sales-returns/${id}/items/${itemId}`, data),

  deleteItem: (id: number, itemId: number) =>
    request.delete<ApiResponse<void>>(`/sales-returns/${id}/items/${itemId}`),
}
