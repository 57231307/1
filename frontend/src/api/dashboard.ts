import { request } from './request'
import type { ApiResponse } from './request'

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

export const dashboardApi = {
  getOverview: (params?: DashboardQuery) =>
    request.get<ApiResponse<DashboardOverview>>('/dashboard/overview', { params }),

  getSalesStats: (params?: DashboardQuery) =>
    request.get<ApiResponse<SalesStatistics>>('/dashboard/sales-stats', { params }),

  getInventoryStats: (params?: DashboardQuery) =>
    request.get<ApiResponse<InventoryStatistics>>('/dashboard/inventory-stats', { params }),

  getLowStockAlerts: () =>
    request.get<ApiResponse<LowStockAlert[]>>('/dashboard/low-stock-alerts'),
}
