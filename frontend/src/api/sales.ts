import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface SalesOrder {
  id: number
  order_no: string
  customer_id: number
  customer_name: string
  order_date: string
  required_date?: string
  status: string
  total_amount: number
  tax_amount?: number
  discount_amount?: number
  contact_person?: string
  contact_phone?: string
  delivery_address?: string
  remark?: string
  creator_name?: string
  created_at?: string
  updated_at?: string
  items: SalesOrderItem[]
}

export interface SalesOrderItem {
  id: number
  product_id: number
  product_name: string
  product_code: string
  quantity: number
  unit?: string
  unit_price: number
  tax_rate?: number
  tax_amount?: number
  discount_rate?: number
  discount_amount?: number
  subtotal: number
  delivered_quantity?: number
  delivered_amount?: number
}

export interface SalesOrderQueryParams {
  page?: number
  page_size?: number
  keyword?: string
  customer_id?: number
  status?: string
  order_date_from?: string
  order_date_to?: string
}

export interface SalesDelivery {
  id: number
  delivery_no: string
  order_id: number
  order_no: string
  customer_id: number
  customer_name: string
  delivery_date: string
  warehouse_id?: number
  status: 'draft' | 'pending' | 'shipped' | 'delivered'
  items: SalesDeliveryItem[]
  remark?: string
  created_at?: string
}

export interface SalesDeliveryItem {
  id?: number
  delivery_id?: number
  product_id: number
  product_name?: string
  product_code?: string
  quantity: number
  unit?: string
  remark?: string
}

export interface SalesDeliveryQueryParams {
  page?: number
  page_size?: number
  keyword?: string
  order_id?: number
  status?: string
  delivery_date_from?: string
  delivery_date_to?: string
}

export interface SalesStatisticsParams {
  date_from?: string
  date_to?: string
  group_by?: 'day' | 'week' | 'month'
  customer_id?: number
}

export interface SalesStatisticsData {
  total_amount: number
  total_orders: number
  total_customers: number
  trends: { date: string; amount: number; orders: number }[]
}

// D14 Batch 5b：原 salesApi.getOrderList 转为风格 B 函数
export const getSalesOrderList = (params?: SalesOrderQueryParams) =>
  request.get<ApiResponse<{ list: SalesOrder[]; total: number }>>('/sales/orders', {
    params,
  })

// D14 Batch 5b：原 salesApi.getOrderById 转为风格 B 函数
export const getSalesOrderById = (id: number) =>
  request.get<ApiResponse<SalesOrder>>(`/sales/orders/${id}`)

// D14 Batch 5b：原 salesApi.createOrder 转为风格 B 函数
export const createSalesOrder = (data: Partial<SalesOrder>) =>
  request.post<ApiResponse<SalesOrder>>('/sales/orders', data)

// D14 Batch 5b：原 salesApi.updateOrder 转为风格 B 函数
export const updateSalesOrder = (id: number, data: Partial<SalesOrder>) =>
  request.put<ApiResponse<SalesOrder>>(`/sales/orders/${id}`, data)

// D14 Batch 5b：原 salesApi.deleteOrder 转为风格 B 函数
export const deleteSalesOrder = (id: number) =>
  request.delete<ApiResponse<null>>(`/sales/orders/${id}`)

// D14 Batch 5b：原 salesApi.submitOrder 转为风格 B 函数
export const submitSalesOrder = (id: number) =>
  request.post<ApiResponse<null>>(`/sales/orders/${id}/submit`)

// D14 Batch 5b：原 salesApi.approveOrder 转为风格 B 函数
export const approveSalesOrder = (id: number) =>
  request.post<ApiResponse<null>>(`/sales/orders/${id}/approve`)

// D14 Batch 5b：原 salesApi.rejectOrder 转为风格 B 函数
export const rejectSalesOrder = (id: number, reason: string) =>
  request.post<ApiResponse<null>>(`/sales/orders/${id}/reject`, { reason })

// D14 Batch 5b：原 salesApi.cancelOrder 转为风格 B 函数
export const cancelSalesOrder = (id: number) =>
  request.post<ApiResponse<null>>(`/sales/orders/${id}/cancel`)

// D14 Batch 5b：原 salesApi.createDelivery 转为风格 B 函数
export const createSalesDelivery = (orderId: number, data: Partial<SalesDelivery>) =>
  request.post<ApiResponse<SalesDelivery>>(`/sales/orders/${orderId}/deliveries`, data)

// D14 Batch 5b：原 salesApi.getDeliveries 转为风格 B 函数
export const getSalesDeliveryList = (orderId: number) =>
  request.get<ApiResponse<SalesDelivery[]>>(`/sales/orders/${orderId}/deliveries`)

// D14 Batch 5b：原 salesApi.getOrderStatistics 转为风格 B 函数
export const getSalesOrderStatistics = (params: SalesStatisticsParams) =>
  request.get<ApiResponse<SalesStatisticsData>>('/sales/orders/statistics', { params })

/**
 * 生成销售订单号（P1-1 补齐 generate-no 端点）
 * 后端: GET /api/v1/erp/sales/orders/generate-no
 * 返回: { prefix: "SO", order_no: "SO20260617001" }
 */
// D14 Batch 5b：原 salesApi.generateOrderNo 转为风格 B 函数
export const generateSalesOrderNo = () =>
  request.get<ApiResponse<{ prefix: string; order_no: string }>>('/sales/orders/generate-no')
