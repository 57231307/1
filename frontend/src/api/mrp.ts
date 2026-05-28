import { request } from './request'
import type { ApiResponse, PageResult } from '../types/api'

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
  return request.post('/mrp/calculate', data)
}

export function getMrpHistory(params?: {
  page?: number
  page_size?: number
}): Promise<ApiResponse<PageResult<MrpHistoryRecord>>> {
  return request.get('/mrp/history', { params })
}

export function getMrpResult(id: number): Promise<ApiResponse<MrpCalculationResult>> {
  return request.get(`/mrp/history/${id}`)
}

export function convertToOrder(
  data: ConvertToOrderParams
): Promise<ApiResponse<{ order_ids: number[] }>> {
  return request.post('/mrp/convert', data)
}

export function getProductsForMrp(params?: {
  keyword?: string
}): Promise<ApiResponse<MrpProduct[]>> {
  return request.get('/mrp/products', { params })
}
