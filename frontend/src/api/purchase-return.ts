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

// D14 Batch 5b：原 purchaseReturnApi.list 转为风格 B 函数
export const getPurchaseReturnList = (params?: PurchaseReturnQueryParams) =>
  request.get<ApiResponse<{ list: PurchaseReturn[]; total: number }>>('/purchase/returns', {
    params,
  })

// D14 Batch 5b：原 purchaseReturnApi.create 转为风格 B 函数
export const createPurchaseReturn = (data: Partial<PurchaseReturn>) =>
  request.post<ApiResponse<PurchaseReturn>>('/purchase/returns', data)

// D14 Batch 5b：原 purchaseReturnApi.getById 转为风格 B 函数
export const getPurchaseReturnById = (id: number) =>
  request.get<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}`)

// D14 Batch 5b：原 purchaseReturnApi.update 转为风格 B 函数
export const updatePurchaseReturn = (id: number, data: Partial<PurchaseReturn>) =>
  request.put<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}`, data)

// D14 Batch 5b：原 purchaseReturnApi.delete 转为风格 B 函数
export const deletePurchaseReturn = (id: number) =>
  request.delete<ApiResponse<void>>(`/purchase/returns/${id}`)

// D14 Batch 5b：原 purchaseReturnApi.submit 转为风格 B 函数
export const submitPurchaseReturn = (id: number) =>
  request.post<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}/submit`)

// D14 Batch 5b：原 purchaseReturnApi.approve 转为风格 B 函数
export const approvePurchaseReturn = (id: number) =>
  request.post<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}/approve`)

// D14 Batch 5b：原 purchaseReturnApi.reject 转为风格 B 函数
export const rejectPurchaseReturn = (id: number, reason?: string) =>
  request.post<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}/reject`, { reason })

// D14 Batch 5b：原 purchaseReturnApi.listItems 转为风格 B 函数
export const getPurchaseReturnItemList = (id: number) =>
  request.get<ApiResponse<{ items: PurchaseReturnItem[] }>>(`/purchase/returns/${id}/items`)

// D14 Batch 5b：原 purchaseReturnApi.createItem 转为风格 B 函数
export const createPurchaseReturnItem = (id: number, data: Partial<PurchaseReturnItem>) =>
  request.post<ApiResponse<PurchaseReturnItem>>(`/purchase/returns/${id}/items`, data)

// D14 Batch 5b：原 purchaseReturnApi.updateItem 转为风格 B 函数
export const updatePurchaseReturnItem = (
  id: number,
  itemId: number,
  data: Partial<PurchaseReturnItem>
) =>
  request.put<ApiResponse<PurchaseReturnItem>>(`/purchase/returns/${id}/items/${itemId}`, data)

// D14 Batch 5b：原 purchaseReturnApi.deleteItem 转为风格 B 函数
export const deletePurchaseReturnItem = (id: number, itemId: number) =>
  request.delete<ApiResponse<void>>(`/purchase/returns/${id}/items/${itemId}`)
