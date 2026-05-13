import { request } from './request'
import type { ApiResponse } from './request'

export interface DashboardStats {
  fabricCount: number
  inventoryTotal: number
  monthOrders: number
  customerCount: number
  todayOrders: number
  pendingOrders: number
  lowStockProducts: number
  monthSales: number
  recentActivities: Activity[]
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

export const dashboardApi = {
  getStats: () => request.get<ApiResponse<DashboardStats>>('/dashboard/stats'),

  getSalesTrend: (days = 7) =>
    request.get<ApiResponse<SalesTrend[]>>('/dashboard/sales-trend', {
      params: { days },
    }),

  getInventoryDistribution: () =>
    request.get<ApiResponse<ChartData[]>>('/dashboard/inventory-distribution'),

  getRecentActivities: () =>
    request.get<ApiResponse<Activity[]>>('/dashboard/activities'),
}
