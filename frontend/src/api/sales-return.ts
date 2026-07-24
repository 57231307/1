import { request } from './request'
import type { ApiResponse } from '@/types/api'

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

// D14 Batch 5b：原 salesReturnApi.list 转为风格 B 函数
export const getSalesReturnList = (params?: SalesReturnQueryParams) =>
  request.get<ApiResponse<{ list: SalesReturn[]; total: number }>>('/sales-returns', { params })

// D14 Batch 5b：原 salesReturnApi.create 转为风格 B 函数
export const createSalesReturn = (data: Partial<SalesReturn>) =>
  request.post<ApiResponse<SalesReturn>>('/sales-returns', data)

// D14 Batch 5b：原 salesReturnApi.getById 转为风格 B 函数
export const getSalesReturnById = (id: number) =>
  request.get<ApiResponse<SalesReturn>>(`/sales-returns/${id}`)

// D14 Batch 5b：原 salesReturnApi.update 转为风格 B 函数
export const updateSalesReturn = (id: number, data: Partial<SalesReturn>) =>
  request.put<ApiResponse<SalesReturn>>(`/sales-returns/${id}`, data)

// D14 Batch 5b：原 salesReturnApi.delete 转为风格 B 函数
export const deleteSalesReturn = (id: number) =>
  request.delete<ApiResponse<void>>(`/sales-returns/${id}`)

// D14 Batch 5b：原 salesReturnApi.submit 转为风格 B 函数
export const submitSalesReturn = (id: number) =>
  request.post<ApiResponse<SalesReturn>>(`/sales-returns/${id}/submit`)

// D14 Batch 5b：原 salesReturnApi.approve 转为风格 B 函数
export const approveSalesReturn = (id: number) =>
  request.post<ApiResponse<SalesReturn>>(`/sales-returns/${id}/approve`)

// D14 Batch 5b：原 salesReturnApi.reject 转为风格 B 函数
export const rejectSalesReturn = (id: number, reason?: string) =>
  request.post<ApiResponse<SalesReturn>>(`/sales-returns/${id}/reject`, { reason })

// D14 Batch 5b：原 salesReturnApi.execute 转为风格 B 函数
export const executeSalesReturn = (id: number) =>
  request.post<ApiResponse<SalesReturn>>(`/sales-returns/${id}/execute`)

// D14 Batch 5b：原 salesReturnApi.listItems 转为风格 B 函数
export const getSalesReturnItemList = (id: number) =>
  request.get<ApiResponse<{ items: SalesReturnItem[] }>>(`/sales-returns/${id}/items`)

// D14 Batch 5b：原 salesReturnApi.createItem 转为风格 B 函数
export const createSalesReturnItem = (id: number, data: Partial<SalesReturnItem>) =>
  request.post<ApiResponse<SalesReturnItem>>(`/sales-returns/${id}/items`, data)

// D14 Batch 5b：原 salesReturnApi.updateItem 转为风格 B 函数
export const updateSalesReturnItem = (id: number, itemId: number, data: Partial<SalesReturnItem>) =>
  request.put<ApiResponse<SalesReturnItem>>(`/sales-returns/${id}/items/${itemId}`, data)

// D14 Batch 5b：原 salesReturnApi.deleteItem 转为风格 B 函数
export const deleteSalesReturnItem = (id: number, itemId: number) =>
  request.delete<ApiResponse<void>>(`/sales-returns/${id}/items/${itemId}`)
