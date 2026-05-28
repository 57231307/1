import { request } from './request'
import type { ApiResponse } from './request'

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

export const salesApi = {
  getOrderList: (params?: SalesOrderQueryParams) =>
    request.get<ApiResponse<{ list: SalesOrder[]; total: number }>>('/sales/orders', {
      params,
    }),

  getOrderById: (id: number) => request.get<ApiResponse<SalesOrder>>(`/sales/orders/${id}`),

  createOrder: (data: Partial<SalesOrder>) =>
    request.post<ApiResponse<SalesOrder>>('/sales/orders', data),

  updateOrder: (id: number, data: Partial<SalesOrder>) =>
    request.put<ApiResponse<SalesOrder>>(`/sales/orders/${id}`, data),

  deleteOrder: (id: number) => request.delete<ApiResponse<null>>(`/sales/orders/${id}`),

  submitOrder: (id: number) => request.post<ApiResponse<null>>(`/sales/orders/${id}/submit`),

  approveOrder: (id: number) => request.post<ApiResponse<null>>(`/sales/orders/${id}/approve`),

  rejectOrder: (id: number, reason: string) =>
    request.post<ApiResponse<null>>(`/sales/orders/${id}/reject`, { reason }),

  cancelOrder: (id: number) => request.post<ApiResponse<null>>(`/sales/orders/${id}/cancel`),

  createDelivery: (orderId: number, data: any) =>
    request.post<ApiResponse<any>>(`/sales/orders/${orderId}/deliveries`, data),

  getDeliveries: (orderId: number) =>
    request.get<ApiResponse<any[]>>(`/sales/orders/${orderId}/deliveries`),

  getOrderStatistics: (params: any) =>
    request.get<ApiResponse<any>>('/sales/orders/statistics', { params }),

  createReturn: (data: any) => request.post<ApiResponse<any>>('/sales/returns', data),

  getReturns: (params?: any) =>
    request.get<ApiResponse<{ list: any[]; total: number }>>('/sales/returns', { params }),
}
