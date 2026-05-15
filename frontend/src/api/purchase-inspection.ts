import { request } from './request'
import type { ApiResponse } from './request'

export interface PurchaseInspection {
  id?: number
  inspectionNo: string
  purchaseOrderId?: number
  purchaseOrderNo?: string
  receiptId?: number
  receiptNo?: string
  supplierId?: number
  supplierName?: string
  inspectionDate?: string
  inspectorId?: number
  inspectorName?: string
  status?: string
  result?: 'PASS' | 'FAIL' | 'CONDITIONAL_PASS'
  passQuantity?: number
  failQuantity?: number
  defectRate?: number
  defectDescription?: string
  remarks?: string
  items?: PurchaseInspectionItem[]
  createdAt?: string
  updatedAt?: string
}

export interface PurchaseInspectionItem {
  id?: number
  inspectionId?: number
  productId?: number
  productName?: string
  productCode?: string
  quantity: number
  passQuantity?: number
  failQuantity?: number
  defectQuantity?: number
  unit?: string
  defectType?: string
}

export interface PurchaseInspectionQueryParams {
  page?: number
  pageSize?: number
  purchaseOrderId?: number
  supplierId?: number
  status?: string
  startDate?: string
  endDate?: string
}

export const purchaseInspectionApi = {
  list: (params?: PurchaseInspectionQueryParams) =>
    request.get<ApiResponse<{ list: PurchaseInspection[]; total: number }>>('/purchases/inspections', { params }),

  create: (data: Partial<PurchaseInspection>) =>
    request.post<ApiResponse<PurchaseInspection>>('/purchases/inspections', data),

  getById: (id: number) =>
    request.get<ApiResponse<PurchaseInspection>>(`/purchases/inspections/${id}`),

  update: (id: number, data: Partial<PurchaseInspection>) =>
    request.put<ApiResponse<PurchaseInspection>>(`/purchases/inspections/${id}`, data),

  delete: (id: number) =>
    request.delete<ApiResponse<void>>(`/purchases/inspections/${id}`),
}
