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

export const purchaseInspectionApi = {
  // 检验单列表
  list: (params?: PurchaseInspectionQueryParams) =>
    request.get<ApiResponse<{ list: PurchaseInspection[]; total: number }>>(
      '/purchases/inspections',
      { params }
    ),

  // 创建检验单
  create: (data: Partial<PurchaseInspection>) =>
    request.post<ApiResponse<PurchaseInspection>>('/purchases/inspections', data),

  // 获取检验单详情
  getById: (id: number) =>
    request.get<ApiResponse<PurchaseInspection>>(`/purchases/inspections/${id}`),

  // 更新检验单
  update: (id: number, data: Partial<PurchaseInspection>) =>
    request.put<ApiResponse<PurchaseInspection>>(`/purchases/inspections/${id}`, data),

  // 完成检验
  complete: (id: number) =>
    request.post<ApiResponse<PurchaseInspection>>(`/purchases/inspections/${id}/complete`),

  // 获取检验明细
  getItems: (id: number) =>
    request.get<ApiResponse<{ items: PurchaseInspectionItem[] }>>(
      `/purchases/inspections/${id}/items`
    ),

  // 创建检验明细
  createItem: (id: number, data: Partial<PurchaseInspectionItem>) =>
    request.post<ApiResponse<PurchaseInspectionItem>>(`/purchases/inspections/${id}/items`, data),

  // 更新检验明细
  updateItem: (id: number, itemId: number, data: Partial<PurchaseInspectionItem>) =>
    request.put<ApiResponse<PurchaseInspectionItem>>(
      `/purchases/inspections/${id}/items/${itemId}`,
      data
    ),

  // 删除检验明细
  deleteItem: (id: number, itemId: number) =>
    request.delete<ApiResponse<void>>(`/purchases/inspections/${id}/items/${itemId}`),
}
