import { request } from './request'
import type { ApiResponse } from './request'

export interface AssistDimension {
  id?: number
  dimensionName: string
  dimensionCode: string
  dimensionType: string
  description?: string
  isActive?: boolean
  createdAt?: string
  updatedAt?: string
}

export interface AssistRecord {
  id?: number
  dimensionId?: number
  dimension?: AssistDimension
  businessType?: string
  businessId?: number
  data?: Record<string, any>
  createdAt?: string
}

export interface AssistAccountingQueryParams {
  dimensionId?: number
  businessType?: string
  businessId?: number
  fiveDimensionId?: number
  startDate?: string
  endDate?: string
}

export const assistAccountingApi = {
  listDimensions: () =>
    request.get<ApiResponse<{ dimensions: AssistDimension[] }>>('/assist-accounting/dimensions'),

  queryRecords: (params?: AssistAccountingQueryParams) =>
    request.get<ApiResponse<{ records: AssistRecord[]; total: number }>>('/assist-accounting/records', { params }),

  getByBusiness: (businessType: string, businessId: number) =>
    request.get<ApiResponse<{ records: AssistRecord[] }>>(`/assist-accounting/records/business`, {
      params: { businessType, businessId }
    }),

  getByFiveDimension: (fiveDimensionId: number) =>
    request.get<ApiResponse<{ records: AssistRecord[] }>>(`/assist-accounting/records/five-dimension/${fiveDimensionId}`),
}
