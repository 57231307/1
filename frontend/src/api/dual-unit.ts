import { request } from './request'
import type { ApiResponse } from './request'

export interface DualUnitConversion {
  id?: number
  productId?: number
  productName?: string
  baseUnit?: string
  dualUnit?: string
  conversionRate?: number
  conversionFormula?: string
  precision?: number
  isActive?: boolean
  createdAt?: string
  updatedAt?: string
}

export const dualUnitApi = {
  getByProduct: (productId: number) =>
    request.get<ApiResponse<DualUnitConversion>>(`/dual-unit/product/${productId}`),

  convert: (productId: number, quantity: number, fromUnit: string, toUnit: string) =>
    request.get<ApiResponse<{ convertedQuantity: number }>>('/dual-unit/convert', {
      params: { productId, quantity, fromUnit, toUnit }
    }),
}
