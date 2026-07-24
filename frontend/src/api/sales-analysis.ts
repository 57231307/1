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

// P2-9c 修复（批次 82 v1 复审）：销售统计列表查询参数强类型化
export interface SalesStatsQueryParams {
  period?: string
  start_date?: string
  end_date?: string
  category_id?: number
}

// P2-9c 修复（批次 82 v1 复审）：销售报表导出查询参数强类型化
export interface SalesExportQueryParams {
  period?: string
  start_date?: string
  end_date?: string
  category_id?: number
  format?: string
}

// P2-16 修复（批次 86 v2 复审）：销售趋势 ApiResponse<any> → SalesTrendResult
export interface SalesTrendResult {
  period: string
  amount: number
  order_count: number
  profit: number
  growth_rate: number
  [key: string]: unknown
}

// D14 Batch 5b：原 salesAnalysisApi.getStats 转为风格 B 函数
export const getSalesAnalysisStats = (params?: SalesStatsQueryParams) =>
  request.get<ApiResponse<SalesStats>>('/sales-analysis/stats', { params })

// D14 Batch 5b：原 salesAnalysisApi.getProductRanking 转为风格 B 函数
export const getProductRanking = (params?: { type?: string }) =>
  request.get<ApiResponse<ProductRanking[]>>('/sales-analysis/product-ranking', { params })

// D14 Batch 5b：原 salesAnalysisApi.getCustomerRanking 转为风格 B 函数
export const getCustomerRanking = (params?: { type?: string }) =>
  request.get<ApiResponse<CustomerRanking[]>>('/sales-analysis/customer-ranking', { params })

// D14 Batch 5b：原 salesAnalysisApi.getSalesTargets 转为风格 B 函数
export const getSalesTargetList = () =>
  request.get<ApiResponse<SalesTarget[]>>('/sales-analysis/targets')

// D14 Batch 5b：原 salesAnalysisApi.updateSalesTarget 转为风格 B 函数
export const updateSalesTarget = (period: string, data: Partial<SalesTarget>) =>
  request.put<ApiResponse<SalesTarget>>(`/sales-analysis/targets/${period}`, data)

// D14 Batch 5b：原 salesAnalysisApi.getTrendData 转为风格 B 函数
export const getSalesTrendData = (params?: { period?: string }) =>
  request.get<ApiResponse<SalesTrendResult[]>>('/sales-analysis/trend', { params })

// D14 Batch 5b：原 salesAnalysisApi.exportReport 转为风格 B 函数
export const exportSalesAnalysisReport = (params?: SalesExportQueryParams) =>
  request.get<Blob>('/sales-analysis/export', { params, responseType: 'blob' })
