import { request } from './request'
import type { ApiResponse } from './request'

export interface ConvertDualUnitRequest {
  productId: number
  quantity: number
  fromUnit: string
  toUnit: string
}

export interface ConvertDualUnitResponse {
  productId: number
  quantity: number
  fromUnit: string
  toUnit: string
  convertedQuantity: number
  conversionFactor: number
}

export interface ValidateDualUnitRequest {
  productId: number
  quantity: number
  unit: string
}

export interface ValidateDualUnitResponse {
  productId: number
  quantity: number
  unit: string
  isValid: boolean
  message?: string
  alternativeQuantity?: number
  alternativeUnit?: string
}

export function convertDualUnit(data: ConvertDualUnitRequest): Promise<ApiResponse<ConvertDualUnitResponse>> {
  return request.post('/dual-unit/convert', data)
}

export function validateDualUnit(data: ValidateDualUnitRequest): Promise<ApiResponse<ValidateDualUnitResponse>> {
  return request.post('/dual-unit/validate', data)
}
