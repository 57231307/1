import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

// 生产订单接口
export interface ProductionOrder {
  id: number
  order_no: string
  sales_order_id?: number
  product_id: number
  product_name?: string
  planned_quantity: number
  actual_quantity?: number
  scheduled_start_date?: string
  scheduled_end_date?: string
  actual_start_date?: string
  actual_end_date?: string
  status: 'draft' | 'planned' | 'in_production' | 'completed' | 'cancelled'
  priority: number
  work_center_id?: number
  remark?: string
  created_at?: string
  updated_at?: string
}

// 生产订单状态字典
export const PRODUCTION_ORDER_STATUS = {
  draft: { label: '草稿', type: 'info' },
  planned: { label: '已计划', type: 'warning' },
  in_production: { label: '生产中', type: 'primary' },
  completed: { label: '已完成', type: 'success' },
  cancelled: { label: '已取消', type: 'danger' },
}

// 获取生产订单列表
export function getProductionOrderList(
  params?: QueryParams
): Promise<ApiResponse<{ list: ProductionOrder[]; total: number }>> {
  return request.get('/production/production-orders/orders', { params })
}

// 获取生产订单详情
export function getProductionOrder(id: number): Promise<ApiResponse<ProductionOrder>> {
  return request.get(`/production/production-orders/orders/${id}`)
}

// 创建生产订单
export function createProductionOrder(
  data: Partial<ProductionOrder>
): Promise<ApiResponse<ProductionOrder>> {
  return request.post('/production/production-orders/orders', data)
}

// 更新生产订单
export function updateProductionOrder(
  id: number,
  data: Partial<ProductionOrder>
): Promise<ApiResponse<ProductionOrder>> {
  return request.put(`/production/production-orders/orders/${id}`, data)
}

// 删除生产订单
export function deleteProductionOrder(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/production/production-orders/orders/${id}`)
}

// 更新生产订单状态
export function updateProductionOrderStatus(
  id: number,
  status: string
): Promise<ApiResponse<void>> {
  return request.put(`/production/production-orders/orders/${id}/status`, { status })
}

// 提交生产订单审核（后端: POST /production/production-orders/orders/:id/submit-approval）
export function submitProductionOrder(id: number): Promise<ApiResponse<void>> {
  return request.post(`/production/production-orders/orders/${id}/submit-approval`)
}

// 审核生产订单（后端: POST /production/production-orders/orders/:id/approve）
export function approveProductionOrder(
  id: number,
  data: { approved: boolean; remark?: string }
): Promise<ApiResponse<void>> {
  return request.post(`/production/production-orders/orders/${id}/approve`, data)
}

// 汇报生产进度
export function reportProductionProgress(
  id: number,
  data: {
    completed_quantity: number
    defect_quantity?: number
    remark?: string
  }
): Promise<ApiResponse<void>> {
  return request.post(`/production/production-orders/orders/${id}/progress`, data)
}

// 获取生产订单日志
export function getProductionOrderLogs(
  id: number
): Promise<
  ApiResponse<
    { id: number; action: string; operator: string; created_at: string; remark?: string }[]
  >
> {
  return request.get(`/production/production-orders/orders/${id}/logs`)
}
