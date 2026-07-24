import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface PurchaseInspection {
  id?: number
  inspection_no: string
  receipt_id?: number
  receipt_no?: string
  supplier_id?: number
  supplier_name?: string
  inspection_date: string
  status: 'draft' | 'pending' | 'completed' | 'rejected'
  inspector_id?: number
  inspector_name?: string
  result?: 'pass' | 'fail' | 'partial'
  remark?: string
  items?: PurchaseInspectionItem[]
  created_at?: string
  updated_at?: string
}

export interface PurchaseInspectionItem {
  id?: number
  inspection_id?: number
  product_id: number
  product_name?: string
  product_code?: string
  expected_quantity?: number
  inspected_quantity: number
  passed_quantity: number
  failed_quantity: number
  defect_reason?: string
  remark?: string
}

export interface PurchaseInspectionQueryParams {
  page?: number
  page_size?: number
  keyword?: string
  supplier_id?: number
  status?: string
  result?: string
  inspection_date_from?: string
  inspection_date_to?: string
}

// D14 Batch 5b：原 purchaseInspectionApi.list 转为风格 B 函数
// 检验单列表
export const getPurchaseInspectionList = (params?: PurchaseInspectionQueryParams) =>
  request.get<ApiResponse<{ list: PurchaseInspection[]; total: number }>>(
    '/purchase/inspections',
    { params }
  )

// D14 Batch 5b：原 purchaseInspectionApi.create 转为风格 B 函数
// 创建检验单
export const createPurchaseInspection = (data: Partial<PurchaseInspection>) =>
  request.post<ApiResponse<PurchaseInspection>>('/purchase/inspections', data)

// D14 Batch 5b：原 purchaseInspectionApi.getById 转为风格 B 函数
// 获取检验单详情
export const getPurchaseInspectionById = (id: number) =>
  request.get<ApiResponse<PurchaseInspection>>(`/purchase/inspections/${id}`)

// D14 Batch 5b：原 purchaseInspectionApi.update 转为风格 B 函数
// 更新检验单
export const updatePurchaseInspection = (id: number, data: Partial<PurchaseInspection>) =>
  request.put<ApiResponse<PurchaseInspection>>(`/purchase/inspections/${id}`, data)

// D14 Batch 5b：原 purchaseInspectionApi.complete 转为风格 B 函数
// 完成检验
export const completePurchaseInspection = (id: number) =>
  request.post<ApiResponse<PurchaseInspection>>(`/purchase/inspections/${id}/complete`)

// D14 Batch 5b：原 purchaseInspectionApi.getItems 转为风格 B 函数
// 获取检验明细
export const getPurchaseInspectionItemList = (id: number) =>
  request.get<ApiResponse<{ items: PurchaseInspectionItem[] }>>(
    `/purchase/inspections/${id}/items`
  )

// D14 Batch 5b：原 purchaseInspectionApi.createItem 转为风格 B 函数
// 创建检验明细
export const createPurchaseInspectionItem = (id: number, data: Partial<PurchaseInspectionItem>) =>
  request.post<ApiResponse<PurchaseInspectionItem>>(`/purchase/inspections/${id}/items`, data)

// D14 Batch 5b：原 purchaseInspectionApi.updateItem 转为风格 B 函数
// 更新检验明细
export const updatePurchaseInspectionItem = (
  id: number,
  itemId: number,
  data: Partial<PurchaseInspectionItem>
) =>
  request.put<ApiResponse<PurchaseInspectionItem>>(
    `/purchase/inspections/${id}/items/${itemId}`,
    data
  )

// D14 Batch 5b：原 purchaseInspectionApi.deleteItem 转为风格 B 函数
// 删除检验明细
export const deletePurchaseInspectionItem = (id: number, itemId: number) =>
  request.delete<ApiResponse<void>>(`/purchase/inspections/${id}/items/${itemId}`)
