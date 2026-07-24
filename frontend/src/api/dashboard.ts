import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface DashboardQuery {
  start_date?: string
  end_date?: string
}

export interface DashboardOverview {
  fabricCount?: number
  inventoryTotal?: number
  monthOrders?: number
  customerCount?: number
  todayOrders?: number
  pendingOrders?: number
  lowStockProducts?: number
  monthSales?: number
  recentActivities?: Activity[]
}

export interface Activity {
  id: number
  type: string
  content: string
  time: string
  user: string
}

export interface ChartData {
  label: string
  value: number
}

export interface SalesTrend {
  date: string
  amount: number
  count: number
}

export interface SalesStatistics {
  totalAmount?: number
  orderCount?: number
  customerCount?: number
  avgOrderAmount?: number
  trends?: SalesTrend[]
}

export interface InventoryStatistics {
  totalItems?: number
  totalValue?: number
  warehouseCount?: number
  categoryDistribution?: ChartData[]
}

export interface LowStockAlert {
  id: number
  productId: number
  productName: string
  productCode: string
  warehouseId: number
  warehouseName: string
  currentQuantity: number
  minQuantity: number
  unit?: string
  alertLevel: 'warning' | 'danger'
}

// D14 Batch 5b：原 dashboardApi.getOverview 转为风格 B 函数
export const getDashboardOverview = (params?: DashboardQuery) =>
  request.get<ApiResponse<DashboardOverview>>('/dashboard/overview', { params })

// D14 Batch 5b：原 dashboardApi.getSalesStats 转为风格 B 函数
export const getDashboardSalesStats = (params?: DashboardQuery) =>
  request.get<ApiResponse<SalesStatistics>>('/dashboard/sales-stats', { params })

// D14 Batch 5b：原 dashboardApi.getInventoryStats 转为风格 B 函数
export const getDashboardInventoryStats = (params?: DashboardQuery) =>
  request.get<ApiResponse<InventoryStatistics>>('/dashboard/inventory-stats', { params })

// D14 Batch 5b：原 dashboardApi.getLowStockAlerts 转为风格 B 函数
export const getDashboardLowStockAlerts = () =>
  request.get<ApiResponse<LowStockAlert[]>>('/dashboard/low-stock-alerts')
