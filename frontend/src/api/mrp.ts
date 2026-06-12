import { request } from './request'
import type { ApiResponse, PageResult } from '@/types/api'

export interface MrpProduct {
  id: number
  product_code: string
  product_name: string
  specification?: string
  unit?: string
}

export interface MrpMaterialRequirement {
  id: number
  material_code: string
  material_name: string
  specification?: string
  unit: string
  required_quantity: number
  available_stock: number
  in_transit_quantity: number
  safety_stock: number
  net_requirement: number
  suggested_order_quantity: number
  suggested_date: string
  warehouse_name?: string
}

export interface MrpCalculationParams {
  product_ids: number[]
  demand_quantity: number
  demand_date: string
  consider_safety_stock: boolean
  consider_in_transit: boolean
}

export interface MrpCalculationResult {
  calculation_id: number
  calculation_no: string
  status: 'pending' | 'calculating' | 'completed' | 'failed'
  products: MrpProduct[]
  demand_quantity: number
  demand_date: string
  materials: MrpMaterialRequirement[]
  created_at: string
  completed_at?: string
}

export interface MrpHistoryRecord {
  id: number
  calculation_no: string
  products: MrpProduct[]
  demand_quantity: number
  demand_date: string
  status: 'pending' | 'calculating' | 'completed' | 'failed'
  created_at: string
  completed_at?: string
}

export interface ConvertToOrderParams {
  calculation_id: number
  material_ids?: number[]
  order_type: 'purchase' | 'production'
}

export function calculateMrp(
  data: MrpCalculationParams
): Promise<ApiResponse<MrpCalculationResult>> {
  return request.post('/production/mrp/calculate', data)
}

export function getMrpHistory(params?: {
  page?: number
  page_size?: number
}): Promise<ApiResponse<PageResult<MrpHistoryRecord>>> {
  return request.get('/production/mrp-history', { params })
}

export function getMrpResult(id: number): Promise<ApiResponse<MrpCalculationResult>> {
  return request.get(`/production/mrp-history/${id}`)
}

export function convertToOrder(
  data: ConvertToOrderParams
): Promise<ApiResponse<{ order_ids: number[] }>> {
  return request.post('/production/mrp/convert-orders', data)
}

export function getProductsForMrp(params?: {
  keyword?: string
}): Promise<ApiResponse<MrpProduct[]>> {
  return request.get('/production/mrp/products', { params })
}

// 取消MRP计算
export function cancelMrpCalculation(id: number): Promise<ApiResponse<void>> {
  return request.put(`/production/mrp-history/${id}/cancel`)
}

// 导出MRP结果
export function exportMrpResult(id: number): Promise<void> {
  return request.get(`/production/mrp-history/${id}/export`, {
    responseType: 'blob',
  }) as Promise<void>
}

// 获取物料需求明细
export function getMaterialRequirementDetail(
  calculationId: number,
  materialId: number
): Promise<ApiResponse<MrpMaterialRequirement & { supply_details: any[] }>> {
  return request.get(`/production/mrp-history/${calculationId}/materials/${materialId}`)
}
