import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface SalesStats {
  monthOrders: number
  monthAmount: number
  grossProfitRate: number
  activeCustomers: number
  orderTrend: number
  amountTrend: number
  profitTrend: number
  customerTrend: number
}

export interface ProductRanking {
  product_name: string
  amount: number
  quantity: number
  percentage: number
}

export interface CustomerRanking {
  customer_name: string
  amount: number
  order_count: number
  percentage: number
}

export interface SalesTarget {
  period: string
  target_amount: number
  actual_amount: number
  completion_rate: number
  variance: number
  status: string
}

export const salesAnalysisApi = {
  getStats: (params?: any) =>
    request.get<ApiResponse<SalesStats>>('/sales-analysis/stats', { params }),

  getProductRanking: (params?: { type?: string }) =>
    request.get<ApiResponse<ProductRanking[]>>('/sales-analysis/product-ranking', { params }),

  getCustomerRanking: (params?: { type?: string }) =>
    request.get<ApiResponse<CustomerRanking[]>>('/sales-analysis/customer-ranking', { params }),

  getSalesTargets: () => request.get<ApiResponse<SalesTarget[]>>('/sales-analysis/targets'),

  updateSalesTarget: (period: string, data: Partial<SalesTarget>) =>
    request.put<ApiResponse<SalesTarget>>(`/sales-analysis/targets/${period}`, data),

  getTrendData: (params?: { period?: string }) =>
    request.get<ApiResponse<any>>('/sales-analysis/trend', { params }),

  exportReport: (params?: any) =>
    request.get<Blob>('/sales-analysis/export', { params, responseType: 'blob' }),
}
