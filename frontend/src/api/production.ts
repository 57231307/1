import { request } from './request'
import type { ApiResponse } from './request'

export interface ProductionOrder {
  id: number
  order_no: string
  order_type: string
  product_id: number
  product_name: string
  quantity: number
  unit?: string
  scheduled_start_date?: string
  scheduled_end_date?: string
  actual_start_date?: string
  actual_end_date?: string
  status: string
  priority: string
  remark?: string
  items?: ProductionOrderItem[]
}

export interface ProductionOrderItem {
  id: number
  order_id: number
  material_id: number
  material_name: string
  required_quantity: number
  available_quantity: number
}

export interface CostCollection {
  id: number
  collection_no: string
  cost_type: string
  period: string
  department_id: number
  department_name: string
  total_cost: number
  status: string
  remark?: string
}

export const productionApi = {
  listOrders: (params?: any) =>
    request.get<ApiResponse<{ list: ProductionOrder[]; total: number }>>('/production/orders', { params }),

  createOrder: (data: Partial<ProductionOrder>) =>
    request.post<ApiResponse<ProductionOrder>>('/production/orders', data),

  getOrder: (id: number) =>
    request.get<ApiResponse<ProductionOrder>>(`/production/orders/${id}`),

  updateOrder: (id: number, data: Partial<ProductionOrder>) =>
    request.put<ApiResponse<ProductionOrder>>(`/production/orders/${id}`, data),

  deleteOrder: (id: number) =>
    request.delete<ApiResponse<null>>(`/production/orders/${id}`),

  updateOrderStatus: (id: number, status: string) =>
    request.put<ApiResponse<null>>(`/production/orders/${id}/status`, { status }),
}

export const costApi = {
  listCollections: (params?: any) =>
    request.get<ApiResponse<{ list: CostCollection[]; total: number }>>('/cost-collections', { params }),

  createCollection: (data: Partial<CostCollection>) =>
    request.post<ApiResponse<CostCollection>>('/cost-collections', data),

  getCollection: (id: number) =>
    request.get<ApiResponse<CostCollection>>(`/cost-collections/${id}`),

  updateCollection: (id: number, data: Partial<CostCollection>) =>
    request.put<ApiResponse<CostCollection>>(`/cost-collections/${id}`, data),

  deleteCollection: (id: number) =>
    request.delete<ApiResponse<null>>(`/cost-collections/${id}`),

  getCostAnalysisSummary: (params?: { period?: string }) =>
    request.get<ApiResponse<any>>('/cost-collections/analysis/summary', { params }),

  getCostByBatch: (params?: { batch_id?: number }) =>
    request.get<ApiResponse<any>>('/cost-collections/analysis/by-batch', { params }),
}
